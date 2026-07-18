use std::{
    collections::HashSet,
    fs::{self, File, Metadata},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use super::{
    EMULATOR_ROOT, EntryKind, IMAGE_METADATA_PATH, ImageError, ImageManifest, ImageNotice,
    InventoryEntry, MAX_ARCHIVE_ENTRIES, MAX_IMAGE_METADATA_BYTES, MAX_UNCOMPRESSED_BYTES,
    NOTICES_ROOT, digest_string, io_error, path_error,
};
use crate::{
    path_policy::{is_reparse_point, join_portable, to_portable_path, windows_case_key},
    recipe::Recipe,
};

#[derive(Clone, Debug)]
pub struct BuildRequest<'a> {
    pub recipe: &'a Recipe,
    pub emulator_root: &'a Path,
    pub rom: &'a Path,
    pub output: &'a Path,
    pub force: bool,
}

#[derive(Clone, Debug)]
pub struct BuildResult {
    pub output: PathBuf,
    pub image_sha256: String,
    pub image_size: u64,
    pub manifest: ImageManifest,
}

#[derive(Clone, Debug)]
struct BuildEntry {
    archive_path: String,
    kind: EntryKind,
    source: Option<PathBuf>,
    size: u64,
    sha256: Option<String>,
}

pub fn build_image(request: &BuildRequest<'_>) -> Result<BuildResult, ImageError> {
    request.recipe.validate()?;
    validate_output_name(request.output)?;
    if request.output.exists() && !request.force {
        return Err(ImageError::OutputExists(request.output.to_path_buf()));
    }
    if request.output.exists() {
        let metadata = fs::symlink_metadata(request.output)
            .map_err(|source| io_error("inspect", request.output, source))?;
        if is_reparse_point(&metadata) {
            return Err(ImageError::Invalid(format!(
                "output {} is a link or reparse point",
                request.output.display()
            )));
        }
    }
    if request.output.is_dir() {
        return Err(ImageError::Invalid(format!(
            "output {} is a directory",
            request.output.display()
        )));
    }

    let emulator_root = canonical_plain_directory("emulator root", request.emulator_root)?;
    let rom = canonical_plain_file("ROM", request.rom)?;
    let output = canonical_output_path(request.output)?;
    if host_path_is_same_or_ancestor(&emulator_root, &rom) {
        return Err(ImageError::Invalid(
            "the ROM must not be located inside the emulator root".to_owned(),
        ));
    }
    if host_path_is_same_or_ancestor(&emulator_root, &output) {
        return Err(ImageError::Invalid(
            "the output image must not be located inside the emulator root".to_owned(),
        ));
    }
    if host_paths_equal(&rom, &output) {
        return Err(ImageError::Invalid(
            "the output image cannot replace the ROM input".to_owned(),
        ));
    }

    validate_recipe_files(request.recipe, &emulator_root)?;

    let (rom_size, rom_sha256) = sha256_file(&rom)?;
    if rom_sha256 != request.recipe.content.sha256 {
        return Err(ImageError::DigestMismatch {
            subject: format!("ROM {}", rom.display()),
            expected: request.recipe.content.sha256.clone(),
            actual: rom_sha256,
        });
    }

    let mut entries = Vec::new();
    let mut source_paths = HashSet::new();
    walk_emulator(
        &emulator_root,
        &emulator_root,
        &mut source_paths,
        &mut entries,
    )?;
    entries.push(BuildEntry {
        archive_path: format!(
            "{}/{}",
            super::CONTENT_ROOT,
            request.recipe.content.filename
        ),
        kind: EntryKind::File,
        source: Some(rom),
        size: rom_size,
        sha256: Some(request.recipe.content.sha256.clone()),
    });

    let mut image_notices = Vec::new();
    for notice in &request.recipe.notices {
        let source = join_portable(&emulator_root, &notice.source)
            .map_err(|error| path_error("invalid notice source", error))?;
        let source = canonical_plain_file("notice source", &source)?;
        if !host_path_is_same_or_ancestor(&emulator_root, &source) {
            return Err(ImageError::Invalid(format!(
                "notice source {} escapes the emulator root",
                source.display()
            )));
        }
        let archive_path = format!("{NOTICES_ROOT}/{}", notice.output);
        let (size, sha256) = sha256_file(&source)?;
        entries.push(BuildEntry {
            archive_path: archive_path.clone(),
            kind: EntryKind::File,
            source: Some(source),
            size,
            sha256: Some(sha256),
        });
        image_notices.push(ImageNotice { path: archive_path });
    }

    entries.sort_by(|left, right| left.archive_path.cmp(&right.archive_path));
    image_notices.sort_by(|left, right| left.path.cmp(&right.path));
    validate_build_entries(&entries)?;

    let inventory = entries
        .iter()
        .map(|entry| InventoryEntry {
            path: entry.archive_path.clone(),
            kind: entry.kind,
            size: entry.size,
            sha256: entry.sha256.clone(),
        })
        .collect();
    let manifest = ImageManifest::from_recipe(request.recipe, rom_size, image_notices, inventory);
    manifest.validate()?;

    let mut metadata = serde_json::to_vec(&manifest)?;
    metadata.push(b'\n');
    if metadata.len() as u64 > MAX_IMAGE_METADATA_BYTES {
        return Err(ImageError::Invalid(format!(
            "image metadata exceeds the {MAX_IMAGE_METADATA_BYTES}-byte limit"
        )));
    }

    let output_parent = usable_parent(request.output);
    let mut temporary = NamedTempFile::new_in(output_parent)
        .map_err(|source| io_error("create temporary image in", output_parent, source))?;
    write_archive(temporary.as_file_mut(), &metadata, &entries)?;
    temporary
        .as_file_mut()
        .sync_all()
        .map_err(|source| io_error("flush temporary image", temporary.path(), source))?;
    let temporary_path = temporary.path().to_path_buf();
    let (image_size, image_sha256) = sha256_open_file(temporary.as_file_mut(), &temporary_path)?;

    let persist_result = if request.force {
        temporary.persist(request.output)
    } else {
        temporary.persist_noclobber(request.output)
    };
    persist_result.map_err(|error| io_error("publish image to", request.output, error.error))?;

    Ok(BuildResult {
        output: request.output.to_path_buf(),
        image_sha256,
        image_size,
        manifest,
    })
}

