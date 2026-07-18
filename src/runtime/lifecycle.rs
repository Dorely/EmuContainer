use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::path_policy::is_reparse_point;

use super::{
    fs_util::{
        atomic_write_json, read_json, remove_tree_no_follow, sha256_file, temporary_sibling,
    },
    locks::RuntimeLock,
    paths::RuntimePaths,
};

const INSTALL_FORMAT_VERSION: u32 = 2;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InstallRecord {
    format_version: u32,
    generation: String,
    version: String,
    executable_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeStatus {
    pub supported: bool,
    pub installed: bool,
    pub healthy: bool,
    pub install_root: PathBuf,
    pub executable: PathBuf,
    pub version: Option<String>,
    pub expected_sha256: Option<String>,
    pub actual_sha256: Option<String>,
    pub association_registered: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InstallReport {
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub association_registered: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PruneReport {
    pub removed_image_caches: u64,
    pub removed_container_instances: u64,
    pub preserved_game_data_root: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct UninstallReport {
    pub deferred_to_worker: bool,
    pub removed_installation: bool,
    pub removed_association: bool,
    pub preserved_game_data_root: PathBuf,
}

pub fn install(paths: &RuntimePaths) -> Result<InstallReport> {
    require_windows()?;
    paths.ensure_runtime_roots()?;
    let _lifecycle =
        RuntimeLock::try_acquire(&paths.lifecycle_lock(), "runtime lifecycle operation")?;

    let source = std::env::current_exe().context("could not locate the running executable")?;
    let destination = paths.installed_executable();
    paths.ensure_install_root()?;

    if !same_existing_file(&source, &destination)? {
        copy_executable_atomic(&source, &destination)?;
    }

    let executable_sha256 = sha256_file(&destination)?;
    let record = InstallRecord {
        format_version: INSTALL_FORMAT_VERSION,
        generation: install_generation(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        executable_sha256: executable_sha256.clone(),
    };
    atomic_write_json(&paths.install_record(), &record)?;
    platform::register_association(&destination)?;

    Ok(InstallReport {
        executable: destination,
        executable_sha256,
        association_registered: true,
    })
}

pub fn status(paths: &RuntimePaths) -> Result<RuntimeStatus> {
    let executable = paths.installed_executable();
    let mut diagnostics = Vec::new();
    if !cfg!(windows) {
        diagnostics.push("the v1 runtime is supported only on Windows x86-64".to_owned());
        return Ok(RuntimeStatus {
            supported: false,
            installed: false,
            healthy: false,
            install_root: paths.install_root(),
            executable,
            version: None,
            expected_sha256: None,
            actual_sha256: None,
            association_registered: false,
            diagnostics,
        });
    }

    let installed = executable.is_file();
    let record = match read_json::<InstallRecord>(&paths.install_record()) {
        Ok(record) if record.format_version == INSTALL_FORMAT_VERSION => Some(record),
        Ok(_) => {
            diagnostics.push("install.json uses an unsupported format version".to_owned());
            None
        }
        Err(error) => {
            diagnostics.push(format!("could not validate install.json: {error:#}"));
            None
        }
    };

    let actual_sha256 = if installed {
        match sha256_file(&executable) {
            Ok(hash) => Some(hash),
            Err(error) => {
                diagnostics.push(format!("could not hash installed executable: {error:#}"));
                None
            }
        }
    } else {
        diagnostics.push("installed executable is missing".to_owned());
        None
    };
    let association_registered =
        platform::association_matches(&executable).unwrap_or_else(|error| {
            diagnostics.push(format!("could not inspect .emuc association: {error:#}"));
            false
        });
    if !association_registered {
        diagnostics.push(".emuc association does not point to this installation".to_owned());
    }

    let expected_sha256 = record
        .as_ref()
        .map(|record| record.executable_sha256.clone());
    if expected_sha256.is_some() && expected_sha256 != actual_sha256 {
        diagnostics.push("installed executable hash does not match install.json".to_owned());
    }
    let version = record.as_ref().map(|record| record.version.clone());
    let healthy = installed
        && record.is_some()
        && expected_sha256 == actual_sha256
        && association_registered
        && diagnostics.is_empty();

    Ok(RuntimeStatus {
        supported: true,
        installed,
        healthy,
        install_root: paths.install_root(),
        executable,
        version,
        expected_sha256,
        actual_sha256,
        association_registered,
        diagnostics,
    })
}

pub fn prune(paths: &RuntimePaths) -> Result<PruneReport> {
    require_windows()?;
    paths.ensure_runtime_roots()?;
    let _lifecycle =
        RuntimeLock::try_acquire(&paths.lifecycle_lock(), "runtime lifecycle operation")?;

    let removed_image_caches = remove_children(&paths.image_cache_root())?;
    let removed_container_instances = remove_container_instances(&paths.containers_root())?;
    Ok(PruneReport {
        removed_image_caches,
        removed_container_instances,
        preserved_game_data_root: paths.games_root(),
    })
}

pub fn uninstall(paths: &RuntimePaths) -> Result<UninstallReport> {
    require_windows()?;
    paths.ensure_runtime_roots()?;
    let current = std::env::current_exe().context("could not locate the running executable")?;
    let installed = paths.installed_executable();
    let is_worker = std::env::var_os("EMUCONTAINER_UNINSTALL_WORKER").is_some();

    if !is_worker && same_existing_file(&current, &installed)? {
        let expected_sha256 = sha256_file(&installed)?;
        let install_record: InstallRecord = read_json(&paths.install_record())
            .context("could not read the installed runtime generation")?;
        if install_record.format_version != INSTALL_FORMAT_VERSION
            || install_record.executable_sha256 != expected_sha256
        {
            bail!(
                "installed runtime metadata does not match its executable; uninstall was not started"
            );
        }
        let worker = platform::create_uninstall_worker(&current)?;
        let mut command = std::process::Command::new(&worker);
        command
            .args(["runtime", "uninstall"])
            .env("EMUCONTAINER_UNINSTALL_WORKER", "1")
            .env("EMUCONTAINER_UNINSTALL_EXPECTED_SHA256", expected_sha256)
            .env(
                "EMUCONTAINER_UNINSTALL_EXPECTED_GENERATION",
                install_record.generation,
            );
        platform::configure_hidden(&mut command);
        if let Err(error) = command.spawn() {
            let _ = fs::remove_file(&worker);
            return Err(error)
                .with_context(|| format!("could not start uninstall worker {}", worker.display()));
        }
        return Ok(UninstallReport {
            deferred_to_worker: true,
            removed_installation: false,
            removed_association: false,
            preserved_game_data_root: paths.games_root(),
        });
    }

    let _lifecycle = if is_worker {
        RuntimeLock::acquire(&paths.lifecycle_lock(), "runtime lifecycle operation")?
    } else {
        RuntimeLock::try_acquire(&paths.lifecycle_lock(), "runtime lifecycle operation")?
    };
    if is_worker {
        let expected = std::env::var("EMUCONTAINER_UNINSTALL_EXPECTED_SHA256")
            .context("uninstall worker is missing its expected installation hash")?;
        let expected_generation = std::env::var("EMUCONTAINER_UNINSTALL_EXPECTED_GENERATION")
            .context("uninstall worker is missing its expected installation generation")?;
        let install_record: InstallRecord = read_json(&paths.install_record()).context(
            "the installed runtime metadata changed or disappeared before deferred uninstall",
        )?;
        let actual = sha256_file(&installed).context(
            "the installed runtime changed or disappeared before the uninstall worker acquired its lock",
        )?;
        if install_record.format_version != INSTALL_FORMAT_VERSION
            || install_record.generation != expected_generation
            || install_record.executable_sha256 != expected
            || actual != expected
        {
            bail!(
                "the installed runtime changed before deferred uninstall; the replacement was preserved"
            );
        }
    }
    let removed_association = platform::remove_owned_association(&installed)?;
    let removed_installation = remove_installation_with_retry(&paths.install_root(), is_worker)?;
    if is_worker {
        platform::schedule_worker_cleanup(&current)?;
    }
    Ok(UninstallReport {
        deferred_to_worker: false,
        removed_installation,
        removed_association,
        preserved_game_data_root: paths.games_root(),
    })
}

fn require_windows() -> Result<()> {
    if !cfg!(all(windows, target_arch = "x86_64")) {
        bail!("the EmuContainer v1 runtime is supported only on Windows x86-64");
    }
    Ok(())
}

fn install_generation() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos}-{}", std::process::id())
}

fn same_existing_file(left: &Path, right: &Path) -> Result<bool> {
    if !left.exists() || !right.exists() {
        return Ok(false);
    }
    let left =
        fs::canonicalize(left).with_context(|| format!("could not resolve {}", left.display()))?;
    let right = fs::canonicalize(right)
        .with_context(|| format!("could not resolve {}", right.display()))?;
    Ok(left == right)
}

fn copy_executable_atomic(source: &Path, destination: &Path) -> Result<()> {
    let temporary = temporary_sibling(destination, "install");
    if temporary.exists() {
        fs::remove_file(&temporary)
            .with_context(|| format!("could not remove stale {}", temporary.display()))?;
    }
    fs::copy(source, &temporary).with_context(|| {
        format!(
            "could not copy runtime from {} to {}",
            source.display(),
            temporary.display()
        )
    })?;
    if destination.exists() {
        fs::remove_file(destination).with_context(|| {
            format!(
                "could not replace {}; run install from a different copy of EmuContainer",
                destination.display()
            )
        })?;
    }
    fs::rename(&temporary, destination)
        .with_context(|| format!("could not publish {}", destination.display()))
}

fn remove_children(root: &Path) -> Result<u64> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(error).with_context(|| format!("could not inspect {}", root.display()));
        }
    };
    if is_reparse_point(&metadata) || !metadata.is_dir() {
        remove_tree_no_follow(root)?;
        fs::create_dir(root)
            .with_context(|| format!("could not recreate normal directory {}", root.display()))?;
        return Ok(1);
    }
    let entries = fs::read_dir(root)
        .with_context(|| format!("could not read {}", root.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    let mut removed = 0;
    for entry in entries {
        remove_tree_no_follow(&entry.path())?;
        removed += 1;
    }
    Ok(removed)
}

fn remove_container_instances(root: &Path) -> Result<u64> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(error).with_context(|| format!("could not inspect {}", root.display()));
        }
    };
    if is_reparse_point(&metadata) || !metadata.is_dir() {
        remove_tree_no_follow(root)?;
        fs::create_dir(root)
            .with_context(|| format!("could not recreate normal directory {}", root.display()))?;
        return Ok(1);
    }
    let mut removed = 0;
    for game in fs::read_dir(root).with_context(|| format!("could not read {}", root.display()))? {
        let game = game?;
        let game_metadata = fs::symlink_metadata(game.path())
            .with_context(|| format!("could not inspect {}", game.path().display()))?;
        if is_reparse_point(&game_metadata) || !game_metadata.is_dir() {
            remove_tree_no_follow(&game.path())?;
            removed += 1;
            continue;
        }
        removed += remove_children(&game.path())?;
        if game.path().read_dir()?.next().is_none() {
            fs::remove_dir(game.path())?;
        }
    }
    Ok(removed)
}

