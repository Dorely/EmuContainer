use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::image::ImageManifest;

use super::{
    fs_util::{
        atomic_write_json, copy_tree, is_directory_empty, read_json, remove_tree_no_follow,
        temporary_sibling,
    },
    paths::RuntimePaths,
};

const IDENTITY_FORMAT_VERSION: u32 = 1;
const INSTANCE_FORMAT_VERSION: u32 = 1;
const VOLUME_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GameIdentity {
    format_version: u32,
    game_id: String,
    system: String,
    content_sha256: String,
    emulator_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InstanceRecord {
    format_version: u32,
    image_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VolumeRecord {
    format_version: u32,
    name: String,
    seeded_from_image_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct MountedVolume {
    pub name: String,
    pub target: PathBuf,
    pub volume: PathBuf,
    pub seeded: bool,
}

pub fn ensure_game_identity(paths: &RuntimePaths, manifest: &ImageManifest) -> Result<()> {
    let game_root = paths.game(&manifest.game.id)?;
    paths.ensure_data_directory(&game_root)?;
    let identity_path = game_root.join("identity.json");
    let expected = GameIdentity {
        format_version: IDENTITY_FORMAT_VERSION,
        game_id: manifest.game.id.clone(),
        system: manifest.game.system.clone(),
        content_sha256: manifest.content.sha256.clone(),
        emulator_id: manifest.emulator.id.clone(),
    };

    let identity_metadata = match fs::symlink_metadata(&identity_path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error)
                .with_context(|| format!("could not inspect {}", identity_path.display()));
        }
    };
    if let Some(metadata) = identity_metadata {
        if !metadata.is_file() || crate::path_policy::is_reparse_point(&metadata) {
            bail!(
                "{} must be a normal identity file; player data was not changed",
                identity_path.display()
            );
        }
        let actual: GameIdentity = read_json(&identity_path)?;
        if actual.format_version != IDENTITY_FORMAT_VERSION {
            bail!(
                "{} uses an unsupported identity format; player data was not changed",
                identity_path.display()
            );
        }
        if actual.game_id != expected.game_id
            || actual.system != expected.system
            || actual.content_sha256 != expected.content_sha256
            || actual.emulator_id != expected.emulator_id
        {
            bail!(
                "game ID '{}' is already bound to a different system, ROM, or emulator; choose a new game ID to protect existing saves",
                manifest.game.id
            );
        }
        return Ok(());
    }

    refuse_unowned_player_data(&game_root)?;
    atomic_write_json(&identity_path, &expected)
}

pub fn ensure_instance(
    paths: &RuntimePaths,
    manifest: &ImageManifest,
    image_sha256: &str,
    cached_rootfs: &Path,
) -> Result<PathBuf> {
    let instance = paths.container(&manifest.game.id, image_sha256)?;
    let parent = instance
        .parent()
        .context("container instance does not have a parent directory")?;
    paths.ensure_data_directory(parent)?;
    if valid_instance(&instance, image_sha256)? {
        return Ok(instance.join("rootfs"));
    }
    remove_tree_no_follow(&instance)?;

    let temporary = temporary_sibling(&instance, "instance");
    remove_tree_no_follow(&temporary)?;
    let result = (|| {
        fs::create_dir_all(&temporary)
            .with_context(|| format!("could not create {}", temporary.display()))?;
        copy_tree(cached_rootfs, &temporary.join("rootfs"))?;
        atomic_write_json(
            &temporary.join("complete.json"),
            &InstanceRecord {
                format_version: INSTANCE_FORMAT_VERSION,
                image_sha256: image_sha256.to_owned(),
            },
        )?;
        fs::rename(&temporary, &instance).with_context(|| {
            format!(
                "could not publish container instance {}",
                instance.display()
            )
        })?;
        Ok(instance.join("rootfs"))
    })();
    if result.is_err() {
        let _ = remove_tree_no_follow(&temporary);
    }
    result
}

pub fn prepare_volumes(
    paths: &RuntimePaths,
    manifest: &ImageManifest,
    image_sha256: &str,
    rootfs: &Path,
) -> Result<Vec<MountedVolume>> {
    if manifest.state.mounts.is_empty() {
        return Ok(Vec::new());
    }
    if !cfg!(windows) {
        bail!("persistent state mounts are supported only on Windows");
    }

    validate_mount_relationships(manifest)?;
    let volumes_root = paths.game_volumes(&manifest.game.id)?;
    let records_root = volumes_root.join(".initialized");
    paths.ensure_data_directory(&records_root)?;
    let emulator_root = rootfs.join("emulator");
    let mut mounted = Vec::with_capacity(manifest.state.mounts.len());

    for mount in &manifest.state.mounts {
        if mount.name == ".initialized" {
            bail!("state mount name '.initialized' is reserved by the runtime");
        }
        let volume = volumes_root.join(&mount.name);
        let record_path = records_root.join(format!("{}.json", mount.name));
        let target = join_portable(&emulator_root, &mount.target);
        let seeded = initialize_volume(&volume, &target, &record_path, &mount.name, image_sha256)?;
        platform::replace_with_junction(&target, &volume)?;
        mounted.push(MountedVolume {
            name: mount.name.clone(),
            target,
            volume,
            seeded,
        });
    }
    Ok(mounted)
}

fn valid_instance(path: &Path, image_sha256: &str) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("could not inspect {}", path.display()));
        }
    };
    if !metadata.is_dir() || crate::path_policy::is_reparse_point(&metadata) {
        return Ok(false);
    }
    let record: InstanceRecord = match read_json(&path.join("complete.json")) {
        Ok(record) => record,
        Err(_) => return Ok(false),
    };
    for required in [
        path.join("rootfs"),
        path.join("rootfs/emulator"),
        path.join("rootfs/content"),
    ] {
        let metadata = match fs::symlink_metadata(&required) {
            Ok(metadata) => metadata,
            Err(_) => return Ok(false),
        };
        if !metadata.is_dir() || crate::path_policy::is_reparse_point(&metadata) {
            return Ok(false);
        }
    }
    Ok(record.format_version == INSTANCE_FORMAT_VERSION && record.image_sha256 == image_sha256)
}