pub fn sha256_file(path: &Path) -> Result<(u64, String), ImageError> {
    let mut file = File::open(path).map_err(|source| io_error("open", path, source))?;
    sha256_reader(&mut file, path)
}

fn sha256_open_file(file: &mut File, path: &Path) -> Result<(u64, String), ImageError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|source| io_error("seek", path, source))?;
    sha256_reader(file, path)
}

fn sha256_reader(reader: &mut impl Read, path: &Path) -> Result<(u64, String), ImageError> {
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|source| io_error("read", path, source))?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .ok_or_else(|| ImageError::Invalid(format!("file {} is too large", path.display())))?;
        hasher.update(&buffer[..read]);
    }
    Ok((size, digest_string(hasher.finalize().as_slice())))
}

fn walk_emulator(
    root: &Path,
    directory: &Path,
    seen_paths: &mut HashSet<String>,
    entries: &mut Vec<BuildEntry>,
) -> Result<bool, ImageError> {
    let read_directory =
        fs::read_dir(directory).map_err(|source| io_error("read directory", directory, source))?;
    let mut children = Vec::new();
    for child in read_directory {
        let child = child.map_err(|source| io_error("read directory", directory, source))?;
        let path = child.path();
        let portable = to_portable_path(root, &path)
            .map_err(|error| path_error("invalid emulator input path", error))?;
        let metadata =
            fs::symlink_metadata(&path).map_err(|source| io_error("inspect", &path, source))?;
        children.push((portable, path, metadata));
    }
    children.sort_by(|left, right| left.0.cmp(&right.0));

    for (portable, path, metadata) in &children {
        if is_reparse_point(metadata) {
            return Err(ImageError::Invalid(format!(
                "emulator input {} is a link or reparse point",
                path.display()
            )));
        }
        if !seen_paths.insert(windows_case_key(portable)) {
            return Err(ImageError::Invalid(format!(
                "emulator input path {portable:?} collides on Windows"
            )));
        }

        if metadata.is_dir() {
            let has_children = walk_emulator(root, path, seen_paths, entries)?;
            if !has_children {
                entries.push(BuildEntry {
                    archive_path: format!("{EMULATOR_ROOT}/{portable}/"),
                    kind: EntryKind::Directory,
                    source: None,
                    size: 0,
                    sha256: None,
                });
            }
        } else if metadata.is_file() {
            let (size, sha256) = sha256_file(path)?;
            entries.push(BuildEntry {
                archive_path: format!("{EMULATOR_ROOT}/{portable}"),
                kind: EntryKind::File,
                source: Some(path.clone()),
                size,
                sha256: Some(sha256),
            });
        } else {
            return Err(ImageError::Invalid(format!(
                "emulator input {} is not a regular file or directory",
                path.display()
            )));
        }
    }

    Ok(!children.is_empty())
}

