use std::env;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use regex::Regex;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProjVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    #[serde(default)]
    pub dev: u32,
}

impl ProjVersion {
    pub fn to_string(&self, include_v: bool) -> String {
        let prefix = if include_v { "v" } else { "" };
        if self.dev > 0 {
            format!("{}{}.{}.{}.{}", prefix, self.major, self.minor, self.patch, self.dev)
        } else {
            format!("{}{}.{}.{}", prefix, self.major, self.minor, self.patch)
        }
    }

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

use std::fs;

pub fn version_get() -> Option<ProjVersion> {
    let root = root()?;
    let version_file_path = root.join("VERSION");
    let content = fs::read_to_string(version_file_path).ok()?;
    ProjVersion::parse(&content)
}

pub fn version_set(version_str: &str) -> Result<(), String> {
    let root = root().ok_or("Could not find project root containing VERSION file")?;
    let version = ProjVersion::parse(version_str)
        .ok_or(format!("Could not parse version: {}", version_str))?;

    let version_file_path = root.join("VERSION");
    let content = format!("Version: {}", version.to_string(true));

    fs::write(version_file_path, content).map_err(|e| e.to_string())
}

pub fn yml_get() -> String {
    "projr yml content".to_string()
}

/// Finds the project root by searching upwards for a "VERSION" file.
pub fn root() -> Option<PathBuf> {
    let current_dir = env::current_dir().ok()?;
    let mut current_path = current_dir.as_path();

    loop {
        if current_path.join("VERSION").exists() {
            return Some(current_path.to_path_buf());
        }
        match current_path.parent() {
            Some(parent) => current_path = parent,
            None => break,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yml_get() {
        assert_eq!(yml_get(), "projr yml content");
    }

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

    #[test]
    fn test_root_detection() {
        // Find existing root (which contains the actual project VERSION file for proj)
        assert!(root().is_some());
    }
}
