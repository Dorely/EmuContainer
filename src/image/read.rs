use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use tempfile::Builder as TempBuilder;
use zip::{CompressionMethod, DateTime, ZipArchive};

use super::{
    EntryKind, IMAGE_METADATA_PATH, ImageError, ImageManifest, InventoryEntry, MAX_ARCHIVE_ENTRIES,
    MAX_IMAGE_METADATA_BYTES, digest_string, io_error, path_error, sha256_file,
};
use crate::path_policy::{
    is_reparse_point, join_portable, validate_archive_path, windows_case_key,
};

#[derive(Clone, Debug)]
pub struct VerifiedImage {
    pub manifest: ImageManifest,
    pub image_sha256: String,
    pub image_size: u64,
}

#[derive(Clone, Debug)]
struct ArchiveEntry {
    index: usize,
    path: String,
    kind: EntryKind,
    size: u64,
}

/// Verify the outer file and every archive entry without materializing it.
pub fn verify_image(path: &Path) -> Result<VerifiedImage, ImageError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| io_error("inspect", path, source))?;
    if is_reparse_point(&metadata) || !metadata.is_file() {
        return Err(ImageError::Invalid(format!(
            "image {} must be a regular, non-reparse file",
            path.display()
        )));
    }

    let (first_size, first_sha256) = sha256_file(path)?;
    let manifest = verify_archive(path)?;
    let (image_size, image_sha256) = sha256_file(path)?;
    if first_size != image_size || first_sha256 != image_sha256 {
        return Err(ImageError::Invalid(format!(
            "image {} changed while it was being verified",
            path.display()
        )));
    }

    Ok(VerifiedImage {
        manifest,
        image_sha256,
        image_size,
    })
}

/// Fully verify and extract an image into a destination that does not yet
/// exist. Extraction is staged beside the destination and atomically renamed.
pub fn extract_image(
    path: &Path,
    destination: &Path,
    expected_sha256: &str,
) -> Result<VerifiedImage, ImageError> {
    let verified = verify_image(path)?;
    if verified.image_sha256 != expected_sha256 {
        return Err(ImageError::DigestMismatch {
            subject: format!("image {}", path.display()),
            expected: expected_sha256.to_owned(),
            actual: verified.image_sha256,
        });
    }
    if destination.exists() {
        return Err(ImageError::Invalid(format!(
            "extraction destination {} already exists",
            destination.display()
        )));
    }

    let parent = usable_parent(destination);
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|source| io_error("inspect extraction parent", parent, source))?;
    if is_reparse_point(&parent_metadata) || !parent_metadata.is_dir() {
        return Err(ImageError::Invalid(format!(
            "extraction parent {} must be a regular, non-reparse directory",
            parent.display()
        )));
    }

    let staging = TempBuilder::new()
        .prefix(".emucontainer-extract-")
        .tempdir_in(parent)
        .map_err(|source| io_error("create extraction staging directory in", parent, source))?;
    extract_archive(path, staging.path(), &verified.manifest)?;

    let (final_size, final_sha256) = sha256_file(path)?;
    if final_size != verified.image_size || final_sha256 != verified.image_sha256 {
        return Err(ImageError::Invalid(format!(
            "image {} changed while it was being extracted",
            path.display()
        )));
    }

    let staging_path = staging.keep();
    if let Err(source) = fs::rename(&staging_path, destination) {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(io_error("publish extracted image to", destination, source));
    }

    Ok(verified)
}