fn validate_recipe_files(recipe: &Recipe, emulator_root: &Path) -> Result<(), ImageError> {
    let entrypoint = join_portable(emulator_root, &recipe.emulator.entrypoint)
        .map_err(|error| path_error("invalid emulator entrypoint", error))?;
    canonical_plain_file("emulator entrypoint", &entrypoint)?;

    if recipe.emulator.working_directory != "." {
        let working_directory = join_portable(emulator_root, &recipe.emulator.working_directory)
            .map_err(|error| path_error("invalid emulator working directory", error))?;
        canonical_plain_directory("emulator working directory", &working_directory)?;
    }

    for mount in &recipe.state.mounts {
        let target = join_portable(emulator_root, &mount.target)
            .map_err(|error| path_error("invalid state mount target", error))?;
        canonical_plain_directory("state mount target", &target)?;
    }

    Ok(())
}

fn canonical_plain_file(label: &str, path: &Path) -> Result<PathBuf, ImageError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| io_error("inspect", path, source))?;
    reject_reparse(label, path, &metadata)?;
    if !metadata.is_file() {
        return Err(ImageError::Invalid(format!(
            "{label} {} is not a regular file",
            path.display()
        )));
    }
    fs::canonicalize(path).map_err(|source| io_error("canonicalize", path, source))
}

fn canonical_plain_directory(label: &str, path: &Path) -> Result<PathBuf, ImageError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| io_error("inspect", path, source))?;
    reject_reparse(label, path, &metadata)?;
    if !metadata.is_dir() {
        return Err(ImageError::Invalid(format!(
            "{label} {} is not a directory",
            path.display()
        )));
    }
    fs::canonicalize(path).map_err(|source| io_error("canonicalize", path, source))
}

fn reject_reparse(label: &str, path: &Path, metadata: &Metadata) -> Result<(), ImageError> {
    if is_reparse_point(metadata) {
        Err(ImageError::Invalid(format!(
            "{label} {} is a link or reparse point",
            path.display()
        )))
    } else {
        Ok(())
    }
}

fn canonical_output_path(path: &Path) -> Result<PathBuf, ImageError> {
    if path.exists() {
        return fs::canonicalize(path).map_err(|source| io_error("canonicalize", path, source));
    }
    let parent = usable_parent(path);
    let parent = fs::canonicalize(parent)
        .map_err(|source| io_error("canonicalize output parent", parent, source))?;
    let filename = path.file_name().ok_or_else(|| {
        ImageError::Invalid(format!("output {} has no file name", path.display()))
    })?;
    Ok(parent.join(filename))
}

fn validate_output_name(path: &Path) -> Result<(), ImageError> {
    let extension = path.extension().and_then(|value| value.to_str());
    if !extension.is_some_and(|value| value.eq_ignore_ascii_case("emuc")) {
        return Err(ImageError::Invalid(
            "output file name must use the .emuc extension".to_owned(),
        ));
    }
    Ok(())
}