fn refuse_unowned_player_data(game_root: &Path) -> Result<()> {
    for directory in ["state", "volumes"] {
        let path = game_root.join(directory);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if crate::path_policy::is_reparse_point(&metadata) => {
                bail!(
                    "{} is a link or reparse point; player data was not inspected or changed",
                    path.display()
                );
            }
            Ok(metadata) if metadata.is_dir() => {
                if !is_directory_empty(&path)? {
                    bail!(
                        "{} contains player data but identity.json is missing; restore the identity record or move this data before launching",
                        game_root.display()
                    );
                }
            }
            Ok(_) => bail!("{} is not a normal player-data directory", path.display()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("could not inspect {}", path.display()));
            }
        }
    }
    Ok(())
}

fn validate_mount_relationships(manifest: &ImageManifest) -> Result<()> {
    let mut targets = BTreeSet::new();
    for mount in &manifest.state.mounts {
        if mount.target == "." || mount.target.is_empty() {
            bail!(
                "state mount '{}' cannot replace the emulator root",
                mount.name
            );
        }
        let folded = mount.target.to_ascii_lowercase();
        if !targets.insert(folded.clone()) {
            bail!(
                "state mount target '{}' is declared more than once",
                mount.target
            );
        }
        for previous in &targets {
            if previous == &folded {
                continue;
            }
            if portable_path_contains(previous, &folded)
                || portable_path_contains(&folded, previous)
            {
                bail!("state mount targets must not overlap or nest");
            }
        }
        let entrypoint = manifest.emulator.entrypoint.to_ascii_lowercase();
        if entrypoint == folded || portable_path_contains(&folded, &entrypoint) {
            bail!(
                "state mount '{}' would replace emulator entrypoint '{}'",
                mount.target,
                manifest.emulator.entrypoint
            );
        }
    }
    Ok(())
}

