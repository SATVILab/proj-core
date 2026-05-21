use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use crate::ignore::{root, update_ignores};

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

pub fn yml_get() -> String {
    "projr yml content".to_string() // Left for backwards compatibility, though to be replaced/removed as per plan.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yml_get() {
        assert_eq!(yml_get(), "projr yml content");
    }
}
