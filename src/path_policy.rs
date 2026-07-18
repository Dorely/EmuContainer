use std::{
    error::Error,
    fmt,
    path::{Component, Path, PathBuf},
};

/// A validation failure for a path that will be interpreted on Windows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathPolicyError {
    value: String,
    reason: String,
}

impl PathPolicyError {
    fn new(value: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            reason: reason.into(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for PathPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid portable path {:?}: {}",
            self.value, self.reason
        )
    }
}

impl Error for PathPolicyError {}

/// Validate a forward-slash-separated relative path using rules that are safe
/// on Windows even when validation happens on another host.
pub fn validate_relative_path(value: &str) -> Result<(), PathPolicyError> {
    if value.is_empty() {
        return Err(PathPolicyError::new(value, "the path is empty"));
    }
    if value == "." {
        return Err(PathPolicyError::new(
            value,
            "the payload root is not valid in this field",
        ));
    }
    if value.starts_with('/') {
        return Err(PathPolicyError::new(
            value,
            "absolute paths are not allowed",
        ));
    }
    if value.contains('\\') {
        return Err(PathPolicyError::new(
            value,
            "use forward slashes as separators",
        ));
    }
    if value.encode_utf16().count() > 32_767 {
        return Err(PathPolicyError::new(
            value,
            "the path exceeds the Windows extended path limit",
        ));
    }

    for segment in value.split('/') {
        validate_segment(value, segment)?;
    }

    Ok(())
}

/// Validate a working-directory path, for which `.` names the emulator root.
pub fn validate_relative_path_or_dot(value: &str) -> Result<(), PathPolicyError> {
    if value == "." {
        Ok(())
    } else {
        validate_relative_path(value)
    }
}

/// Validate a single Windows-safe file name.
pub fn validate_file_name(value: &str) -> Result<(), PathPolicyError> {
    validate_relative_path(value)?;
    if value.contains('/') {
        return Err(PathPolicyError::new(
            value,
            "a file name cannot contain path separators",
        ));
    }
    Ok(())
}

/// Validate an archive entry. Directory entries must end in `/`; file entries
/// must not. The returned path omits a directory's trailing slash.
pub fn validate_archive_path(value: &str, is_directory: bool) -> Result<&str, PathPolicyError> {
    let path = if is_directory {
        value.strip_suffix('/').ok_or_else(|| {
            PathPolicyError::new(value, "directory archive entries must end in a slash")
        })?
    } else {
        if value.ends_with('/') {
            return Err(PathPolicyError::new(
                value,
                "file archive entries cannot end in a slash",
            ));
        }
        value
    };
    validate_relative_path(path)?;
    Ok(path)
}

/// Produce the comparison key used by Windows for collision checks.
pub fn windows_case_key(value: &str) -> String {
    value.to_lowercase()
}

/// Return true when `ancestor` is the same portable path as `path`, or is an
/// ancestor of it, using case-insensitive Windows comparisons.
pub fn is_same_or_ancestor(ancestor: &str, path: &str) -> bool {
    let ancestor = windows_case_key(ancestor).trim_end_matches('/').to_owned();
    let path = windows_case_key(path).trim_end_matches('/').to_owned();

    path == ancestor
        || path
            .strip_prefix(&ancestor)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

/// Convert a validated portable path to a host path rooted below `base`.
pub fn join_portable(base: &Path, portable: &str) -> Result<PathBuf, PathPolicyError> {
    validate_relative_path(portable)?;
    let mut result = base.to_path_buf();
    result.extend(portable.split('/'));
    Ok(result)
}

/// Convert a host path below `root` into a validated portable path.
pub fn to_portable_path(root: &Path, path: &Path) -> Result<String, PathPolicyError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        PathPolicyError::new(
            path.to_string_lossy(),
            format!("the path is not below {}", root.display()),
        )
    })?;

    let mut segments = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(segment) => {
                let segment = segment.to_str().ok_or_else(|| {
                    PathPolicyError::new(path.to_string_lossy(), "the path contains non-UTF-8 text")
                })?;
                segments.push(segment);
            }
            _ => {
                return Err(PathPolicyError::new(
                    path.to_string_lossy(),
                    "the path contains a non-normal component",
                ));
            }
        }
    }

    let portable = segments.join("/");
    validate_relative_path(&portable)?;
    Ok(portable)
}

/// Detect links and other Windows reparse points without following them.
pub fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn validate_segment(full_path: &str, segment: &str) -> Result<(), PathPolicyError> {
    if segment.is_empty() {
        return Err(PathPolicyError::new(
            full_path,
            "empty path segments are not allowed",
        ));
    }
    if matches!(segment, "." | "..") {
        return Err(PathPolicyError::new(
            full_path,
            "dot and parent segments are not allowed",
        ));
    }
    if segment.contains(':') {
        return Err(PathPolicyError::new(
            full_path,
            "colons and alternate data streams are not allowed",
        ));
    }
    if segment
        .chars()
        .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return Err(PathPolicyError::new(
            full_path,
            "characters forbidden in Windows file names are not allowed",
        ));
    }
    if segment
        .chars()
        .any(|character| character == '\0' || character.is_control())
    {
        return Err(PathPolicyError::new(
            full_path,
            "control characters are not allowed",
        ));
    }
    if segment.ends_with(['.', ' ']) {
        return Err(PathPolicyError::new(
            full_path,
            "segments cannot end in a dot or space on Windows",
        ));
    }
    if segment.encode_utf16().count() > 255 {
        return Err(PathPolicyError::new(
            full_path,
            "a path segment exceeds 255 UTF-16 code units",
        ));
    }
    if is_reserved_windows_name(segment) {
        return Err(PathPolicyError::new(
            full_path,
            format!("{segment:?} is a reserved Windows device name"),
        ));
    }

    Ok(())
}

fn is_reserved_windows_name(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or(segment).to_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }

    let mut characters = stem.chars();
    let prefix = characters.by_ref().take(3).collect::<String>();
    let suffix = characters.collect::<String>();
    matches!(prefix.as_str(), "COM" | "LPT")
        && matches!(
            suffix.as_str(),
            "1" | "2"
                | "3"
                | "4"
                | "5"
                | "6"
                | "7"
                | "8"
                | "9"
                | "\u{00b9}"
                | "\u{00b2}"
                | "\u{00b3}"
        )
}