fn verify_archive(path: &Path) -> Result<ImageManifest, ImageError> {
    let mut archive = open_canonical_archive(path)?;
    let entries = scan_archive(&mut archive)?;

    let metadata_entry = entries
        .iter()
        .find(|entry| entry.path == IMAGE_METADATA_PATH)
        .ok_or_else(|| ImageError::Invalid("image archive is missing image.json".to_owned()))?;
    if metadata_entry.kind != EntryKind::File {
        return Err(ImageError::Invalid(
            "image.json must be a regular file".to_owned(),
        ));
    }
    if metadata_entry.size > MAX_IMAGE_METADATA_BYTES {
        return Err(ImageError::Invalid(format!(
            "image.json exceeds the {MAX_IMAGE_METADATA_BYTES}-byte limit"
        )));
    }

    let metadata_bytes = read_entry_bytes(
        &mut archive,
        metadata_entry.index,
        metadata_entry.size,
        IMAGE_METADATA_PATH,
    )?;
    let manifest: ImageManifest = serde_json::from_slice(&metadata_bytes)?;
    manifest.validate()?;
    let canonical = canonical_metadata(&manifest)?;
    if metadata_bytes != canonical {
        return Err(ImageError::Invalid(
            "image.json is not in canonical EmuContainer JSON form".to_owned(),
        ));
    }

    verify_inventory_shape(&entries, &manifest.inventory)?;
    for inventory_entry in &manifest.inventory {
        if inventory_entry.kind == EntryKind::File {
            let archive_entry = entries
                .iter()
                .find(|entry| entry.path == inventory_entry.path)
                .expect("inventory shape was checked");
            let actual = hash_archive_entry(
                &mut archive,
                archive_entry.index,
                inventory_entry.size,
                &inventory_entry.path,
            )?;
            let expected = inventory_entry
                .sha256
                .as_deref()
                .expect("manifest validation requires file hashes");
            if actual != expected {
                return Err(ImageError::DigestMismatch {
                    subject: format!("archive entry {}", inventory_entry.path),
                    expected: expected.to_owned(),
                    actual,
                });
            }
        }
    }

    Ok(manifest)
}

fn scan_archive(archive: &mut ZipArchive<File>) -> Result<Vec<ArchiveEntry>, ImageError> {
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ImageError::Invalid(format!(
            "image archive exceeds the {MAX_ARCHIVE_ENTRIES}-entry limit"
        )));
    }
    if !archive.comment().is_empty() {
        return Err(ImageError::Invalid(
            "image archive comments are not supported".to_owned(),
        ));
    }

    let mut entries = Vec::with_capacity(archive.len());
    let mut case_paths = HashSet::new();
    let mut previous: Option<String> = None;
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index)?;
        if index == 0 && entry.header_start() != 0 {
            return Err(ImageError::Invalid(
                "bytes before the first ZIP entry are not supported".to_owned(),
            ));
        }
        if entry.encrypted() {
            return Err(ImageError::Invalid(format!(
                "encrypted archive entry at index {index} is not supported"
            )));
        }
        if entry.is_symlink() {
            return Err(ImageError::Invalid(format!(
                "symbolic-link archive entry at index {index} is not supported"
            )));
        }
        if !entry.comment().is_empty() {
            return Err(ImageError::Invalid(format!(
                "archive entry comments are not supported at index {index}"
            )));
        }
        if entry.extra_data().is_some_and(|data| !data.is_empty()) {
            return Err(ImageError::Invalid(format!(
                "archive entry at index {index} contains unsupported extra metadata"
            )));
        }
        if entry.last_modified() != Some(DateTime::default()) {
            return Err(ImageError::Invalid(format!(
                "archive entry at index {index} has a noncanonical timestamp"
            )));
        }

        let raw_name = std::str::from_utf8(entry.name_raw()).map_err(|_| {
            ImageError::Invalid(format!(
                "archive entry at index {index} does not have a UTF-8 name"
            ))
        })?;
        if raw_name != entry.name() {
            return Err(ImageError::Invalid(format!(
                "archive entry at index {index} has a noncanonical name encoding"
            )));
        }

        let kind = if entry.is_dir() {
            EntryKind::Directory
        } else if entry.is_file() {
            EntryKind::File
        } else {
            return Err(ImageError::Invalid(format!(
                "archive entry {raw_name:?} is not a regular file or directory"
            )));
        };
        validate_archive_path(raw_name, kind == EntryKind::Directory).map_err(|error| {
            path_error(format!("invalid archive entry at index {index}"), error)
        })?;
        let expected_compression = match kind {
            EntryKind::File => CompressionMethod::Deflated,
            EntryKind::Directory => CompressionMethod::Stored,
        };
        if entry.compression() != expected_compression {
            return Err(ImageError::Invalid(format!(
                "archive entry {raw_name:?} uses unsupported compression {:?}",
                entry.compression()
            )));
        }
        if kind == EntryKind::Directory && entry.size() != 0 {
            return Err(ImageError::Invalid(format!(
                "directory archive entry {raw_name:?} is not empty"
            )));
        }
        let expected_mode = match kind {
            EntryKind::File => 0o100644,
            EntryKind::Directory => 0o040755,
        };
        if entry.unix_mode() != Some(expected_mode) {
            return Err(ImageError::Invalid(format!(
                "archive entry {raw_name:?} has noncanonical permissions"
            )));
        }
        if previous.as_deref().is_some_and(|path| path >= raw_name) {
            return Err(ImageError::Invalid(format!(
                "archive entries are not in canonical sorted order at {raw_name:?}"
            )));
        }
        previous = Some(raw_name.to_owned());
        if !case_paths.insert(windows_case_key(raw_name.trim_end_matches('/'))) {
            return Err(ImageError::Invalid(format!(
                "archive entry {raw_name:?} collides with another path on Windows"
            )));
        }

        entries.push(ArchiveEntry {
            index,
            path: raw_name.to_owned(),
            kind,
            size: entry.size(),
        });
    }

    Ok(entries)
}

