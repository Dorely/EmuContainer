//! Windows v1 runtime for verified, single-file EmuContainer images.
//!
//! Images are immutable inputs. The runtime materializes a content-addressed
//! cache, creates a writable per-image instance, mounts stable per-game state,
//! and supervises the emulator as one process tree. This is an image/runtime
//! lifecycle model, not a security sandbox.

mod fs_util;
mod instance;
mod launch;
mod lifecycle;
mod locks;
mod logging;
mod materialize;
mod paths;
mod process;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

pub use launch::{RunOptions, RunReport};
pub use lifecycle::{InstallReport, PruneReport, RuntimeStatus, UninstallReport};
pub use paths::RuntimePaths;

pub fn run_image(image: &Path, options: RunOptions) -> Result<RunReport> {
    require_supported_platform()?;
    let paths = RuntimePaths::discover()?;
    launch::run(image, &paths, options)
}

pub fn run_image_with_paths(
    image: &Path,
    paths: &RuntimePaths,
    options: RunOptions,
) -> Result<RunReport> {
    launch::run(image, paths, options)
}

pub fn install_runtime() -> Result<InstallReport> {
    require_supported_platform()?;
    lifecycle::install(&RuntimePaths::discover()?)
}

pub fn runtime_status() -> Result<RuntimeStatus> {
    if !supported_platform() {
        return Ok(RuntimeStatus {
            supported: false,
            installed: false,
            healthy: false,
            install_root: PathBuf::new(),
            executable: PathBuf::new(),
            version: None,
            expected_sha256: None,
            actual_sha256: None,
            association_registered: false,
            diagnostics: vec![
                "the EmuContainer v1 runtime is supported only on Windows x86-64".to_owned(),
            ],
        });
    }
    lifecycle::status(&RuntimePaths::discover()?)
}

pub fn prune_runtime() -> Result<PruneReport> {
    require_supported_platform()?;
    lifecycle::prune(&RuntimePaths::discover()?)
}

pub fn uninstall_runtime() -> Result<UninstallReport> {
    require_supported_platform()?;
    lifecycle::uninstall(&RuntimePaths::discover()?)
}

fn supported_platform() -> bool {
    cfg!(all(windows, target_arch = "x86_64"))
}

fn require_supported_platform() -> Result<()> {
    if !supported_platform() {
        bail!("the EmuContainer v1 runtime is supported only on Windows x86-64");
    }
    Ok(())
}
