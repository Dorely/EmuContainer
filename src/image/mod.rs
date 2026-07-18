mod build;
mod model;
mod read;

use std::{error::Error, fmt, fmt::Write as _, path::PathBuf};

pub use build::{BuildRequest, BuildResult, build_image, sha256_file};
pub use model::{
    CONTENT_ROOT, EMULATOR_ROOT, EntryKind, GameMetadata, IMAGE_FORMAT_VERSION,
    IMAGE_METADATA_PATH, ImageContent, ImageEmulator, ImageManifest, ImageNotice, ImageState,
    ImageValidationError, InventoryEntry, Isolation, MAX_ARCHIVE_ENTRIES, MAX_IMAGE_METADATA_BYTES,
    MAX_UNCOMPRESSED_BYTES, MINIMUM_RUNTIME_VERSION, NOTICES_ROOT,
};
pub use read::{VerifiedImage, extract_image, verify_image};

use crate::{path_policy::PathPolicyError, recipe::RecipeError};

#[derive(Debug)]
pub enum ImageError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    Recipe(RecipeError),
    Path {
        context: String,
        source: PathPolicyError,
    },
    Json(serde_json::Error),
    Zip(zip::result::ZipError),
    Validation(ImageValidationError),
    Invalid(String),
    OutputExists(PathBuf),
    DigestMismatch {
        subject: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for ImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "failed to {operation} {}: {source}",
                path.display()
            ),
            Self::Recipe(source) => source.fmt(formatter),
            Self::Path { context, source } => write!(formatter, "{context}: {source}"),
            Self::Json(source) => write!(formatter, "invalid image metadata JSON: {source}"),
            Self::Zip(source) => write!(formatter, "invalid image archive: {source}"),
            Self::Validation(source) => source.fmt(formatter),
            Self::Invalid(message) => formatter.write_str(message),
            Self::OutputExists(path) => write!(
                formatter,
                "output {} already exists; pass --force to replace it",
                path.display()
            ),
            Self::DigestMismatch {
                subject,
                expected,
                actual,
            } => write!(
                formatter,
                "{subject} SHA-256 mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for ImageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Recipe(source) => Some(source),
            Self::Path { source, .. } => Some(source),
            Self::Json(source) => Some(source),
            Self::Zip(source) => Some(source),
            Self::Validation(source) => Some(source),
            Self::Invalid(_) | Self::OutputExists(_) | Self::DigestMismatch { .. } => None,
        }
    }
}

impl From<RecipeError> for ImageError {
    fn from(source: RecipeError) -> Self {
        Self::Recipe(source)
    }
}

impl From<serde_json::Error> for ImageError {
    fn from(source: serde_json::Error) -> Self {
        Self::Json(source)
    }
}

impl From<zip::result::ZipError> for ImageError {
    fn from(source: zip::result::ZipError) -> Self {
        Self::Zip(source)
    }
}

impl From<ImageValidationError> for ImageError {
    fn from(source: ImageValidationError) -> Self {
        Self::Validation(source)
    }
}

pub(crate) fn io_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: std::io::Error,
) -> ImageError {
    ImageError::Io {
        operation,
        path: path.into(),
        source,
    }
}

pub(crate) fn path_error(context: impl Into<String>, source: PathPolicyError) -> ImageError {
    ImageError::Path {
        context: context.into(),
        source,
    }
}

pub(crate) fn digest_string(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}
