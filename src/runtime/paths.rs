use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::path_policy::is_reparse_point;

const PRODUCT_DIRECTORY: &str = "EmuContainer";

/// All user-scoped locations owned by the runtime.
#[derive(Clone, Debug)]
pub struct RuntimePaths {
    local_app_data: PathBuf,
}

impl RuntimePaths {
    /// Discover the Windows user-local application data directory.
    pub fn discover() -> Result<Self> {
        let local_app_data = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .context("LOCALAPPDATA is not set; EmuContainer requires a Windows user profile")?;
        Self::from_local_app_data(local_app_data)
    }

    /// Build a path layout rooted at an explicit LocalAppData directory.
    ///
    /// This is useful to inspect or manually exercise runtime behavior without
    /// changing the active user's normal data directory.
    pub fn from_local_app_data(local_app_data: impl Into<PathBuf>) -> Result<Self> {
        let local_app_data = local_app_data.into();
        if !local_app_data.is_absolute() {
            bail!("the LocalAppData root must be an absolute path");
        }
        Ok(Self { local_app_data })
    }

    pub fn local_app_data(&self) -> &Path {
        &self.local_app_data
    }

    pub fn install_root(&self) -> PathBuf {
        self.local_app_data.join("Programs").join(PRODUCT_DIRECTORY)
    }

    pub fn installed_executable(&self) -> PathBuf {
        self.install_root().join("emucontainer.exe")
    }

    pub fn install_record(&self) -> PathBuf {
        self.install_root().join("install.json")
    }

    pub fn data_root(&self) -> PathBuf {
        self.local_app_data.join(PRODUCT_DIRECTORY)
    }

    pub fn image_cache_root(&self) -> PathBuf {
        self.data_root().join("cache").join("images")
    }

    pub fn image_cache(&self, digest: &str) -> Result<PathBuf> {
        validate_component("image digest", digest)?;
        Ok(self.image_cache_root().join(digest))
    }

    pub fn containers_root(&self) -> PathBuf {
        self.data_root().join("containers")
    }

    pub fn container(&self, game_id: &str, digest: &str) -> Result<PathBuf> {
        validate_component("game ID", game_id)?;
        validate_component("image digest", digest)?;
        Ok(self.containers_root().join(game_id).join(digest))
    }

    pub fn games_root(&self) -> PathBuf {
        self.data_root().join("games")
    }

    pub fn game(&self, game_id: &str) -> Result<PathBuf> {
        validate_component("game ID", game_id)?;
        Ok(self.games_root().join(game_id))
    }

    pub fn game_state(&self, game_id: &str) -> Result<PathBuf> {
        Ok(self.game(game_id)?.join("state"))
    }

    pub fn game_volumes(&self, game_id: &str) -> Result<PathBuf> {
        Ok(self.game(game_id)?.join("volumes"))
    }

    pub fn game_logs(&self, game_id: &str) -> Result<PathBuf> {
        Ok(self.game(game_id)?.join("logs"))
    }

    pub fn launcher_logs(&self) -> PathBuf {
        self.data_root().join("logs").join("launcher")
    }

    pub fn locks(&self) -> PathBuf {
        self.data_root().join("locks")
    }

    pub fn game_lock(&self, game_id: &str) -> Result<PathBuf> {
        validate_component("game ID", game_id)?;
        Ok(self.locks().join(format!("game-{game_id}.lock")))
    }

    pub fn image_lock(&self, digest: &str) -> Result<PathBuf> {
        validate_component("image digest", digest)?;
        Ok(self.locks().join(format!("image-{digest}.lock")))
    }

    pub fn lifecycle_lock(&self) -> PathBuf {
        self.locks().join("lifecycle.lock")
    }

    pub fn ensure_runtime_roots(&self) -> Result<()> {
        std::fs::create_dir_all(&self.local_app_data).with_context(|| {
            format!(
                "could not create LocalAppData root {}",
                self.local_app_data.display()
            )
        })?;
        for path in [
            self.data_root(),
            self.data_root().join("cache"),
            self.image_cache_root(),
            self.containers_root(),
            self.games_root(),
            self.data_root().join("logs"),
            self.launcher_logs(),
            self.locks(),
        ] {
            ensure_plain_directory(&path)?;
        }
        Ok(())
    }

    pub(crate) fn ensure_install_root(&self) -> Result<()> {
        let programs = self.local_app_data.join("Programs");
        std::fs::create_dir_all(&programs)
            .with_context(|| format!("could not create {}", programs.display()))?;
        ensure_plain_directory(&self.install_root())
    }

    pub(crate) fn ensure_data_directory(&self, path: &Path) -> Result<()> {
        self.ensure_runtime_roots()?;
        let relative = path.strip_prefix(self.data_root()).with_context(|| {
            format!(
                "runtime directory {} is outside {}",
                path.display(),
                self.data_root().display()
            )
        })?;
        let mut current = self.data_root();
        for component in relative.components() {
            let std::path::Component::Normal(component) = component else {
                bail!("runtime directory {} is not a normal path", path.display());
            };
            current.push(component);
            ensure_plain_directory(&current)?;
        }
        Ok(())
    }
}

fn ensure_plain_directory(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !is_reparse_point(&metadata) => Ok(()),
        Ok(_) => bail!(
            "runtime-owned path {} must be a normal directory, not a file, link, or reparse point",
            path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(path)
            .with_context(|| format!("could not create normal directory {}", path.display())),
        Err(error) => Err(error).with_context(|| format!("could not inspect {}", path.display())),
    }
}

fn validate_component(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || matches!(value, "." | "..")
        || value.contains(['/', '\\', ':'])
        || value.chars().any(char::is_control)
    {
        bail!("{label} is not safe to use as a runtime path component");
    }
    Ok(())
}