fn remove_installation_with_retry(install_root: &Path, retry: bool) -> Result<bool> {
    if !install_root.exists() {
        return Ok(false);
    }
    let attempts = if retry { 150 } else { 1 };
    let mut last_error = None;
    for _ in 0..attempts {
        match remove_tree_no_follow(install_root) {
            Ok(()) => return Ok(true),
            Err(error) => last_error = Some(error),
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    match last_error {
        Some(error) => Err(error),
        None => bail!("could not remove runtime installation"),
    }
}

#[cfg(windows)]
mod platform {
    use std::{
        fs::{File, OpenOptions},
        io::{self, Write},
        path::{Path, PathBuf},
        process::{Command, Stdio},
        time::{SystemTime, UNIX_EPOCH},
    };

    use anyhow::{Context, Result};
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    };

    const EXTENSION_KEY: &str = r"Software\Classes\.emuc";
    const PROG_ID: &str = "EmuContainer.Image";
    const PROG_ID_KEY: &str = r"Software\Classes\EmuContainer.Image";

    pub fn register_association(executable: &Path) -> Result<()> {
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let (extension, _) = current_user
            .create_subkey(EXTENSION_KEY)
            .context("could not create the .emuc registry key")?;
        extension.set_value("", &PROG_ID)?;

        let (program, _) = current_user.create_subkey(PROG_ID_KEY)?;
        program.set_value("", &"EmuContainer Game Image")?;
        let (command, _) = program.create_subkey(r"shell\open\command")?;
        command.set_value("", &open_command(executable))?;
        Ok(())
    }

    pub fn association_matches(executable: &Path) -> Result<bool> {
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let extension = match current_user.open_subkey(EXTENSION_KEY) {
            Ok(key) => key,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let extension_value: String = extension.get_value("")?;
        if extension_value != PROG_ID {
            return Ok(false);
        }
        let command = match current_user.open_subkey(format!(r"{PROG_ID_KEY}\shell\open\command")) {
            Ok(key) => key,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let command_value: String = command.get_value("")?;
        Ok(command_value == open_command(executable))
    }

    pub fn remove_owned_association(executable: &Path) -> Result<bool> {
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let extension_owned = match current_user.open_subkey(EXTENSION_KEY) {
            Ok(key) => key
                .get_value::<String, _>("")
                .is_ok_and(|value| value == PROG_ID),
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        let program_owned =
            match current_user.open_subkey(format!(r"{PROG_ID_KEY}\shell\open\command")) {
                Ok(key) => key
                    .get_value::<String, _>("")
                    .is_ok_and(|value| value == open_command(executable)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => false,
                Err(error) => return Err(error.into()),
            };

        if !extension_owned && !program_owned {
            return Ok(false);
        }

        if program_owned {
            let command_path = format!(r"{PROG_ID_KEY}\shell\open\command");
            let command = current_user
                .open_subkey_with_flags(&command_path, KEY_READ | KEY_WRITE)
                .context("could not open the owned image open-command key")?;
            command.delete_value("")?;
            drop(command);
            delete_key_if_empty(&current_user, &command_path)?;
            delete_key_if_empty(&current_user, &format!(r"{PROG_ID_KEY}\shell\open"))?;
            delete_key_if_empty(&current_user, &format!(r"{PROG_ID_KEY}\shell"))?;

            if let Ok(program) =
                current_user.open_subkey_with_flags(PROG_ID_KEY, KEY_READ | KEY_WRITE)
                && program
                    .get_value::<String, _>("")
                    .is_ok_and(|value| value == "EmuContainer Game Image")
            {
                program.delete_value("")?;
            }
            delete_key_if_empty(&current_user, PROG_ID_KEY)?;
        }

        if extension_owned {
            let extension = current_user
                .open_subkey_with_flags(EXTENSION_KEY, KEY_READ | KEY_WRITE)
                .context("could not open the owned .emuc registry key for removal")?;
            extension.delete_value("")?;
            let metadata = extension.query_info()?;
            drop(extension);
            if metadata.sub_keys == 0 && metadata.values == 0 {
                current_user.delete_subkey(EXTENSION_KEY)?;
            }
        }
        Ok(true)
    }

    fn delete_key_if_empty(parent: &RegKey, path: &str) -> Result<()> {
        let key = match parent.open_subkey_with_flags(path, KEY_READ | KEY_WRITE) {
            Ok(key) => key,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let metadata = key.query_info()?;
        drop(key);
        if metadata.sub_keys == 0 && metadata.values == 0 {
            parent.delete_subkey(path)?;
        }
        Ok(())
    }

    fn open_command(executable: &Path) -> String {
        format!(r#""{}" run "%1""#, executable.display())
    }

    pub fn configure_hidden(command: &mut Command) {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    pub fn create_uninstall_worker(source: &Path) -> Result<PathBuf> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for suffix in 0_u16..=u16::MAX {
            let path = std::env::temp_dir().join(format!(
                "emucontainer-uninstall-{}-{nonce}-{suffix}.exe",
                std::process::id()
            ));
            let mut worker = match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(worker) => worker,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("could not create {}", path.display()));
                }
            };
            let mut input = File::open(source)
                .with_context(|| format!("could not open {}", source.display()))?;
            if let Err(error) = io::copy(&mut input, &mut worker) {
                drop(worker);
                let _ = std::fs::remove_file(&path);
                return Err(error).with_context(|| format!("could not write {}", path.display()));
            }
            if let Err(error) = worker.sync_all() {
                drop(worker);
                let _ = std::fs::remove_file(&path);
                return Err(error).with_context(|| format!("could not flush {}", path.display()));
            }
            drop(worker);
            return Ok(path);
        }
        anyhow::bail!("could not allocate a unique uninstall worker")
    }

    pub fn schedule_worker_cleanup(worker: &Path) -> Result<()> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut cleanup = None;
        for suffix in 0_u16..=u16::MAX {
            let path = std::env::temp_dir().join(format!(
                "emucontainer-cleanup-{}-{nonce}-{suffix}.cmd",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    file.write_all(
                        b"@echo off\r\nfor /L %%i in (1,1,1000) do (\r\n  del /F /Q \"%~1\" >nul 2>&1\r\n  if not exist \"%~1\" goto cleaned\r\n)\r\n:cleaned\r\ndel /F /Q \"%~f0\" >nul 2>&1\r\n",
                    )?;
                    file.sync_all()?;
                    cleanup = Some(path);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).context("could not create uninstall cleanup script");
                }
            }
        }
        let cleanup = cleanup.context("could not allocate an uninstall cleanup script")?;
        let mut command = Command::new("cmd.exe");
        command
            .args(["/D", "/Q", "/C"])
            .arg(&cleanup)
            .arg(worker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        configure_hidden(&mut command);
        command
            .spawn()
            .context("could not start uninstall cleanup script")?;
        Ok(())
    }
}

#[cfg(not(windows))]
mod platform {
    use std::{
        path::{Path, PathBuf},
        process::Command,
    };

    use anyhow::{Result, bail};

    pub fn register_association(_executable: &Path) -> Result<()> {
        bail!(".emuc file association is supported only on Windows")
    }

    pub fn association_matches(_executable: &Path) -> Result<bool> {
        Ok(false)
    }

    pub fn remove_owned_association(_executable: &Path) -> Result<bool> {
        bail!(".emuc file association is supported only on Windows")
    }

    pub fn configure_hidden(_command: &mut Command) {}

    pub fn create_uninstall_worker(_source: &Path) -> Result<PathBuf> {
        bail!("uninstall workers are supported only on Windows")
    }

    pub fn schedule_worker_cleanup(_worker: &Path) -> Result<()> {
        bail!("uninstall-worker cleanup is supported only on Windows")
    }
}