fn validate_build_entries(entries: &[BuildEntry]) -> Result<(), ImageError> {
    if entries.len() + 1 > MAX_ARCHIVE_ENTRIES {
        return Err(ImageError::Invalid(format!(
            "image exceeds the {MAX_ARCHIVE_ENTRIES}-entry limit"
        )));
    }

    let mut previous: Option<&str> = None;
    let mut case_paths = HashSet::new();
    let mut total_size = 0_u64;
    for entry in entries {
        if previous.is_some_and(|path| path >= entry.archive_path.as_str()) {
            return Err(ImageError::Invalid(format!(
                "duplicate image path {:?}",
                entry.archive_path
            )));
        }
        previous = Some(&entry.archive_path);
        if !case_paths.insert(windows_case_key(entry.archive_path.trim_end_matches('/'))) {
            return Err(ImageError::Invalid(format!(
                "image path {:?} collides on Windows",
                entry.archive_path
            )));
        }
        total_size = total_size.checked_add(entry.size).ok_or_else(|| {
            ImageError::Invalid("image uncompressed size overflows u64".to_owned())
        })?;
    }
    if total_size > MAX_UNCOMPRESSED_BYTES {
        return Err(ImageError::Invalid(format!(
            "image exceeds the {MAX_UNCOMPRESSED_BYTES}-byte uncompressed limit"
        )));
    }
    Ok(())
}

fn write_archive(
    output: &mut File,
    metadata: &[u8],
    entries: &[BuildEntry],
) -> Result<(), ImageError> {
    let mut archive = ZipWriter::new(output);
    archive.start_file(IMAGE_METADATA_PATH, file_options())?;
    archive
        .write_all(metadata)
        .map_err(|source| io_error("write image metadata", IMAGE_METADATA_PATH, source))?;

    for entry in entries {
        match entry.kind {
            EntryKind::Directory => {
                archive.add_directory(&entry.archive_path, directory_options())?
            }
            EntryKind::File => {
                archive.start_file(&entry.archive_path, file_options())?;
                write_source_entry(&mut archive, entry)?;
            }
        }
    }
    let _ = archive.finish()?;
    Ok(())
}

fn write_source_entry<W: Write + Seek>(
    archive: &mut ZipWriter<W>,
    entry: &BuildEntry,
) -> Result<(), ImageError> {
    let source = entry.source.as_deref().ok_or_else(|| {
        ImageError::Invalid(format!("file entry {:?} has no source", entry.archive_path))
    })?;
    let expected_sha256 = entry.sha256.as_deref().ok_or_else(|| {
        ImageError::Invalid(format!("file entry {:?} has no hash", entry.archive_path))
    })?;
    let mut input = File::open(source).map_err(|error| io_error("open", source, error))?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| io_error("read", source, error))?;
        if read == 0 {
            break;
        }
        size = size.checked_add(read as u64).ok_or_else(|| {
            ImageError::Invalid(format!("source {} is too large", source.display()))
        })?;
        hasher.update(&buffer[..read]);
        archive
            .write_all(&buffer[..read])
            .map_err(|error| io_error("write archive entry", &entry.archive_path, error))?;
    }
    let actual_sha256 = digest_string(hasher.finalize().as_slice());
    if size != entry.size || actual_sha256 != expected_sha256 {
        return Err(ImageError::Invalid(format!(
            "source {} changed while the image was being built",
            source.display()
        )));
    }
    Ok(())
}

fn file_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9))
        .last_modified_time(DateTime::default())
        .unix_permissions(0o644)
}

fn directory_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::default())
        .unix_permissions(0o755)
}

fn usable_parent(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn normalized_host_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn host_paths_equal(left: &Path, right: &Path) -> bool {
    normalized_host_path(left).trim_end_matches('/')
        == normalized_host_path(right).trim_end_matches('/')
}

fn host_path_is_same_or_ancestor(ancestor: &Path, path: &Path) -> bool {
    let ancestor = normalized_host_path(ancestor)
        .trim_end_matches('/')
        .to_owned();
    let path = normalized_host_path(path).trim_end_matches('/').to_owned();
    path == ancestor
        || path
            .strip_prefix(&ancestor)
            .is_some_and(|remainder| remainder.starts_with('/'))
}
