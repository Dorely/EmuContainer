use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::image::{EntryKind, VerifiedImage, extract_image};

use super::{
    fs_util::{
        atomic_write_json, read_json, remove_tree_no_follow, sha256_file, temporary_sibling,
    },
    locks::RuntimeLock,
    paths::RuntimePaths,
};

const CACHE_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheDisposition {
    Reused,
    Created,
    Repaired,
}

#[derive(Clone, Debug)]
pub struct MaterializedImage {
    pub rootfs: PathBuf,
    pub disposition: CacheDisposition,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CacheRecord {
    format_version: u32,
    image_sha256: String,
}

pub fn ensure_materialized(
    image_path: &Path,
    image: &VerifiedImage,
    paths: &RuntimePaths,
) -> Result<MaterializedImage> {
    let cache = paths.image_cache(&image.image_sha256)?;
    let _image_lock = RuntimeLock::acquire(
        &paths.image_lock(&image.image_sha256)?,
        "image materialization",
    )?;

    if valid_cache(&cache, image)? {
        return Ok(MaterializedImage {
            rootfs: cache.join("rootfs"),
            disposition: CacheDisposition::Reused,
        });
    }
    let disposition = if fs::symlink_metadata(&cache).is_ok() {
        CacheDisposition::Repaired
    } else {
        CacheDisposition::Created
    };
    remove_tree_no_follow(&cache)?;
    let temporary = temporary_sibling(&cache, "materialize");
    remove_tree_no_follow(&temporary)?;
    let parent = cache
        .parent()
        .context("image cache does not have a parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;

    let result = (|| {
        extract_image(image_path, &temporary, &image.image_sha256)?;
        let payload = temporary.join("payload");
        let rootfs = temporary.join("rootfs");
        fs::rename(&payload, &rootfs).with_context(|| {
            format!(
                "could not establish runtime root filesystem at {}",
                rootfs.display()
            )
        })?;
        atomic_write_json(
            &temporary.join("complete.json"),
            &CacheRecord {
                format_version: CACHE_FORMAT_VERSION,
                image_sha256: image.image_sha256.clone(),
            },
        )?;
        if !valid_cache(&temporary, image)? {
            bail!("materialized image failed its post-extraction integrity check");
        }
        fs::rename(&temporary, &cache)
            .with_context(|| format!("could not publish image cache {}", cache.display()))?;
        Ok(MaterializedImage {
            rootfs: cache.join("rootfs"),
            disposition,
        })
    })();
    if result.is_err() {
        let _ = remove_tree_no_follow(&temporary);
    }
    result
}

fn valid_cache(cache: &Path, image: &VerifiedImage) -> Result<bool> {
    if !cache.is_dir() {
        return Ok(false);
    }
    let record: CacheRecord = match read_json(&cache.join("complete.json")) {
        Ok(record) => record,
        Err(_) => return Ok(false),
    };
    if record.format_version != CACHE_FORMAT_VERSION || record.image_sha256 != image.image_sha256 {
        return Ok(false);
    }

    let cached_manifest: serde_json::Value = match read_json(&cache.join("image.json")) {
        Ok(manifest) => manifest,
        Err(_) => return Ok(false),
    };
    if cached_manifest != serde_json::to_value(&image.manifest)? {
        return Ok(false);
    }

    let mut expected_paths = BTreeSet::new();
    for entry in &image.manifest.inventory {
        let path = cache_path(cache, &entry.path);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => return Ok(false),
        };
        if is_link_or_reparse(&metadata) {
            return Ok(false);
        }
        match entry.kind {
            EntryKind::Directory => {
                if !metadata.is_dir() || entry.size != 0 || entry.sha256.is_some() {
                    return Ok(false);
                }
            }
            EntryKind::File => {
                if !metadata.is_file() || metadata.len() != entry.size {
                    return Ok(false);
                }
                let Some(expected_hash) = entry.sha256.as_deref() else {
                    return Ok(false);
                };
                if sha256_file(&path)? != expected_hash {
                    return Ok(false);
                }
            }
        }
        expected_paths.insert(relative_key(cache, &path)?);
        let mut parent = path.parent();
        while let Some(directory) = parent {
            if directory == cache {
                break;
            }
            expected_paths.insert(relative_key(cache, directory)?);
            parent = directory.parent();
        }
    }
    expected_paths.insert("complete.json".to_owned());
    expected_paths.insert("image.json".to_owned());

    let cached_paths = match collect_paths(cache) {
        Ok(paths) => paths,
        Err(_) => return Ok(false),
    };
    if cached_paths.into_iter().collect::<BTreeSet<_>>() != expected_paths {
        return Ok(false);
    }
    Ok(cache.join("rootfs/emulator").is_dir() && cache.join("rootfs/content").is_dir())
}

fn cache_path(cache: &Path, archive_path: &str) -> PathBuf {
    let archive_path = archive_path.trim_end_matches('/');
    if let Some(relative) = archive_path.strip_prefix("payload/") {
        join_portable(&cache.join("rootfs"), relative)
    } else {
        join_portable(cache, archive_path)
    }
}

fn join_portable(root: &Path, portable: &str) -> PathBuf {
    portable
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

fn collect_paths(root: &Path) -> Result<Vec<String>> {
    let mut result = Vec::new();
    collect_paths_recursive(root, root, &mut result)?;
    result.sort();
    Ok(result)
}

fn collect_paths_recursive(root: &Path, directory: &Path, result: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("could not read {}", directory.display()))?
    {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if is_link_or_reparse(&metadata) {
            bail!(
                "materialized image contains an unexpected link or reparse point: {}",
                entry.path().display()
            );
        }
        result.push(relative_key(root, &entry.path())?);
        if metadata.is_dir() {
            collect_paths_recursive(root, &entry.path(), result)?;
        } else if !metadata.is_file() {
            bail!("materialized image contains an unsupported file type");
        }
    }
    Ok(())
}

fn relative_key(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?;
    let parts = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();
    Ok(parts.join("/").to_ascii_lowercase())
}

fn is_link_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || is_reparse(metadata)
}

#[cfg(windows)]
fn is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &fs::Metadata) -> bool {
    false
}
