use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::Serialize;

use crate::{
    image::{ImageManifest, VerifiedImage, verify_image},
    recipe::{LaunchTokens, Target, expand_launch_value},
};

use super::{
    fs_util::sha256_file,
    instance::{ensure_game_identity, ensure_instance, prepare_volumes},
    locks::RuntimeLock,
    logging::{EventLogger, unix_milliseconds},
    materialize::ensure_materialized,
    paths::RuntimePaths,
    process,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct RunOptions {
    pub json_events: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunReport {
    pub game_id: String,
    pub image_sha256: String,
    pub run_id: String,
    pub cache_rootfs: PathBuf,
    pub instance_rootfs: PathBuf,
    pub log_directory: PathBuf,
    pub process_id: u32,
    pub exit_code: i32,
    pub remaining_process_ids: Vec<u32>,
}

pub fn run(image_path: &Path, paths: &RuntimePaths, options: RunOptions) -> Result<RunReport> {
    require_windows()?;
    paths.ensure_runtime_roots()?;
    let _lifecycle = RuntimeLock::try_acquire_shared(
        &paths.lifecycle_lock(),
        "a runtime prune, install, or uninstall operation",
    )?;
    let mut launcher_events = create_launcher_logger(paths)?;
    launcher_events.write(
        "launch_requested",
        serde_json::json!({ "image": image_path }),
    )?;
    let image = match verify_image(image_path)
        .with_context(|| format!("image verification failed for {}", image_path.display()))
    {
        Ok(image) => image,
        Err(error) => {
            let _ = launcher_events.write(
                "launch_rejected",
                serde_json::json!({ "image": image_path, "error": format!("{error:#}") }),
            );
            return Err(error);
        }
    };
    validate_runtime_compatibility(&image.manifest)?;
    let _game = RuntimeLock::try_acquire(
        &paths.game_lock(&image.manifest.game.id)?,
        &format!("game '{}'", image.manifest.game.id),
    )?;

    let log_directory = create_run_directory(paths, &image.manifest.game.id)?;
    let run_id = log_directory
        .file_name()
        .and_then(|name| name.to_str())
        .context("run log directory does not have a UTF-8 name")?
        .to_owned();
    let mut events = EventLogger::create(&log_directory.join("events.jsonl"), options.json_events)?;
    launcher_events.write(
        "game_log_ready",
        serde_json::json!({
            "game_id": image.manifest.game.id,
            "image_sha256": image.image_sha256,
            "log_directory": log_directory,
        }),
    )?;
    events.write(
        "image_verified",
        serde_json::json!({
            "image": image_path,
            "image_sha256": image.image_sha256,
            "image_size": image.image_size,
            "game_id": image.manifest.game.id,
        }),
    )?;

    match run_verified(
        image_path,
        paths,
        &image,
        &run_id,
        &log_directory,
        &mut events,
    ) {
        Ok(report) => Ok(report),
        Err(error) => {
            let _ = events.write(
                "run_failed",
                serde_json::json!({ "error": format!("{error:#}") }),
            );
            Err(error)
        }
    }
}

fn run_verified(
    image_path: &Path,
    paths: &RuntimePaths,
    image: &VerifiedImage,
    run_id: &str,
    log_directory: &Path,
    events: &mut EventLogger,
) -> Result<RunReport> {
    let materialized = ensure_materialized(image_path, image, paths)?;
    let cache_rootfs = materialized.rootfs;
    events.write(
        "cache_ready",
        serde_json::json!({
            "rootfs": cache_rootfs,
            "disposition": materialized.disposition,
        }),
    )?;

    ensure_game_identity(paths, &image.manifest)?;
    let instance_rootfs =
        ensure_instance(paths, &image.manifest, &image.image_sha256, &cache_rootfs)?;
    events.write(
        "instance_ready",
        serde_json::json!({ "rootfs": instance_rootfs }),
    )?;

    let mounted = prepare_volumes(
        paths,
        &image.manifest,
        &image.image_sha256,
        &instance_rootfs,
    )?;
    for volume in &mounted {
        events.write(
            "volume_mounted",
            serde_json::json!({
                "name": volume.name,
                "target": volume.target,
                "volume": volume.volume,
                "seeded": volume.seeded,
            }),
        )?;
    }

    let state = paths.game_state(&image.manifest.game.id)?;
    paths.ensure_data_directory(&state)?;
    let command_line = prepare_command(&image.manifest, &instance_rootfs, &state, log_directory)?;
    events.write(
        "launch_prepared",
        serde_json::json!({
            "executable": command_line.executable,
            "working_directory": command_line.working_directory,
            "arguments": command_line.arguments,
            "environment": command_line.environment,
            "stdout": log_directory.join("stdout.log"),
            "stderr": log_directory.join("stderr.log"),
        }),
    )?;

    let stdout = File::create(log_directory.join("stdout.log"))?;
    let stderr = File::create(log_directory.join("stderr.log"))?;
    let mut command = Command::new(&command_line.executable);
    command
        .args(&command_line.arguments)
        .envs(&command_line.environment)
        .current_dir(&command_line.working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    let (process_id, outcome) = process::supervise(
        || {
            command.spawn().with_context(|| {
                format!(
                    "could not launch emulator {}",
                    command_line.executable.display()
                )
            })
        },
        |process_id| {
            events.write(
                "process_started",
                serde_json::json!({ "process_id": process_id }),
            )
        },
    )?;
    let exit_code = outcome.status.code().unwrap_or(1);
    events.write(
        "process_exited",
        serde_json::json!({
            "process_id": process_id,
            "exit_code": exit_code,
            "success": outcome.status.success(),
            "remaining_process_ids": outcome.remaining_process_ids,
        }),
    )?;
    events.write(
        "run_completed",
        serde_json::json!({
            "exit_code": exit_code,
            "remaining_process_count": outcome.remaining_process_ids.len(),
        }),
    )?;

    Ok(RunReport {
        game_id: image.manifest.game.id.clone(),
        image_sha256: image.image_sha256.clone(),
        run_id: run_id.to_owned(),
        cache_rootfs,
        instance_rootfs,
        log_directory: log_directory.to_path_buf(),
        process_id,
        exit_code,
        remaining_process_ids: outcome.remaining_process_ids,
    })
}

#[derive(Debug)]
struct PreparedCommand {
    executable: PathBuf,
    working_directory: PathBuf,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
}

fn prepare_command(
    manifest: &ImageManifest,
    instance_rootfs: &Path,
    state: &Path,
    logs: &Path,
) -> Result<PreparedCommand> {
    let emulator = instance_rootfs.join("emulator");
    let rom_relative = manifest
        .content
        .path
        .strip_prefix("payload/")
        .context("image content path is outside payload/")?;
    let rom = join_portable(instance_rootfs, rom_relative);
    let executable = join_portable(&emulator, &manifest.emulator.entrypoint);
    let working_directory = if manifest.emulator.working_directory == "." {
        emulator.clone()
    } else {
        join_portable(&emulator, &manifest.emulator.working_directory)
    };
    if !executable.is_file() {
        bail!("emulator entrypoint is missing: {}", executable.display());
    }
    if !rom.is_file() {
        bail!("packaged ROM is missing: {}", rom.display());
    }
    if !working_directory.is_dir() {
        bail!(
            "emulator working directory is missing: {}",
            working_directory.display()
        );
    }
    verify_instance_file(
        instance_rootfs,
        &executable,
        manifest
            .inventory
            .iter()
            .find(|entry| {
                entry.path == format!("payload/emulator/{}", manifest.emulator.entrypoint)
            })
            .and_then(|entry| entry.sha256.as_deref())
            .context("image inventory does not hash the emulator entrypoint")?,
        "emulator entrypoint",
    )?;
    verify_instance_file(
        instance_rootfs,
        &rom,
        &manifest.content.sha256,
        "packaged ROM",
    )?;

    let tokens = LaunchTokens {
        emulator: &emulator,
        rom: &rom,
        state,
        logs,
        instance: instance_rootfs,
    };
    let arguments = manifest
        .emulator
        .arguments
        .iter()
        .map(|argument| expand_launch_value(argument, &tokens).map_err(anyhow::Error::from))
        .collect::<Result<Vec<_>>>()?;
    let environment = manifest
        .emulator
        .environment
        .iter()
        .map(|(name, value)| {
            expand_launch_value(value, &tokens)
                .map(|value| (name.clone(), value))
                .map_err(anyhow::Error::from)
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    Ok(PreparedCommand {
        executable,
        working_directory,
        arguments,
        environment,
    })
}

fn verify_instance_file(
    root: &Path,
    path: &Path,
    expected_sha256: &str,
    label: &str,
) -> Result<()> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("{label} is outside the container instance"))?;
    let mut current = root.to_path_buf();
    for component in std::iter::once(std::path::Component::CurDir).chain(relative.components()) {
        if component != std::path::Component::CurDir {
            let std::path::Component::Normal(component) = component else {
                bail!("{label} uses a non-normal runtime path");
            };
            current.push(component);
        }
        let metadata = fs::symlink_metadata(&current)
            .with_context(|| format!("could not inspect {label} path {}", current.display()))?;
        if crate::path_policy::is_reparse_point(&metadata) {
            bail!(
                "{label} path contains a link or reparse point: {}",
                current.display()
            );
        }
    }
    let actual_sha256 = sha256_file(path)?;
    if actual_sha256 != expected_sha256 {
        bail!(
            "{label} in the writable instance no longer matches the verified image; prune the disposable instance before launching"
        );
    }
    Ok(())
}

fn validate_runtime_compatibility(manifest: &ImageManifest) -> Result<()> {
    if manifest.target != Target::WindowsX86_64 {
        bail!(
            "image target '{}' is not supported by this runtime",
            manifest.target
        );
    }
    let minimum = Version::parse(&manifest.minimum_runtime_version).with_context(|| {
        format!(
            "image minimum_runtime_version {:?} is not valid semantic versioning",
            manifest.minimum_runtime_version
        )
    })?;
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    if current < minimum {
        bail!("image requires EmuContainer {minimum} or newer; current runtime is {current}");
    }
    Ok(())
}

fn require_windows() -> Result<()> {
    if !cfg!(all(windows, target_arch = "x86_64")) {
        bail!("the EmuContainer v1 runtime is supported only on Windows x86-64");
    }
    Ok(())
}

fn create_run_directory(paths: &RuntimePaths, game_id: &str) -> Result<PathBuf> {
    let logs = paths.game_logs(game_id)?;
    paths.ensure_data_directory(&logs)?;
    let base = format!("{}-{}", unix_milliseconds(), std::process::id());
    for suffix in 0_u16..=u16::MAX {
        let name = if suffix == 0 {
            base.clone()
        } else {
            format!("{base}-{suffix}")
        };
        let candidate = logs.join(name);
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("could not create {}", candidate.display()));
            }
        }
    }
    bail!("could not allocate a unique run log directory")
}

fn create_launcher_logger(paths: &RuntimePaths) -> Result<EventLogger> {
    let logs = paths.launcher_logs();
    paths.ensure_data_directory(&logs)?;
    let base = format!("{}-{}", unix_milliseconds(), std::process::id());
    for suffix in 0_u16..=u16::MAX {
        let name = if suffix == 0 {
            format!("{base}.jsonl")
        } else {
            format!("{base}-{suffix}.jsonl")
        };
        match EventLogger::create(&logs.join(name), false) {
            Ok(logger) => return Ok(logger),
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::AlreadyExists) =>
            {
                continue;
            }
            Err(error) => return Err(error),
        }
    }
    bail!("could not allocate a unique launcher log")
}

fn join_portable(root: &Path, portable: &str) -> PathBuf {
    portable
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}
