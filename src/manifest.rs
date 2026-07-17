use std::{
    collections::HashSet,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub system: String,
    pub targets: Vec<Target>,
    pub runner: Runner,
    pub content: Content,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub presentation: Presentation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub enum Target {
    #[serde(rename = "linux-x86_64")]
    LinuxX86_64,
    #[serde(rename = "windows-x86_64")]
    WindowsX86_64,
}

impl fmt::Display for Target {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LinuxX86_64 => formatter.write_str("linux-x86_64"),
            Self::WindowsX86_64 => formatter.write_str("windows-x86_64"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runner {
    pub kind: RunnerKind,
    pub frontend: String,
    pub core: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum RunnerKind {
    #[serde(rename = "libretro")]
    Libretro,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub mode: ContentMode,
    pub path: Option<String>,
    pub filename: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ContentMode {
    Contentless,
    Bundled,
    UserSupplied,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Permissions {
    pub display: bool,
    pub audio: bool,
    pub gpu: bool,
    pub input: bool,
    pub network: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Presentation {
    pub fullscreen: bool,
}

#[derive(Debug)]
pub enum ManifestError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse(toml::de::Error),
    Validation(Vec<String>),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::Parse(source) => write!(formatter, "invalid TOML: {source}"),
            Self::Validation(errors) => {
                writeln!(formatter, "manifest validation failed:")?;
                for error in errors {
                    writeln!(formatter, "- {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl Error for ManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse(source) => Some(source),
            Self::Validation(_) => None,
        }
    }
}

impl Manifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        Self::parse(&source)
    }

    pub fn parse(source: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(source).map_err(ManifestError::Parse)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        let mut errors = Vec::new();

        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            errors.push(format!(
                "schema_version must be {SUPPORTED_SCHEMA_VERSION}, got {}",
                self.schema_version
            ));
        }

        validate_id(&self.id, &mut errors);
        validate_nonempty("title", &self.title, &mut errors);
        validate_nonempty("system", &self.system, &mut errors);
        validate_nonempty("runner.frontend", &self.runner.frontend, &mut errors);
        validate_nonempty("runner.core", &self.runner.core, &mut errors);

        if self.targets.is_empty() {
            errors.push("targets must contain at least one build target".to_owned());
        }

        let unique_targets = self.targets.iter().copied().collect::<HashSet<_>>();
        if unique_targets.len() != self.targets.len() {
            errors.push("targets must not contain duplicates".to_owned());
        }

        validate_content(&self.content, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ManifestError::Validation(errors))
        }
    }
}

fn validate_id(id: &str, errors: &mut Vec<String>) {
    if id.is_empty() {
        errors.push("id must not be empty".to_owned());
        return;
    }

    if !id.contains('.') {
        errors.push("id must use a stable dotted identifier such as org.example.game".to_owned());
    }

    if !id.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-' | '_')
    }) {
        errors.push(
            "id may contain only lowercase ASCII letters, digits, dots, hyphens, and underscores"
                .to_owned(),
        );
    }

    if id.split('.').any(str::is_empty) {
        errors.push("id must not contain empty dot-separated segments".to_owned());
    }
}

fn validate_nonempty(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{field} must not be empty"));
    }
}

fn validate_content(content: &Content, errors: &mut Vec<String>) {
    match content.mode {
        ContentMode::Contentless => {
            reject_field("content.path", &content.path, errors);
            reject_field("content.filename", &content.filename, errors);
            reject_field("content.sha256", &content.sha256, errors);
        }
        ContentMode::Bundled => {
            reject_field("content.filename", &content.filename, errors);
            match content.path.as_deref() {
                Some(path) => validate_portable_relative_path("content.path", path, errors),
                None => errors.push("content.path is required for bundled content".to_owned()),
            }
            match content.sha256.as_deref() {
                Some(hash) => validate_sha256(hash, errors),
                None => errors.push("content.sha256 is required for bundled content".to_owned()),
            }
        }
        ContentMode::UserSupplied => {
            reject_field("content.path", &content.path, errors);
            match content.filename.as_deref() {
                Some(filename) if filename.trim().is_empty() => {
                    errors.push("content.filename must not be empty".to_owned());
                }
                Some(filename) if filename.contains(['/', '\\']) => {
                    errors.push("content.filename must be a file name, not a path".to_owned());
                }
                Some(_) => {}
                None => {
                    errors
                        .push("content.filename is required for user-supplied content".to_owned());
                }
            }
            if let Some(hash) = content.sha256.as_deref() {
                validate_sha256(hash, errors);
            }
        }
    }
}

fn reject_field(field: &str, value: &Option<String>, errors: &mut Vec<String>) {
    if value.is_some() {
        errors.push(format!("{field} is not valid for this content mode"));
    }
}

fn validate_portable_relative_path(field: &str, path: &str, errors: &mut Vec<String>) {
    let has_drive_prefix = path.as_bytes().get(1) == Some(&b':');
    if path.is_empty()
        || path.starts_with('/')
        || has_drive_prefix
        || path.contains('\\')
        || path.split('/').any(|segment| segment == "..")
    {
        errors.push(format!(
            "{field} must be a portable, forward-slash-separated relative path"
        ));
    }
}

fn validate_sha256(hash: &str, errors: &mut Vec<String>) {
    if hash.len() != 64 || !hash.chars().all(|character| character.is_ascii_hexdigit()) {
        errors.push("content.sha256 must contain exactly 64 hexadecimal characters".to_owned());
    }
}