fn verify_inventory_shape(
    archive_entries: &[ArchiveEntry],
    inventory: &[InventoryEntry],
) -> Result<(), ImageError> {
    if archive_entries.len() != inventory.len() + 1 {
        return Err(ImageError::Invalid(format!(
            "archive contains {} entries but image.json declares {} payload entries",
            archive_entries.len(),
            inventory.len()
        )));
    }

    let expected = inventory
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    for archive_entry in archive_entries {
        if archive_entry.path == IMAGE_METADATA_PATH {
            continue;
        }
        let inventory_entry = expected.get(archive_entry.path.as_str()).ok_or_else(|| {
            ImageError::Invalid(format!(
                "archive contains undeclared entry {:?}",
                archive_entry.path
            ))
        })?;
        if archive_entry.kind != inventory_entry.kind {
            return Err(ImageError::Invalid(format!(
                "archive entry {:?} kind does not match inventory",
                archive_entry.path
            )));
        }
        if archive_entry.size != inventory_entry.size {
            return Err(ImageError::Invalid(format!(
                "archive entry {:?} size does not match inventory",
                archive_entry.path
            )));
        }
    }
    Ok(())
}

fn extract_archive(
    image_path: &Path,
    destination: &Path,
    expected_manifest: &ImageManifest,
) -> Result<(), ImageError> {
    let mut archive = open_canonical_archive(image_path)?;
    let entries = scan_archive(&mut archive)?;
    verify_inventory_shape(&entries, &expected_manifest.inventory)?;
    let inventory = expected_manifest
        .inventory
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let canonical_metadata = canonical_metadata(expected_manifest)?;

    for entry in &entries {
        let output_path = join_portable(destination, entry.path.trim_end_matches('/'))
            .map_err(|error| path_error("invalid extraction path", error))?;
        match entry.kind {
            EntryKind::Directory => fs::create_dir_all(&output_path)
                .map_err(|source| io_error("create directory", &output_path, source))?,
            EntryKind::File => {
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|source| io_error("create directory", parent, source))?;
                }
                let bytes_or_hash = if entry.path == IMAGE_METADATA_PATH {
                    let bytes =
                        read_entry_bytes(&mut archive, entry.index, entry.size, &entry.path)?;
                    if bytes != canonical_metadata {
                        return Err(ImageError::Invalid(
                            "image.json changed during extraction".to_owned(),
                        ));
                    }
                    EntryData::Bytes(bytes)
                } else {
                    let expected = inventory
                        .get(entry.path.as_str())
                        .expect("inventory shape was checked");
                    EntryData::HashAndSize {
                        sha256: expected
                            .sha256
                            .as_deref()
                            .expect("manifest validation requires file hashes"),
                        size: expected.size,
                    }
                };
                write_extracted_file(
                    &mut archive,
                    entry.index,
                    &output_path,
                    &entry.path,
                    bytes_or_hash,
                )?;
            }
        }
    }

    Ok(())
}

