use std::collections::{BTreeMap, HashMap, HashSet};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    path_policy::{
        is_same_or_ancestor, validate_archive_path, validate_relative_path, windows_case_key,
    },
    recipe::{
        Content, Emulator, Recipe, RecipeError, SUPPORTED_SCHEMA_VERSION, State, StateMount, Target,
    },
};

pub const IMAGE_FORMAT_VERSION: u32 = 1;
pub const MINIMUM_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const IMAGE_METADATA_PATH: &str = "image.json";
pub const EMULATOR_ROOT: &str = "payload/emulator";
pub const CONTENT_ROOT: &str = "payload/content";
pub const NOTICES_ROOT: &str = "notices";
pub const MAX_ARCHIVE_ENTRIES: usize = 100_000;
pub const MAX_IMAGE_METADATA_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024 * 1024;

/// Canonical metadata stored as `image.json` in every image.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageManifest {
    pub format_version: u32,
    pub minimum_runtime_version: String,
    pub target: Target,
    pub game: GameMetadata,
    pub emulator: ImageEmulator,
    pub content: ImageContent,
    pub state: ImageState,
    pub notices: Vec<ImageNotice>,
    pub isolation: Isolation,
    pub network_enforced: bool,
    /// Every payload/notice file and explicitly stored empty directory. The
    /// metadata file itself is necessarily excluded to avoid a self-hash.
    pub inventory: Vec<InventoryEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameMetadata {
    pub id: String,
    pub title: String,
    pub system: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageEmulator {
    pub id: String,
    pub version: String,
    pub source: String,
    pub root: String,
    pub entrypoint: String,
    pub working_directory: String,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageContent {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImageState {
    pub mounts: Vec<StateMount>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageNotice {
    pub path: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Isolation {
    None,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryEntry {
    pub path: String,
    pub kind: EntryKind,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    File,
    Directory,
}

impl ImageManifest {
    pub(crate) fn from_recipe(
        recipe: &Recipe,
        content_size: u64,
        notices: Vec<ImageNotice>,
        inventory: Vec<InventoryEntry>,
    ) -> Self {
        Self {
            format_version: IMAGE_FORMAT_VERSION,
            minimum_runtime_version: MINIMUM_RUNTIME_VERSION.to_owned(),
            target: recipe.target,
            game: GameMetadata {
                id: recipe.id.clone(),
                title: recipe.title.clone(),
                system: recipe.system.clone(),
            },
            emulator: ImageEmulator {
                id: recipe.emulator.id.clone(),
                version: recipe.emulator.version.clone(),
                source: recipe.emulator.source.clone(),
                root: EMULATOR_ROOT.to_owned(),
                entrypoint: recipe.emulator.entrypoint.clone(),
                working_directory: recipe.emulator.working_directory.clone(),
                arguments: recipe.emulator.arguments.clone(),
                environment: recipe.emulator.environment.clone(),
            },
            content: ImageContent {
                path: format!("{CONTENT_ROOT}/{}", recipe.content.filename),
                filename: recipe.content.filename.clone(),
                size: content_size,
                sha256: recipe.content.sha256.clone(),
            },
            state: ImageState {
                mounts: recipe.state.mounts.clone(),
            },
            notices,
            isolation: Isolation::None,
            network_enforced: false,
            inventory,
        }
    }

    /// Validate semantic invariants that cannot be expressed by Serde alone.
    pub fn validate(&self) -> Result<(), ImageValidationError> {
        let mut errors = Vec::new();

        if self.format_version != IMAGE_FORMAT_VERSION {
            errors.push(format!(
                "format_version must be {IMAGE_FORMAT_VERSION}, got {}",
                self.format_version
            ));
        }
        if self.minimum_runtime_version.trim().is_empty()
            || self
                .minimum_runtime_version
                .chars()
                .any(|character| character.is_control())
        {
            errors.push("minimum_runtime_version is invalid".to_owned());
        } else if Version::parse(&self.minimum_runtime_version).is_err() {
            errors.push("minimum_runtime_version must be valid semantic versioning".to_owned());
        }
        if self.emulator.root != EMULATOR_ROOT {
            errors.push(format!("emulator.root must be {EMULATOR_ROOT:?}"));
        }
        if self.network_enforced {
            errors.push("network_enforced must be false for image format v1".to_owned());
        }

        let recipe = Recipe {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            id: self.game.id.clone(),
            title: self.game.title.clone(),
            system: self.game.system.clone(),
            target: self.target,
            emulator: Emulator {
                id: self.emulator.id.clone(),
                version: self.emulator.version.clone(),
                source: self.emulator.source.clone(),
                entrypoint: self.emulator.entrypoint.clone(),
                working_directory: self.emulator.working_directory.clone(),
                arguments: self.emulator.arguments.clone(),
                environment: self.emulator.environment.clone(),
            },
            content: Content {
                filename: self.content.filename.clone(),
                sha256: self.content.sha256.clone(),
            },
            notices: Vec::new(),
            state: State {
                mounts: self.state.mounts.clone(),
            },
        };
        if let Err(RecipeError::Validation(recipe_errors)) = recipe.validate() {
            errors.extend(
                recipe_errors
                    .into_iter()
                    .map(|error| format!("embedded contract: {error}")),
            );
        }

        let expected_content_path = format!("{CONTENT_ROOT}/{}", self.content.filename);
        if self.content.path != expected_content_path {
            errors.push(format!(
                "content.path must be {expected_content_path:?}, got {:?}",
                self.content.path
            ));
        }

        validate_inventory(self, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ImageValidationError { errors })
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageValidationError {
    errors: Vec<String>,
}

impl ImageValidationError {
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn into_errors(self) -> Vec<String> {
        self.errors
    }
}

impl std::fmt::Display for ImageValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(formatter, "image metadata validation failed:")?;
        for error in &self.errors {
            writeln!(formatter, "- {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ImageValidationError {}

fn validate_inventory(manifest: &ImageManifest, errors: &mut Vec<String>) {
    if manifest.inventory.len() > MAX_ARCHIVE_ENTRIES.saturating_sub(1) {
        errors.push(format!(
            "inventory exceeds the maximum of {} payload entries",
            MAX_ARCHIVE_ENTRIES - 1
        ));
    }

    let mut previous_path: Option<&str> = None;
    let mut case_keys = HashSet::new();
    let mut entries = HashMap::new();
    let mut total_size = 0_u64;

    for (index, entry) in manifest.inventory.iter().enumerate() {
        if previous_path.is_some_and(|previous| previous >= entry.path.as_str()) {
            errors.push(format!(
                "inventory must be strictly sorted; entry {index} is out of order or duplicated"
            ));
        }
        previous_path = Some(&entry.path);

        if let Err(error) = validate_archive_path(&entry.path, entry.kind == EntryKind::Directory) {
            errors.push(format!("inventory[{index}].path: {}", error.reason()));
            continue;
        }
        if !case_keys.insert(windows_case_key(entry.path.trim_end_matches('/'))) {
            errors.push(format!(
                "inventory[{index}].path collides with another entry on Windows"
            ));
        }
        if entry.path == IMAGE_METADATA_PATH {
            errors.push("inventory must not contain image.json".to_owned());
        }

        match entry.kind {
            EntryKind::File => {
                let Some(hash) = entry.sha256.as_deref() else {
                    errors.push(format!("inventory[{index}] file is missing sha256"));
                    continue;
                };
                validate_hash(&format!("inventory[{index}].sha256"), hash, errors);
                total_size = match total_size.checked_add(entry.size) {
                    Some(total) => total,
                    None => {
                        errors.push("inventory total size overflows u64".to_owned());
                        u64::MAX
                    }
                };
            }
            EntryKind::Directory => {
                if entry.size != 0 {
                    errors.push(format!("inventory[{index}] directory size must be zero"));
                }
                if entry.sha256.is_some() {
                    errors.push(format!(
                        "inventory[{index}] directory cannot contain sha256"
                    ));
                }
            }
        }
        entries.insert(entry.path.trim_end_matches('/'), entry);
    }

    if total_size > MAX_UNCOMPRESSED_BYTES {
        errors.push(format!(
            "inventory exceeds the {MAX_UNCOMPRESSED_BYTES}-byte uncompressed limit"
        ));
    }

    validate_required_file(
        &entries,
        &manifest.content.path,
        manifest.content.size,
        &manifest.content.sha256,
        "content",
        errors,
    );
    let entrypoint = format!("{EMULATOR_ROOT}/{}", manifest.emulator.entrypoint);
    validate_file_exists(&entries, &entrypoint, "emulator entrypoint", errors);

    if manifest.emulator.working_directory != "." {
        let working_directory = format!("{EMULATOR_ROOT}/{}", manifest.emulator.working_directory);
        validate_directory_exists(
            &entries,
            &working_directory,
            "emulator working directory",
            errors,
        );
    }

    let mut notice_paths = HashSet::new();
    for (index, notice) in manifest.notices.iter().enumerate() {
        if validate_relative_path(&notice.path).is_err()
            || !notice.path.starts_with(&format!("{NOTICES_ROOT}/"))
        {
            errors.push(format!(
                "notices[{index}].path must be below {NOTICES_ROOT}/"
            ));
            continue;
        }
        if !notice_paths.insert(windows_case_key(&notice.path)) {
            errors.push(format!(
                "notices[{index}].path duplicates another notice on Windows"
            ));
        }
        validate_file_exists(&entries, &notice.path, "notice", errors);
    }

    for mount in &manifest.state.mounts {
        let target = format!("{EMULATOR_ROOT}/{}", mount.target);
        validate_directory_exists(&entries, &target, "state mount target", errors);
    }

    for entry in &manifest.inventory {
        let path = entry.path.trim_end_matches('/');
        let known_notice = notice_paths.contains(&windows_case_key(path));
        if path != manifest.content.path
            && !is_same_or_ancestor(EMULATOR_ROOT, path)
            && !known_notice
        {
            errors.push(format!(
                "inventory contains undeclared path {:?}",
                entry.path
            ));
        }
    }
}

fn validate_required_file(
    entries: &HashMap<&str, &InventoryEntry>,
    path: &str,
    size: u64,
    sha256: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    match entries.get(path) {
        Some(entry) if entry.kind != EntryKind::File => {
            errors.push(format!("{label} path {path:?} is not a file"));
        }
        Some(entry) => {
            if entry.size != size {
                errors.push(format!("{label} size does not match inventory"));
            }
            if entry.sha256.as_deref() != Some(sha256) {
                errors.push(format!("{label} sha256 does not match inventory"));
            }
        }
        None => errors.push(format!("{label} path {path:?} is missing from inventory")),
    }
}

fn validate_file_exists(
    entries: &HashMap<&str, &InventoryEntry>,
    path: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    match entries.get(path) {
        Some(entry) if entry.kind == EntryKind::File => {}
        Some(_) => errors.push(format!("{label} path {path:?} is not a file")),
        None => errors.push(format!("{label} path {path:?} is missing from inventory")),
    }
}

fn validate_directory_exists(
    entries: &HashMap<&str, &InventoryEntry>,
    path: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    if entries
        .get(path)
        .is_some_and(|entry| entry.kind == EntryKind::File)
    {
        errors.push(format!("{label} path {path:?} is a file"));
        return;
    }

    let prefix = format!("{path}/");
    if !entries.contains_key(path)
        && !entries
            .keys()
            .any(|candidate| candidate.starts_with(&prefix))
    {
        errors.push(format!("{label} path {path:?} is missing from inventory"));
    }
}

fn validate_hash(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        errors.push(format!(
            "{field} must contain exactly 64 lowercase hexadecimal characters"
        ));
    }
}
