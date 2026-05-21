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

use std::collections::HashMap;

#[derive(Deserialize, Debug, Default, Clone)]
pub struct ProjConfig {
    #[serde(default)]
    pub directories: HashMap<String, DirConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DirConfig {
    pub path: Option<PathBuf>,
    #[serde(default = "default_ignore")]
    pub ignore: IgnoreConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum IgnoreConfig {
    Single(String),
    Multiple(Vec<String>),
}

fn default_ignore() -> IgnoreConfig {
    IgnoreConfig::Single("all".to_string())
}

pub struct ValidatedConfig {
    pub directories: HashMap<String, ResolvedDir>,
}

pub struct ResolvedDir {
    pub path: PathBuf,
    pub ignore: IgnoreConfig,
}

impl ProjConfig {
    pub fn validate_and_resolve(&self) -> Result<ValidatedConfig, String> {
        let mut resolved = HashMap::new();

        // 1. Process explicit user configurations
        for (label, config) in &self.directories {
            let lower_label = label.to_lowercase();

            // Validate prefix rules
            if !lower_label.starts_with("cache")
                && !lower_label.starts_with("raw")
                && !lower_label.starts_with("output")
                && !lower_label.starts_with("docs")
            {
                return Err(format!(
                    "Invalid directory label '{}' found in _proj.yml. \
                    All custom labels must begin with 'raw', 'output', 'docs', or 'cache'.",
                    label
                ));
            }

            // Fallback to defaults if 'path' key is omitted under a validated label
            let path = match &config.path {
                Some(p) => p.clone(),
                None => match () {
                    _ if lower_label.starts_with("cache") => PathBuf::from("_tmp"),
                    _ if lower_label.starts_with("raw") => PathBuf::from("_raw"),
                    _ if lower_label.starts_with("output") => PathBuf::from("_output"),
                    _ if lower_label.starts_with("docs") => PathBuf::from("docs"),
                    _ => unreachable!(),
                }
            };

            resolved.insert(label.clone(), ResolvedDir {
                path,
                ignore: config.ignore.clone(),
            });
        }

        // 2. Inject missing default base targets if completely omitted from the YAML file
        let base_defaults = [
            ("cache", "_tmp"),
            ("raw", "_raw"),
            ("output", "_output"),
            ("docs", "docs"),
        ];

        for (base_key, default_path) in base_defaults {
            // Check if any existing key satisfies this base (case-insensitive check)
            let exists = resolved.keys().any(|k| k.to_lowercase() == base_key);
            if !exists {
                resolved.insert(base_key.to_string(), ResolvedDir {
                    path: PathBuf::from(default_path),
                    ignore: default_ignore(),
                });
            }
        }

        Ok(ValidatedConfig { directories: resolved })
    }
}

impl ValidatedConfig {
    pub fn get_path(&self, project_root: &std::path::Path, label: &str) -> Result<PathBuf, String> {
        // Rule A: Check for an exact matching key in the map
        if let Some(dir) = self.directories.get(label) {
            return Ok(make_absolute(project_root, &dir.path));
        }

        // Rule B: Dynamic prefix parsing fallback for unlisted sub-labels
        let lower_label = label.to_lowercase();
        let base_prefix = if lower_label.starts_with("cache") { Some("cache") }
            else if lower_label.starts_with("raw") { Some("raw") }
            else if lower_label.starts_with("output") { Some("output") }
            else if lower_label.starts_with("docs") { Some("docs") }
            else { None };

        if let Some(prefix) = base_prefix {
            // Find the base directory configuration (which is guaranteed to exist due to our injector)
            // Perform case-insensitive lookup to avoid panic if base was provided in different casing
            let base_dir = &self.directories
                .iter()
                .find(|(k, _)| k.to_lowercase() == prefix)
                .unwrap().1.path;

            // Extract the suffix part
            let suffix = if label.len() > prefix.len() {
                if label.as_bytes()[prefix.len()] == b'-' {
                    &label[prefix.len() + 1..]
                } else {
                    &label[prefix.len()..]
                }
            } else {
                ""
            };

            let resolved_path = if suffix.is_empty() {
                base_dir.clone()
            } else {
                base_dir.join(suffix)
            };

            Ok(make_absolute(project_root, &resolved_path))
        } else {
            Err(format!("Requested label '{}' does not match any valid structural prefix.", label))
        }
    }
}

fn make_absolute(root: &std::path::Path, path: &std::path::Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
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

pub fn yml_read() -> Result<ValidatedConfig, String> {
    let project_root = root().ok_or("Could not find project root")?;
    let yml_path = project_root.join("_proj.yml");

    let config: ProjConfig = if yml_path.exists() {
        let content = fs::read_to_string(&yml_path).map_err(|e| e.to_string())?;
        serde_yaml::from_str(&content).map_err(|e| e.to_string())?
    } else {
        ProjConfig::default()
    };

    let validated = config.validate_and_resolve()?;

    // Execute the Ignore Demarcation Engine
    update_ignores(&project_root, &validated)?;

    Ok(validated)
}

fn update_ignores(project_root: &std::path::Path, validated: &ValidatedConfig) -> Result<(), String> {
    let mut git_ignores = Vec::new();
    let mut rbuild_ignores = Vec::new();

    for (_, dir) in &validated.directories {
        // Collect ignores based on the ignore config
        let (should_git, should_rbuild) = match &dir.ignore {
            IgnoreConfig::Single(s) => {
                let s = s.to_lowercase();
                (s == "all" || s == "git", s == "all" || s == "rbuild")
            }
            IgnoreConfig::Multiple(vec) => {
                let lower_vec: Vec<String> = vec.iter().map(|s| s.to_lowercase()).collect();
                (lower_vec.contains(&"all".to_string()) || lower_vec.contains(&"git".to_string()),
                 lower_vec.contains(&"all".to_string()) || lower_vec.contains(&"rbuild".to_string()))
            }
        };

        if should_git || should_rbuild {
            let path_str_opt = if dir.path.is_absolute() {
                // Only process if it is within the project root
                if let Ok(rel_path) = dir.path.strip_prefix(project_root) {
                    Some(rel_path.to_string_lossy().to_string())
                } else {
                    None
                }
            } else {
                Some(dir.path.to_string_lossy().to_string())
            };

            if let Some(path_str) = path_str_opt {
                if should_git {
                    let git_pattern = if path_str.ends_with("/**") {
                        path_str.clone()
                    } else {
                        format!("{}/**", path_str)
                    };
                    git_ignores.push(git_pattern);
                }
                if should_rbuild {
                    let rbuild_pattern = format!("^{}/$", path_str.trim_end_matches('/'));
                    rbuild_ignores.push(rbuild_pattern);
                    let rbuild_pattern_2 = format!("^{}$", path_str.trim_end_matches('/'));
                    rbuild_ignores.push(rbuild_pattern_2);
                }
            }
        }
    }

    // Sort and deduplicate
    git_ignores.sort();
    git_ignores.dedup();
    rbuild_ignores.sort();
    rbuild_ignores.dedup();

    let gitignore_path = project_root.join(".gitignore");
    update_ignore_file(&gitignore_path, &git_ignores)?;

    let rbuildignore_path = project_root.join(".Rbuildignore");
    update_ignore_file(&rbuildignore_path, &rbuild_ignores)?;

    Ok(())
}

fn update_ignore_file(path: &std::path::Path, ignores: &[String]) -> Result<(), String> {
    let start_marker = "# --- PROJ MANAGED ---";
    let end_marker = "# --- END PROJ MANAGED ---";

    let content_lines: Vec<String> = if path.exists() {
        fs::read_to_string(path)
            .map_err(|e| e.to_string())?
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        if ignores.is_empty() {
            return Ok(()); // Don't create file if nothing to ignore
        }
        Vec::new()
    };

    let start_idx = content_lines.iter().position(|line| line == start_marker);
    let end_idx = content_lines.iter().position(|line| line == end_marker);

    let mut new_lines = Vec::new();

    match (start_idx, end_idx) {
        (Some(start), Some(end)) if start < end => {
            // Found block, replace it
            new_lines.extend_from_slice(&content_lines[..=start]);
            for ignore in ignores {
                new_lines.push(ignore.clone());
            }
            new_lines.extend_from_slice(&content_lines[end..]);
        }
        _ => {
            // Block not found or malformed, append to end
            new_lines.extend(content_lines);
            if !new_lines.is_empty() && new_lines.last().unwrap() != "" {
                new_lines.push("".to_string());
            }
            new_lines.push(start_marker.to_string());
            for ignore in ignores {
                new_lines.push(ignore.clone());
            }
            new_lines.push(end_marker.to_string());
        }
    }

    let mut final_content = new_lines.join("\n");
    if !final_content.ends_with('\n') {
        final_content.push('\n');
    }

    // Only write if the content actually changed
    if path.exists() {
        if let Ok(existing_content) = fs::read_to_string(path) {
            if existing_content == final_content {
                return Ok(());
            }
        }
    }

    fs::write(path, final_content).map_err(|e| e.to_string())
}

pub fn yml_get() -> String {
    "projr yml content".to_string() // Left for backwards compatibility, though to be replaced/removed as per plan.
}

/// Finds the project root by searching upwards for a "VERSION" file.
/// When it's found, if there isn't a _proj.yml or .git file/directory in the same directory,
/// then it moves up until it finds one, possibly up to five levels up.
/// If it finds a directory that has a VERSION file and a .git or _proj.yml file,
/// then it takes that one, otherwise it takes the original one.
pub fn root() -> Option<PathBuf> {
    let current_dir = env::current_dir().ok()?;
    let mut current_path = current_dir.as_path();

    let first_version_dir;

    loop {
        if current_path.join("VERSION").exists() {
            first_version_dir = Some(current_path.to_path_buf());
            break;
        }
        match current_path.parent() {
            Some(parent) => current_path = parent,
            None => return None,
        }
    }

    let first_dir = first_version_dir?;

    // Check if the first directory also has _proj.yml or .git
    if first_dir.join("_proj.yml").exists() || first_dir.join(".git").exists() {
        return Some(first_dir);
    }

    // Move up to 5 levels to find a directory with VERSION and (_proj.yml or .git)
    let mut search_path = first_dir.as_path();
    for _ in 0..5 {
        match search_path.parent() {
            Some(parent) => search_path = parent,
            None => break,
        }

        if search_path.join("VERSION").exists() && (search_path.join("_proj.yml").exists() || search_path.join(".git").exists()) {
            return Some(search_path.to_path_buf());
        }
    }

    // Fallback to the first directory
    Some(first_dir)
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