fn portable_path_contains(parent: &str, child: &str) -> bool {
    child
        .strip_prefix(parent)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn initialize_volume(
    volume: &Path,
    packaged_target: &Path,
    record_path: &Path,
    name: &str,
    image_sha256: &str,
) -> Result<bool> {
    let pending_path = record_path.with_extension("pending.json");
    let staging = temporary_sibling(volume, "seed");
    if record_path.exists() {
        let record: VolumeRecord = read_json(record_path)?;
        if record.format_version != VOLUME_FORMAT_VERSION || record.name != name {
            bail!(
                "volume initialization record {} is invalid; player data was not changed",
                record_path.display()
            );
        }
        if pending_path.exists() {
            remove_tree_no_follow(&pending_path)?;
        }
        if volume.exists() {
            ensure_normal_volume_directory(volume)?;
        } else {
            fs::create_dir(volume)
                .with_context(|| format!("could not recreate empty volume {}", volume.display()))?;
        }
        return Ok(false);
    }

    if pending_path.exists() {
        // A pending record can only be written before this function returns,
        // and mounts are created afterward. Its final/staged directories have
        // therefore never been exposed to the emulator and are safe to retry.
        remove_tree_no_follow(volume)?;
        remove_tree_no_follow(&staging)?;
        remove_tree_no_follow(&pending_path)?;
    }

    let metadata = fs::symlink_metadata(packaged_target).with_context(|| {
        format!(
            "state mount seed directory is missing from the image: {}",
            packaged_target.display()
        )
    })?;
    if !metadata.is_dir() {
        bail!(
            "state mount target must be a directory: {}",
            packaged_target.display()
        );
    }
    if volume.exists() {
        ensure_normal_volume_directory(volume)?;
        if !is_directory_empty(volume)? {
            bail!(
                "volume {} is nonempty but has no initialization record; player data was not changed",
                volume.display()
            );
        }
        fs::remove_dir(volume)
            .with_context(|| format!("could not remove empty volume {}", volume.display()))?;
    }

    remove_tree_no_follow(&staging)?;
    copy_tree(packaged_target, &staging)?;
    let record = VolumeRecord {
        format_version: VOLUME_FORMAT_VERSION,
        name: name.to_owned(),
        seeded_from_image_sha256: image_sha256.to_owned(),
    };
    atomic_write_json(&pending_path, &record)?;
    fs::rename(&staging, volume).with_context(|| {
        format!(
            "could not publish seeded volume {} as {}",
            staging.display(),
            volume.display()
        )
    })?;
    atomic_write_json(record_path, &record)?;
    remove_tree_no_follow(&pending_path)?;
    Ok(true)
}

fn ensure_normal_volume_directory(volume: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(volume)
        .with_context(|| format!("could not inspect volume {}", volume.display()))?;
    if !metadata.is_dir() || crate::path_policy::is_reparse_point(&metadata) {
        bail!(
            "volume {} must be a normal directory, not a file, link, or reparse point",
            volume.display()
        );
    }
    Ok(())
}

fn join_portable(root: &Path, portable: &str) -> PathBuf {
    portable
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

#[cfg(windows)]
mod platform {
    use std::{fs, path::Path};

    use anyhow::{Context, Result};

    use crate::runtime::fs_util::remove_tree_no_follow;

    pub fn replace_with_junction(target: &Path, volume: &Path) -> Result<()> {
        if junction::exists(target).unwrap_or(false) {
            let actual = junction::get_target(target)
                .with_context(|| format!("could not read junction {}", target.display()))?;
            let actual = fs::canonicalize(actual)?;
            let expected = fs::canonicalize(volume)?;
            if actual == expected {
                return Ok(());
            }
        }

        remove_tree_no_follow(target)?;
        let parent = target
            .parent()
            .context("state mount target does not have a parent directory")?;
        fs::create_dir_all(parent)?;
        junction::create(volume, target).with_context(|| {
            format!(
                "could not mount persistent volume {} at {}",
                volume.display(),
                target.display()
            )
        })
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    use anyhow::{Result, bail};

    pub fn replace_with_junction(_target: &Path, _volume: &Path) -> Result<()> {
        bail!("persistent state mounts are supported only on Windows")
    }
}
