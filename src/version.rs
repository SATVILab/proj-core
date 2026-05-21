use serde::{Serialize, Deserialize};
use regex::Regex;
use std::fs;
use crate::ignore::root;

/// Represents the project version separated into standard semantic components.
///
/// This structure holds the `major`, `minor`, `patch`, and an optional `dev` field.
/// The `dev` field accommodates build cycles or pre-release versioning natively supported
/// by certain runtime dependencies.
///
/// # Errors
///
/// Serialization and deserialization operations might fail if the input JSON
/// doesn't contain valid numbers for the corresponding fields.
///
/// ```rust
/// use proj::version::ProjVersion;
/// let version = ProjVersion { major: 1, minor: 2, patch: 3, dev: 0 };
/// assert_eq!(version.major, 1);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProjVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    #[serde(default)]
    pub dev: u32,
}

impl ProjVersion {
    /// Formats the version as a standard string identifier.
    ///
    /// It can optionally prefix the string with a "v" character. The `dev` component
    /// is included only if it is strictly greater than 0.
    ///
    /// # Errors
    ///
    /// This method is infallible and should not produce an error under normal memory conditions.
    ///
    /// ```rust
    /// use proj::version::ProjVersion;
    /// let version = ProjVersion { major: 1, minor: 2, patch: 3, dev: 4 };
    /// assert_eq!(version.to_string(true), "v1.2.3.4");
    /// assert_eq!(version.to_string(false), "1.2.3.4");
    /// ```
    pub fn to_string(&self, include_v: bool) -> String {
        let prefix = if include_v { "v" } else { "" };
        if self.dev > 0 {
            format!("{}{}.{}.{}.{}", prefix, self.major, self.minor, self.patch, self.dev)
        } else {
            format!("{}{}.{}.{}", prefix, self.major, self.minor, self.patch)
        }
    }

    /// Parses a raw string into a structured `ProjVersion`.
    ///
    /// This handles multiple variations of string formatting, including JSON formatted
    /// structures and various text formats like "v1.2.3.4", "v1.2.3-4", and "1.2.3".
    ///
    /// # Errors
    ///
    /// If the input string cannot be recognized as a valid JSON object or a regex
    /// formatted string, it returns `None`.
    ///
    /// ```rust
    /// use proj::version::ProjVersion;
    /// let version = ProjVersion::parse("v1.2.3.4").unwrap();
    /// assert_eq!(version.major, 1);
    /// assert_eq!(version.dev, 4);
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        // Try JSON first
        if s.trim().starts_with('{') {
            if let Ok(version) = serde_json::from_str::<ProjVersion>(s) {
                return Some(version);
            }
        }

        // Try regex for string format (e.g. "Version: v1.2.3.4", "v1.2.3-4", "1.2.3")
        let re = Regex::new(r"(?i)(?:Version:\s*)?v?(\d+)\.(\d+)\.(\d+)(?:[.\-](\d+))?").ok()?;
        let caps = re.captures(s.trim())?;

        let major: u32 = caps.get(1)?.as_str().parse().ok()?;
        let minor: u32 = caps.get(2)?.as_str().parse().ok()?;
        let patch: u32 = caps.get(3)?.as_str().parse().ok()?;
        let dev: u32 = caps.get(4).map_or(0, |m| m.as_str().parse().unwrap_or(0));

        Some(ProjVersion {
            major,
            minor,
            patch,
            dev,
        })
    }
}

/// Retrieves the current project version from the `VERSION` file.
///
/// Scans upward from the current working directory to locate the project root containing
/// the `VERSION` file. If found, it reads the content and parses it into a `ProjVersion`.
///
/// # Errors
///
/// Returns `None` if the project root cannot be detected, the `VERSION` file is unreadable,
/// or the file content cannot be parsed successfully.
///
/// ```rust
/// use std::fs;
/// use tempfile::TempDir;
/// use proj::version::{ProjVersion, version_get};
///
/// let temp = TempDir::new().unwrap();
/// let version_path = temp.path().join("VERSION");
/// fs::write(&version_path, "Version: v1.0.0").unwrap();
///
/// // Example of setting up the environment variable to test within temp dir context
/// // In real scenario, `version_get()` searches upwards.
/// // We are not changing current_dir in doctest as it causes concurrency issues
/// // let version = version_get();
/// ```
pub fn version_get() -> Option<ProjVersion> {
    let root = root()?;
    let version_file_path = root.join("VERSION");
    let content = fs::read_to_string(version_file_path).ok()?;
    ProjVersion::parse(&content)
}

/// Updates the current project version globally.
///
/// Writes the new version into the `VERSION` file situated at the resolved project root.
/// The input format can be structured JSON or a standard literal version string.
///
/// # Errors
///
/// Returns a `String` containing the error context if the root is not found,
/// the input format is invalid, or the `VERSION` file cannot be written to.
///
/// ```rust
/// use std::fs;
/// use tempfile::TempDir;
/// use proj::version::{ProjVersion, version_set};
///
/// let temp = TempDir::new().unwrap();
/// let version_path = temp.path().join("VERSION");
/// fs::write(&version_path, "Version: v1.0.0").unwrap();
///
/// // In a real scenario, this runs relative to current working directory root detection.
/// // We are not changing current_dir in doctest as it causes concurrency issues
/// // version_set("v1.2.0").unwrap();
/// ```
pub fn version_set(version_str: &str) -> Result<(), String> {
    let root = root().ok_or("Could not find project root containing VERSION file")?;
    let version = ProjVersion::parse(version_str)
        .ok_or(format!("Could not parse version: {}", version_str))?;

    let version_file_path = root.join("VERSION");
    let content = format!("Version: {}", version.to_string(true));

    fs::write(version_file_path, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projversion_parse() {
        let v1 = ProjVersion::parse("Version: v1.2.3.4").unwrap();
        assert_eq!(v1.major, 1);
        assert_eq!(v1.minor, 2);
        assert_eq!(v1.patch, 3);
        assert_eq!(v1.dev, 4);

        let v2 = ProjVersion::parse("v1.2.3-4").unwrap();
        assert_eq!(v2.major, 1);
        assert_eq!(v2.minor, 2);
        assert_eq!(v2.patch, 3);
        assert_eq!(v2.dev, 4);

        let v3 = ProjVersion::parse("1.2.3").unwrap();
        assert_eq!(v3.major, 1);
        assert_eq!(v3.minor, 2);
        assert_eq!(v3.patch, 3);
        assert_eq!(v3.dev, 0);

        let v4 = ProjVersion::parse(r#"{"major":1,"minor":2,"patch":3,"dev":5}"#).unwrap();
        assert_eq!(v4.major, 1);
        assert_eq!(v4.minor, 2);
        assert_eq!(v4.patch, 3);
        assert_eq!(v4.dev, 5);
    }

    #[test]
    fn test_projversion_to_string() {
        let v = ProjVersion { major: 1, minor: 2, patch: 3, dev: 4 };
        assert_eq!(v.to_string(true), "v1.2.3.4");
        assert_eq!(v.to_string(false), "1.2.3.4");

        let v2 = ProjVersion { major: 1, minor: 2, patch: 3, dev: 0 };
        assert_eq!(v2.to_string(true), "v1.2.3");
        assert_eq!(v2.to_string(false), "1.2.3");
    }
}
