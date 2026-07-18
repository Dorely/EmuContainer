use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("could not read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(encoded)
}

pub fn atomic_write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .context("JSON output path does not have a parent directory")?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;

    let temporary = temporary_sibling(path, "json");
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("could not create {}", temporary.display()))?;
        serde_json::to_writer_pretty(&mut file, value)
            .with_context(|| format!("could not serialize {}", path.display()))?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        replace_file(&temporary, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("invalid JSON in {}", path.display()))
}

pub fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("could not inspect {}", source.display()))?;
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        bail!(
            "runtime directory source must be a normal directory: {}",
            source.display()
        );
    }
    fs::create_dir_all(destination)
        .with_context(|| format!("could not create {}", destination.display()))?;
    copy_directory_contents(source, destination)
}

fn copy_directory_contents(source: &Path, destination: &Path) -> Result<()> {
    let mut entries = fs::read_dir(source)
        .with_context(|| format!("could not read {}", source.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)
            .with_context(|| format!("could not inspect {}", source_path.display()))?;
        if is_link_or_reparse(&metadata) {
            bail!(
                "runtime trees cannot contain links or reparse points: {}",
                source_path.display()
            );
        }
        if metadata.is_dir() {
            fs::create_dir(&destination_path)
                .with_context(|| format!("could not create {}", destination_path.display()))?;
            copy_directory_contents(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "could not copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        } else {
            bail!("unsupported runtime file type: {}", source_path.display());
        }
    }
    Ok(())
}

pub fn remove_tree_no_follow(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("could not inspect {}", path.display()));
        }
    };

    if is_reparse(&metadata) {
        remove_reparse(path, metadata.is_dir())?;
        return Ok(());
    }

    if metadata.file_type().is_symlink() {
        remove_link(path, metadata.is_dir())?;
        return Ok(());
    }

    if metadata.is_dir() {
        for entry in
            fs::read_dir(path).with_context(|| format!("could not read {}", path.display()))?
        {
            remove_tree_no_follow(&entry?.path())?;
        }
        fs::remove_dir(path).with_context(|| format!("could not remove {}", path.display()))?;
    } else {
        fs::remove_file(path).with_context(|| format!("could not remove {}", path.display()))?;
    }
    Ok(())
}

pub fn is_directory_empty(path: &Path) -> Result<bool> {
    Ok(fs::read_dir(path)
        .with_context(|| format!("could not read {}", path.display()))?
        .next()
        .transpose()?
        .is_none())
}

pub fn temporary_sibling(path: &Path, purpose: &str) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("runtime");
    path.with_file_name(format!(".{name}.{purpose}-{}.tmp", std::process::id()))
}

fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("could not replace {}", destination.display()))?;
    }
    fs::rename(source, destination).with_context(|| {
        format!(
            "could not publish {} as {}",
            source.display(),
            destination.display()
        )
    })
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

fn remove_link(path: &Path, is_directory: bool) -> Result<()> {
    let result = if is_directory {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };
    result.with_context(|| format!("could not remove link {}", path.display()))
}

#[cfg(windows)]
fn remove_reparse(path: &Path, is_directory: bool) -> Result<()> {
    if junction::exists(path).unwrap_or(false) {
        junction::delete(path)
            .with_context(|| format!("could not clear junction {}", path.display()))?;
        fs::remove_dir(path)
            .with_context(|| format!("could not remove junction directory {}", path.display()))
    } else {
        remove_link(path, is_directory)
    }
}

#[cfg(not(windows))]
fn remove_reparse(path: &Path, is_directory: bool) -> Result<()> {
    remove_link(path, is_directory)
}