fn open_canonical_archive(path: &Path) -> Result<ZipArchive<File>, ImageError> {
    const LOCAL_FILE_HEADER: &[u8; 4] = b"PK\x03\x04";
    const END_OF_CENTRAL_DIRECTORY: &[u8; 4] = b"PK\x05\x06";
    const END_RECORD_SIZE: u64 = 22;

    let mut file = File::open(path).map_err(|source| io_error("open", path, source))?;
    let size = file
        .metadata()
        .map_err(|source| io_error("inspect", path, source))?
        .len();
    if size < END_RECORD_SIZE {
        return Err(ImageError::Invalid(
            "image is too short to be a ZIP archive".to_owned(),
        ));
    }

    let mut signature = [0_u8; 4];
    file.read_exact(&mut signature)
        .map_err(|source| io_error("read ZIP header", path, source))?;
    if &signature != LOCAL_FILE_HEADER {
        return Err(ImageError::Invalid(
            "image contains bytes before the first ZIP entry".to_owned(),
        ));
    }
    file.seek(SeekFrom::End(-(END_RECORD_SIZE as i64)))
        .map_err(|source| io_error("seek to ZIP footer", path, source))?;
    file.read_exact(&mut signature)
        .map_err(|source| io_error("read ZIP footer", path, source))?;
    if &signature != END_OF_CENTRAL_DIRECTORY {
        return Err(ImageError::Invalid(
            "image has trailing bytes or a noncanonical ZIP footer".to_owned(),
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|source| io_error("rewind", path, source))?;
    ZipArchive::new(file).map_err(ImageError::from)
}

enum EntryData<'a> {
    Bytes(Vec<u8>),
    HashAndSize { sha256: &'a str, size: u64 },
}

fn write_extracted_file(
    archive: &mut ZipArchive<File>,
    index: usize,
    output_path: &Path,
    archive_path: &str,
    expected: EntryData<'_>,
) -> Result<(), ImageError> {
    let mut output = File::options()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|source| io_error("create extracted file", output_path, source))?;

    match expected {
        EntryData::Bytes(bytes) => output
            .write_all(&bytes)
            .map_err(|source| io_error("write extracted file", output_path, source))?,
        EntryData::HashAndSize { sha256, size } => {
            let mut input = archive.by_index(index)?;
            let mut hasher = Sha256::new();
            let mut written = 0_u64;
            let mut buffer = [0_u8; 128 * 1024];
            loop {
                let read = input.read(&mut buffer).map_err(|source| {
                    io_error("read archive entry", PathBuf::from(archive_path), source)
                })?;
                if read == 0 {
                    break;
                }
                written = written.checked_add(read as u64).ok_or_else(|| {
                    ImageError::Invalid(format!("archive entry {archive_path:?} is too large"))
                })?;
                if written > size {
                    return Err(ImageError::Invalid(format!(
                        "archive entry {archive_path:?} expands beyond its declared size"
                    )));
                }
                hasher.update(&buffer[..read]);
                output
                    .write_all(&buffer[..read])
                    .map_err(|source| io_error("write extracted file", output_path, source))?;
            }
            let actual = digest_string(hasher.finalize().as_slice());
            if written != size {
                return Err(ImageError::Invalid(format!(
                    "archive entry {archive_path:?} extracted to {written} bytes, expected {size}"
                )));
            }
            if actual != sha256 {
                return Err(ImageError::DigestMismatch {
                    subject: format!("archive entry {archive_path}"),
                    expected: sha256.to_owned(),
                    actual,
                });
            }
        }
    }

    output
        .sync_all()
        .map_err(|source| io_error("flush extracted file", output_path, source))?;
    Ok(())
}

fn read_entry_bytes(
    archive: &mut ZipArchive<File>,
    index: usize,
    expected_size: u64,
    name: &str,
) -> Result<Vec<u8>, ImageError> {
    let capacity = usize::try_from(expected_size).map_err(|_| {
        ImageError::Invalid(format!("archive entry {name:?} is too large for this host"))
    })?;
    let mut entry = archive.by_index(index)?;
    let mut bytes = Vec::with_capacity(capacity);
    entry
        .by_ref()
        .take(expected_size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| io_error("read archive entry", PathBuf::from(name), source))?;
    if bytes.len() as u64 != expected_size {
        return Err(ImageError::Invalid(format!(
            "archive entry {name:?} read as {} bytes, expected {expected_size}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn hash_archive_entry(
    archive: &mut ZipArchive<File>,
    index: usize,
    expected_size: u64,
    name: &str,
) -> Result<String, ImageError> {
    let mut entry = archive.by_index(index)?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = entry
            .read(&mut buffer)
            .map_err(|source| io_error("read archive entry", PathBuf::from(name), source))?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .ok_or_else(|| ImageError::Invalid(format!("archive entry {name:?} is too large")))?;
        if size > expected_size {
            return Err(ImageError::Invalid(format!(
                "archive entry {name:?} expands beyond its declared size"
            )));
        }
        hasher.update(&buffer[..read]);
    }
    if size != expected_size {
        return Err(ImageError::Invalid(format!(
            "archive entry {name:?} read as {size} bytes, expected {expected_size}"
        )));
    }
    Ok(digest_string(hasher.finalize().as_slice()))
}

fn canonical_metadata(manifest: &ImageManifest) -> Result<Vec<u8>, ImageError> {
    let mut bytes = serde_json::to_vec(manifest)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn usable_parent(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}
