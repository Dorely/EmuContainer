use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use anyhow::{Context, Result, bail};
use fs4::{FileExt, TryLockError};

/// An operating-system file lock held until this value is dropped.
#[derive(Debug)]
pub struct RuntimeLock {
    _file: File,
}

impl RuntimeLock {
    pub fn acquire(path: &Path, description: &str) -> Result<Self> {
        let file = open_lock_file(path)?;
        FileExt::lock(&file).with_context(|| format!("could not acquire {description} lock"))?;
        Ok(Self { _file: file })
    }

    pub fn try_acquire(path: &Path, description: &str) -> Result<Self> {
        let file = open_lock_file(path)?;
        match FileExt::try_lock(&file) {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => {
                bail!("{description} is already active; try again after it exits")
            }
            Err(TryLockError::Error(error)) => {
                Err(error).with_context(|| format!("could not acquire {description} lock"))
            }
        }
    }

    pub fn try_acquire_shared(path: &Path, description: &str) -> Result<Self> {
        let file = open_lock_file(path)?;
        match FileExt::try_lock_shared(&file) {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => {
                bail!("{description} is already active; try again after it exits")
            }
            Err(TryLockError::Error(error)) => {
                Err(error).with_context(|| format!("could not acquire {description} lock"))
            }
        }
    }
}

fn open_lock_file(path: &Path) -> Result<File> {
    let parent = path
        .parent()
        .context("runtime lock path does not have a parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("could not create lock directory {}", parent.display()))?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("could not open runtime lock {}", path.display()))
}
