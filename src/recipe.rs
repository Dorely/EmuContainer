use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::path_policy::{
    is_same_or_ancestor, validate_file_name, validate_relative_path, validate_relative_path_or_dot,
    windows_case_key,
};

pub const SUPPORTED_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub system: String,
    pub target: Target,
    pub emulator: Emulator,
    pub content: Content,
    #[serde(default)]
    pub notices: Vec<Notice>,
    #[serde(default)]
    pub state: State,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Target {
    #[serde(rename = "windows-x86_64")]
    WindowsX86_64,
}

impl fmt::Display for Target {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowsX86_64 => formatter.write_str("windows-x86_64"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Emulator {
    pub id: String,
    pub version: String,
    pub source: String,
    pub entrypoint: String,
    #[serde(default = "default_working_directory")]
    pub working_directory: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub filename: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Notice {
    pub source: String,
    pub output: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct State {
    pub mounts: Vec<StateMount>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateMount {
    pub name: String,
    pub target: String,
    pub seed: SeedPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SeedPolicy {
    IfEmpty,
}

#[derive(Debug)]
pub enum RecipeError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse(toml::de::Error),
    Validation(Vec<String>),
}

impl fmt::Display for RecipeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read recipe {}: {source}",
                    path.display()
                )
            }
            Self::Parse(source) => write!(formatter, "invalid recipe TOML: {source}"),
            Self::Validation(errors) => {
                writeln!(formatter, "recipe validation failed:")?;
                for error in errors {
                    writeln!(formatter, "- {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl Error for RecipeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse(source) => Some(source),
            Self::Validation(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LaunchTokens<'a> {
    pub emulator: &'a Path,
    pub rom: &'a Path,
    pub state: &'a Path,
    pub logs: &'a Path,
    pub instance: &'a Path,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchTokenError {
    message: String,
}

impl LaunchTokenError {
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LaunchTokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for LaunchTokenError {}

impl Recipe {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RecipeError> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|source| RecipeError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&source)
    }

    pub fn parse(source: &str) -> Result<Self, RecipeError> {
        let recipe: Self = toml::from_str(source).map_err(RecipeError::Parse)?;
        recipe.validate()?;
        Ok(recipe)
    }

    pub fn validate(&self) -> Result<(), RecipeError> {
        let mut errors = Vec::new();

        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            errors.push(format!(
                "schema_version must be {SUPPORTED_SCHEMA_VERSION}, got {}",
                self.schema_version
            ));
        }

        validate_game_id(&self.id, &mut errors);
        validate_text("title", &self.title, &mut errors);
        validate_slug("system", &self.system, &mut errors);
        validate_slug("emulator.id", &self.emulator.id, &mut errors);
        validate_text("emulator.version", &self.emulator.version, &mut errors);
        validate_source(&self.emulator.source, &mut errors);
        validate_path(
            "emulator.entrypoint",
            &self.emulator.entrypoint,
            false,
            &mut errors,
        );
        validate_path(
            "emulator.working_directory",
            &self.emulator.working_directory,
            true,
            &mut errors,
        );
        validate_path(
            "content.filename",
            &self.content.filename,
            false,
            &mut errors,
        );
        if validate_file_name(&self.content.filename).is_err() {
            errors.push("content.filename must be a single Windows-safe file name".to_owned());
        }
        validate_sha256("content.sha256", &self.content.sha256, &mut errors);

        let mut has_rom_token = false;
        for (index, argument) in self.emulator.arguments.iter().enumerate() {
            validate_launch_text(
                &format!("emulator.arguments[{index}]"),
                argument,
                &mut has_rom_token,
                &mut errors,
            );
        }
        let mut environment_names = HashSet::new();
        for (name, value) in &self.emulator.environment {
            if name.is_empty()
                || name.contains('=')
                || name
                    .chars()
                    .any(|character| character == '\0' || character.is_control())
            {
                errors.push(format!(
                    "emulator.environment key {name:?} must be a nonempty Windows environment name without '=' or control characters"
                ));
            }
            if !environment_names.insert(windows_case_key(name)) {
                errors.push(format!(
                    "emulator.environment key {name:?} duplicates another key on Windows"
                ));
            }
            validate_launch_text(
                &format!("emulator.environment[{name:?}]"),
                value,
                &mut has_rom_token,
                &mut errors,
            );
        }
        if !has_rom_token {
            errors.push(
                "an emulator argument or environment value must contain the {rom} token".to_owned(),
            );
        }

        validate_notices(&self.notices, &mut errors);
        validate_mounts(&self.state.mounts, &self.emulator.entrypoint, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(RecipeError::Validation(errors))
        }
    }
}

/// Expand one launch argument or environment value exactly once.
pub fn expand_launch_value(
    value: &str,
    tokens: &LaunchTokens<'_>,
) -> Result<String, LaunchTokenError> {
    let mut expanded = String::with_capacity(value.len());
    let mut cursor = 0;

    while cursor < value.len() {
        let remainder = &value[cursor..];
        let next_open = remainder.find('{');
        let next_close = remainder.find('}');

        if next_close.is_some_and(|close| next_open.is_none_or(|open| close < open)) {
            return Err(token_error(
                "unmatched closing brace",
                cursor + next_close.unwrap(),
            ));
        }

        let Some(open) = next_open else {
            expanded.push_str(remainder);
            break;
        };

        expanded.push_str(&remainder[..open]);
        let token_start = cursor + open;
        let after_open = token_start + 1;
        let close = value[after_open..]
            .find('}')
            .map(|offset| after_open + offset)
            .ok_or_else(|| token_error("unmatched opening brace", token_start))?;
        let token = &value[after_open..close];
        let replacement = match token {
            "emulator" => tokens.emulator,
            "rom" => tokens.rom,
            "state" => tokens.state,
            "logs" => tokens.logs,
            "instance" => tokens.instance,
            _ => {
                return Err(token_error(
                    &format!("unknown launch token {{{token}}}"),
                    token_start,
                ));
            }
        };
        expanded.push_str(&replacement.to_string_lossy());
        cursor = close + 1;
    }

    Ok(expanded)
}

fn default_working_directory() -> String {
    ".".to_owned()
}

fn validate_game_id(value: &str, errors: &mut Vec<String>) {
    validate_slug("id", value, errors);
    if validate_file_name(value).is_err() {
        errors.push("id must also be a Windows-safe runtime directory name".to_owned());
    }
    if !value.contains('.') {
        errors.push("id must use a stable dotted identifier such as org.example.game".to_owned());
    }
    if value.split('.').any(str::is_empty) {
        errors.push("id must not contain empty dot-separated segments".to_owned());
    }
}

fn validate_slug(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.is_empty() {
        errors.push(format!("{field} must not be empty"));
        return;
    }
    if value.len() > 128 {
        errors.push(format!("{field} must not exceed 128 bytes"));
    }
    if !value.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-' | '_')
    }) {
        errors.push(format!(
            "{field} may contain only lowercase ASCII letters, digits, dots, hyphens, and underscores"
        ));
    }
}

fn validate_text(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{field} must not be empty"));
    }
    if value
        .chars()
        .any(|character| character == '\0' || character.is_control())
    {
        errors.push(format!("{field} must not contain control characters"));
    }
}

fn validate_source(value: &str, errors: &mut Vec<String>) {
    validate_text("emulator.source", value, errors);
    let remainder = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"));
    if remainder.is_none_or(str::is_empty) || value.chars().any(char::is_whitespace) {
        errors.push("emulator.source must be an absolute HTTP(S) provenance URL".to_owned());
    }
}

fn validate_path(field: &str, value: &str, allow_dot: bool, errors: &mut Vec<String>) {
    let result = if allow_dot {
        validate_relative_path_or_dot(value)
    } else {
        validate_relative_path(value)
    };
    if let Err(error) = result {
        errors.push(format!("{field}: {}", error.reason()));
    }
}

fn validate_sha256(field: &str, value: &str, errors: &mut Vec<String>) {
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

fn validate_launch_text(
    field: &str,
    value: &str,
    has_rom_token: &mut bool,
    errors: &mut Vec<String>,
) {
    if value.contains('\0') {
        errors.push(format!("{field} must not contain NUL"));
    }
    if value.contains("{rom}") {
        *has_rom_token = true;
    }

    let empty_path = Path::new("");
    let tokens = LaunchTokens {
        emulator: empty_path,
        rom: empty_path,
        state: empty_path,
        logs: empty_path,
        instance: empty_path,
    };
    if let Err(error) = expand_launch_value(value, &tokens) {
        errors.push(format!("{field}: {error}"));
    }
}

fn validate_notices(notices: &[Notice], errors: &mut Vec<String>) {
    let mut outputs = HashSet::new();
    for (index, notice) in notices.iter().enumerate() {
        validate_path(
            &format!("notices[{index}].source"),
            &notice.source,
            false,
            errors,
        );
        validate_path(
            &format!("notices[{index}].output"),
            &notice.output,
            false,
            errors,
        );
        if !outputs.insert(windows_case_key(&notice.output)) {
            errors.push(format!(
                "notices[{index}].output duplicates another output on Windows"
            ));
        }
    }
}

fn validate_mounts(mounts: &[StateMount], entrypoint: &str, errors: &mut Vec<String>) {
    let mut names = HashSet::new();
    for (index, mount) in mounts.iter().enumerate() {
        if validate_file_name(&mount.name).is_err() {
            errors.push(format!(
                "state.mounts[{index}].name must be a single Windows-safe name"
            ));
        }
        if mount.name.eq_ignore_ascii_case(".initialized") {
            errors.push(format!(
                "state.mounts[{index}].name is reserved by the runtime"
            ));
        }
        if !names.insert(windows_case_key(&mount.name)) {
            errors.push(format!(
                "state.mounts[{index}].name duplicates another mount name on Windows"
            ));
        }
        validate_path(
            &format!("state.mounts[{index}].target"),
            &mount.target,
            false,
            errors,
        );
        if is_same_or_ancestor(&mount.target, entrypoint) {
            errors.push(format!(
                "state.mounts[{index}].target cannot contain the emulator entrypoint"
            ));
        }
    }

    for left in 0..mounts.len() {
        for right in (left + 1)..mounts.len() {
            if is_same_or_ancestor(&mounts[left].target, &mounts[right].target)
                || is_same_or_ancestor(&mounts[right].target, &mounts[left].target)
            {
                errors.push(format!(
                    "state.mounts[{left}].target and state.mounts[{right}].target overlap"
                ));
            }
        }
    }
}

fn token_error(message: &str, byte_offset: usize) -> LaunchTokenError {
    LaunchTokenError {
        message: format!("{message} at byte {byte_offset}"),
    }
}
