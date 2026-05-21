use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use crate::ignore::{root, update_ignores};

/// Represents the complete structure of a `_proj.yml` configuration file.
///
/// Contains dynamically user-defined directory labels mapping to specific system paths.
/// These paths adhere to required prefixes like `raw`, `output`, `cache`, or `docs`.
///
/// # Errors
///
/// Deserialization will fail if the provided YAML is structurally invalid or map
/// types are mismatched.
///
/// ```rust
/// use proj::yml::ProjConfig;
/// let config = ProjConfig::default();
/// assert!(config.directories.is_empty());
/// ```

#[derive(Deserialize, Debug, Default, Clone)]
pub struct RemotesConfig {
    pub local: Option<HashMap<String, LocalRemoteConfig>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageStructure {
    Cas,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InspectStrategy {
    Manifest,
    File,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LocalRemoteConfig {
    pub path: PathBuf,
    pub structure: StorageStructure,
    pub content: Vec<String>,
    pub inspect: InspectStrategy,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct ProjConfig {
    #[serde(default)]
    pub remotes: RemotesConfig,
    #[serde(default)]
    pub config: GlobalConfig,
    #[serde(default)]
    pub directories: HashMap<String, DirConfig>,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub dev: DevConfig,
    /// Captures arbitrary parameters maps under any matching alias keys.
    /// Evaluates aliases in order: "parameters", "parameter", "params", "param".
    #[serde(alias = "parameter", alias = "params", alias = "param", default = "default_parameters")]
    pub parameters: serde_yaml::Value,
}

fn default_parameters() -> serde_yaml::Value {
    serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitEngine {
    Auto,
    System,
}

/// Represents the top-level configuration key `config` in `_proj.yml`.
#[derive(Deserialize, Debug, Default, Clone)]
pub struct GlobalConfig {
    #[serde(default)]
    pub git: GeneralGitConfig,
}

fn default_git_engine() -> GitEngine {
    GitEngine::Auto
}

/// Represents general git settings inside `config.git`.
#[derive(Deserialize, Debug, Clone)]
pub struct GeneralGitConfig {
    #[serde(default = "default_use_proj_cred_helper")]
    pub use_proj_cred_helper: bool,
    #[serde(default = "default_git_engine")]
    pub engine: GitEngine,
}

impl Default for GeneralGitConfig {
    fn default() -> Self {
        Self {
            use_proj_cred_helper: true,
            engine: GitEngine::Auto,
        }
    }
}

fn default_use_proj_cred_helper() -> bool {
    true
}

/// Represents the build configuration options within `_proj.yml`.
#[derive(Deserialize, Debug, Default, Clone)]
pub struct BuildConfig {
    pub dest: Option<Vec<String>>,
    pub scripts: Option<Vec<String>>,
    #[serde(default)]
    pub hooks: HooksConfig,
    pub profile: Option<String>,
    #[serde(default)]
    pub git: GitConfigOpt,
    #[serde(default)]
    pub restrictions: RestrictionsConfig,
    pub clear_output: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq)]
pub struct HooksConfig {
    pub pre: Option<Vec<String>>,
    pub post: Option<Vec<String>>,
    pub both: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RestrictionsConfig {
    pub only_branches: Option<Vec<String>>,
    pub not_branches: Option<Vec<String>>,
    pub not_behind: Option<bool>,
}

impl Default for RestrictionsConfig {
    fn default() -> Self {
        RestrictionsConfig {
            only_branches: None,
            not_branches: None,
            not_behind: None, // Omitted means implicitly true
        }
    }
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq)]
pub struct DevConfig {
    pub scripts: Option<Vec<String>>,
    pub hooks: Option<HooksConfig>,
    pub old_dev_remove: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum GitConfigOpt {
    Boolean(bool),
    Detailed(GitConfig),
}

impl Default for GitConfigOpt {
    fn default() -> Self {
        // By default, acts as a detailed block with empty entries to evaluate run-time fallbacks
        GitConfigOpt::Detailed(GitConfig::default())
    }
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq)]
pub struct GitConfig {
    pub commit: Option<bool>,
    pub push: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGitConfig {
    pub commit: bool,
    pub push: bool,
}

/// Represents the configuration for a single directory entry within `_proj.yml`.
///
/// Each custom label specifies an optional specific physical path and a set of
/// ignore rules to be synced automatically with `.gitignore` and `.Rbuildignore`.
///
/// # Errors
///
/// Fails to parse if the `path` key holds a non-string format.
///
/// ```rust
/// use proj::yml::{DirConfig, IgnoreConfig};
/// let dir = DirConfig { path: None, ignore: IgnoreConfig::Single("all".to_string()) };
/// ```
#[derive(Deserialize, Debug, Clone)]
pub struct DirConfig {
    pub path: Option<PathBuf>,
    #[serde(default = "default_ignore")]
    pub ignore: IgnoreConfig,
}

/// Defines auto-generated ignore rules applied to tracked directories.
///
/// Can either be a single string rule or a list of multiple target platforms
/// (e.g. `["git", "rbuild"]`).
///
/// # Errors
///
/// Standard fallback occurs dynamically if untagged keys don't match list or string.
///
/// ```rust
/// use proj::yml::IgnoreConfig;
/// let config1 = IgnoreConfig::Single("all".to_string());
/// let config2 = IgnoreConfig::Multiple(vec!["git".to_string(), "rbuild".to_string()]);
/// ```
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum IgnoreConfig {
    Single(String),
    Multiple(Vec<String>),
}

fn default_ignore() -> IgnoreConfig {
    IgnoreConfig::Single("all".to_string())
}

/// The fully resolved and validated project configuration.
///
/// Once parsed and checked, this structure guarantees that all referenced directories
/// conform to naming rules, and default unreferenced base targets are injected.
///
/// # Panics
///
/// None expected as this acts purely as an internal verified state container.
///
/// ```rust
/// use std::collections::HashMap;
/// use proj::yml::{ValidatedConfig, ResolvedGitConfig, RestrictionsConfig, GlobalConfig};
/// let config = ValidatedConfig { remotes: Default::default(), dest: vec![],
///     config: GlobalConfig::default(), 
///     directories: HashMap::new(), 
///     git: ResolvedGitConfig { commit: false, push: false }, 
///     restrictions: RestrictionsConfig::default(), 
///     clear_output: None, 
///     old_dev_remove: None,
///     parameters: Default::default()
/// };
/// ```
#[derive(Debug)]
pub struct ValidatedConfig {
    pub remotes: RemotesConfig,
    pub dest: Vec<String>,
    pub config: GlobalConfig,
    pub directories: HashMap<String, ResolvedDir>,
    pub git: ResolvedGitConfig,
    pub restrictions: RestrictionsConfig,
    pub clear_output: Option<String>,
    pub old_dev_remove: Option<bool>,
    pub parameters: serde_yaml::Value,
}

/// Represents a validated directory with an absolute or properly referenced system path.
///
/// Holds the final calculated path alongside its resolved `IgnoreConfig` rule set.
///
/// ```rust
/// use std::path::PathBuf;
/// use proj::yml::{ResolvedDir, IgnoreConfig};
/// let resolved = ResolvedDir { path: PathBuf::from("_raw"), ignore: IgnoreConfig::Single("all".to_string()) };
/// ```
#[derive(Debug)]
pub struct ResolvedDir {
    pub path: PathBuf,
    pub ignore: IgnoreConfig,
}

impl ProjConfig {
    /// Validates the raw parsed YAML layout and injects missing base default targets.
    ///
    /// It iterates through all specified configuration blocks to ensure custom label names
    /// start with approved identifiers (`cache`, `raw`, `output`, `docs`).
    ///
    /// # Errors
    ///
    /// Returns a validation error `String` if a defined directory label uses an
    /// unsupported prefix layout.
    ///
    /// ```rust
    /// use proj::yml::ProjConfig;
    /// use std::path::PathBuf;
    /// let config = ProjConfig::default();
    /// let validated = config.validate_and_resolve(&PathBuf::from("."), false).unwrap();
    /// assert!(validated.directories.contains_key("raw"));
    /// ```
    pub fn validate_and_resolve(&self, project_root: &std::path::Path, is_dev: bool) -> Result<ValidatedConfig, String> {
        let mut resolved = HashMap::new();

        // 0. Validate Structural Constraints
        if let Some(dest) = &self.build.dest {
            if let Some(remotes) = &self.remotes.local {
                for dest_tag in dest {
                    if !remotes.contains_key(dest_tag) {
                        return Err(format!("Build destination target '{}' is not registered in remotes.local inventory.", dest_tag));
                    }
                }
            } else {
                 return Err("build.dest contains targets but remotes.local is completely undefined.".to_string());
            }
        }

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

        // 3. Resolve Git configuration
        let has_git = project_root.join(".git").exists();

        let mut resolved_git = ResolvedGitConfig { commit: false, push: false };

        if is_dev {
            // Dev builds bypass git
            resolved_git.commit = false;
            resolved_git.push = false;
        } else {
            match &self.build.git {
                GitConfigOpt::Boolean(false) => {
                    resolved_git.commit = false;
                    resolved_git.push = false;
                }
                GitConfigOpt::Boolean(true) => {
                    resolved_git.commit = true;
                    resolved_git.push = false;
                }
                GitConfigOpt::Detailed(detail) => {
                    resolved_git.commit = detail.commit.unwrap_or(has_git);
                    resolved_git.push = detail.push.unwrap_or(false);
                }
            }
        }

        if resolved_git.push && !resolved_git.commit {
            return Err("Configuration error: 'push' cannot be true if 'commit' is false.".to_string());
        }

        Ok(ValidatedConfig {
            remotes: self.remotes.clone(),
            dest: self.build.dest.clone().unwrap_or_default(),
            config: self.config.clone(),
            directories: resolved,
            git: resolved_git,
            restrictions: self.build.restrictions.clone(),
            clear_output: self.build.clear_output.clone(),
            old_dev_remove: self.dev.old_dev_remove,
            parameters: self.parameters.clone(),
        })
    }
}

impl ValidatedConfig {
    /// Resolves an exact or dynamically prefixed directory label against the physical file system.
    ///
    /// Takes the base project root and constructs absolute paths based on mapped configuration definitions.
    /// If an exact match is missing, it intelligently applies suffix nesting rules based on prefix mappings.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested label completely fails prefix structural checks.
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use std::collections::HashMap;
    /// use proj::yml::{ValidatedConfig, ResolvedDir, IgnoreConfig, ResolvedGitConfig, RestrictionsConfig, GlobalConfig};
    ///
    /// let mut dirs = HashMap::new();
    /// dirs.insert("raw".to_string(), ResolvedDir {
    ///     path: PathBuf::from("_raw"),
    ///     ignore: IgnoreConfig::Single("all".to_string())
    /// });
    ///
    /// let config = ValidatedConfig { remotes: Default::default(), dest: vec![],
    ///     config: GlobalConfig::default(), 
    ///     directories: dirs, 
    ///     git: ResolvedGitConfig { commit: false, push: false }, 
    ///     restrictions: RestrictionsConfig::default(), 
    ///     clear_output: None, 
    ///     old_dev_remove: None,
    ///     parameters: Default::default()
    /// };
    /// let path = config.get_path(&PathBuf::from("/mock/root"), "raw-data").unwrap();
    /// assert_eq!(path, PathBuf::from("/mock/root/_raw/data"));
    /// ```
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

/// Reads, validates, and initializes the local `_proj.yml` configuration mapping.
///
/// This performs an upward search for the root context, parses any existing `_proj.yml`,
/// and runs the automated `update_ignores` routine to manage source tracking boundaries.
///
/// # Errors
///
/// Returns an error if the underlying filesystem context lacks a valid structural root
/// or if YAML formatting constraints are severely violated.
///
/// ```rust
/// use std::fs;
/// use tempfile::TempDir;
/// use proj::yml::yml_read_from;
///
/// let temp = TempDir::new().unwrap();
/// let root_path = temp.path();
/// fs::write(root_path.join("VERSION"), "v1.0.0").unwrap();
/// fs::write(root_path.join("_proj.yml"), "directories:\n  raw:\n    ignore: all\n").unwrap();
///
/// let config = yml_read_from(root_path, false).unwrap();
/// assert!(config.directories.contains_key("raw"));
/// ```
pub fn yml_read_from(project_root: &std::path::Path, is_dev: bool) -> Result<ValidatedConfig, String> {
    let yml_path = project_root.join("_proj.yml");

    let config: ProjConfig = if yml_path.exists() {
        let content = fs::read_to_string(&yml_path).map_err(|e| e.to_string())?;

        // Multi-alias conflict safeguard
        let raw_value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
        if let serde_yaml::Value::Mapping(map) = &raw_value {
            let aliases = ["parameters", "parameter", "params", "param"];
            let found_count = aliases.iter().filter(|&a| map.contains_key(&serde_yaml::Value::String(a.to_string()))).count();
            if found_count > 1 {
                return Err("Configuration error: multiple 'parameters' aliases found in _proj.yml.".to_string());
            }
        }

        serde_yaml::from_str(&content).map_err(|e| e.to_string())?
    } else {
        ProjConfig::default()
    };

    let validated = config.validate_and_resolve(project_root, is_dev)?;

    // Execute the Ignore Demarcation Engine
    update_ignores(project_root, &validated)?;

    Ok(validated)
}

/// Extracts a deeply nested parameter value from the loaded project configuration.
///
/// Iteratively traverses the parsed arbitrary parameters tree using a slice of string keys.
///
/// # Arguments
/// * `validated` - The fully loaded and validated configuration.
/// * `keys` - A slice of keys representing the path to the desired parameter.
///
/// # Returns
///
/// Returns `Some(serde_yaml::Value)` containing the found mapping, scalar, or sequence if the path matches exactly.
/// Returns the full root parameter tree if `keys` is empty.
/// Returns `None` if any segment of the path is missing or points to a non-mapping scalar before traversal is complete.
///
/// ```rust
/// use proj::yml::{ValidatedConfig, get_parameter};
/// use std::collections::HashMap;
/// use serde_yaml::{Value, Mapping};
///
/// let mut map = Mapping::new();
/// map.insert(Value::String("a".to_string()), Value::String("b".to_string()));
/// let mut config = ValidatedConfig {
///     remotes: Default::default(), dest: vec![], config: Default::default(),
///     directories: HashMap::new(), git: proj::yml::ResolvedGitConfig { commit: false, push: false },
///     restrictions: Default::default(), clear_output: None, old_dev_remove: None,
///     parameters: Value::Mapping(map),
/// };
///
/// let val = get_parameter(&config, &["a"]);
/// assert_eq!(val, Some(Value::String("b".to_string())));
/// ```
pub fn get_parameter(validated: &ValidatedConfig, keys: &[&str]) -> Option<serde_yaml::Value> {
    let mut current = &validated.parameters;

    if keys.is_empty() {
        return Some(current.clone());
    }

    for key in keys {
        if let serde_yaml::Value::Mapping(map) = current {
            let yml_key = serde_yaml::Value::String((*key).to_string());
            if let Some(next) = map.get(&yml_key) {
                current = next;
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    Some(current.clone())
}

/// Scans the target `_proj.yml` configuration file and intelligently injects a clean `parameters: {}`
/// mapping block if no parameter-related aliases (`parameters`, `parameter`, `params`, `param`) are found.
///
/// This mutation is idempotent and will cleanly return `false` if any structural parameter block already exists.
///
/// # Arguments
/// * `project_root` - A reference to the root path containing the `_proj.yml` configuration.
///
/// # Returns
///
/// Returns `Ok(true)` if the parameters block was successfully injected.
/// Returns `Ok(false)` if the file already contains a parameters definition (skips modification).
/// Returns an error string if file I/O or YAML parsing fails severely.
///
/// ```rust
/// use proj::yml::add_empty_parameters_block;
/// use std::fs;
/// use tempfile::TempDir;
///
/// let temp = TempDir::new().unwrap();
/// let root_path = temp.path();
/// fs::write(root_path.join("_proj.yml"), "directories:\n  raw:\n    ignore: all\n").unwrap();
///
/// let result = add_empty_parameters_block(root_path).unwrap();
/// assert!(result);
///
/// let content = fs::read_to_string(root_path.join("_proj.yml")).unwrap();
/// assert!(content.contains("parameters: {}"));
/// ```
pub fn add_empty_parameters_block(project_root: &std::path::Path) -> Result<bool, String> {
    let yml_path = project_root.join("_proj.yml");

    if !yml_path.exists() {
        return Ok(false); // Do not create a new file if one doesn't exist
    }

    let content = fs::read_to_string(&yml_path).map_err(|e| e.to_string())?;

    let raw_value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

    if let serde_yaml::Value::Mapping(mut map) = raw_value {
        let aliases = ["parameters", "parameter", "params", "param"];
        let found = aliases.iter().any(|&a| map.contains_key(&serde_yaml::Value::String(a.to_string())));

        if found {
            return Ok(false);
        }

        map.insert(
            serde_yaml::Value::String("parameters".to_string()),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );

        let new_content = serde_yaml::to_string(&map).map_err(|e| e.to_string())?;
        fs::write(&yml_path, new_content).map_err(|e| e.to_string())?;

        return Ok(true);
    } else {
        // If the config file is empty or not a map, recreate it as a map with parameters
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("parameters".to_string()),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
        let new_content = serde_yaml::to_string(&map).map_err(|e| e.to_string())?;
        fs::write(&yml_path, new_content).map_err(|e| e.to_string())?;
        return Ok(true);
    }
}

/// Thin production wrapper: Reads, validates, and initializes the local `_proj.yml` configuration mapping using implicit system root.
///
/// # Errors
///
/// Returns an error if the underlying filesystem context lacks a valid structural root
/// or if YAML formatting constraints are severely violated.
///
/// ```rust,ignore
/// use proj::yml::yml_read;
/// let config = yml_read();
/// ```
pub fn yml_read(is_dev: bool) -> Result<ValidatedConfig, String> {
    let project_root = root().ok_or("Could not find project root")?;
    yml_read_from(&project_root, is_dev)
}

/// Mock integration hook for legacy compatibility workflows.
///
/// Returns a static string. Originally intended to retrieve raw configuration text
/// in preceding legacy builds.
///
/// # Panics
///
/// Does not panic.
///
/// ```rust
/// use proj::yml::yml_get;
/// assert_eq!(yml_get(), "projr yml content");
/// ```
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

    #[test]
    fn test_git_config_opt_parsing() {
        let yaml_bool_true = "
directories: {}
build:
  git: true
";
        let config_true: ProjConfig = serde_yaml::from_str(yaml_bool_true).unwrap();
        assert_eq!(config_true.build.git, GitConfigOpt::Boolean(true));

        let yaml_bool_false = "
directories: {}
build:
  git: false
";
        let config_false: ProjConfig = serde_yaml::from_str(yaml_bool_false).unwrap();
        assert_eq!(config_false.build.git, GitConfigOpt::Boolean(false));

        let yaml_detailed = "
directories: {}
build:
  git:
    commit: true
    push: false
";
        let config_detailed: ProjConfig = serde_yaml::from_str(yaml_detailed).unwrap();
        assert_eq!(
            config_detailed.build.git,
            GitConfigOpt::Detailed(GitConfig { commit: Some(true), push: Some(false) })
        );

        let yaml_default = "
directories: {}
build:
  scripts: []
";
        let config_default: ProjConfig = serde_yaml::from_str(yaml_default).unwrap();
        assert_eq!(
            config_default.build.git,
            GitConfigOpt::Detailed(GitConfig { commit: None, push: None })
        );
    }
}

#[cfg(test)]
mod parameter_tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_parameter_aliases() {
        let yaml_params = "
params:
  key1: val1
";
        let config: ProjConfig = serde_yaml::from_str(yaml_params).unwrap();
        assert_eq!(config.parameters["key1"], "val1");

        let yaml_parameter = "
parameter:
  key1: val1
";
        let config2: ProjConfig = serde_yaml::from_str(yaml_parameter).unwrap();
        assert_eq!(config2.parameters["key1"], "val1");
    }

    #[test]
    fn test_multiple_aliases_conflict() {
        let temp = TempDir::new().unwrap();
        let root_path = temp.path();
        fs::write(root_path.join("VERSION"), "v1.0.0").unwrap();
        let yaml_conflict = "
parameters:
  key1: val1
params:
  key2: val2
";
        fs::write(root_path.join("_proj.yml"), yaml_conflict).unwrap();
        let result = yml_read_from(root_path, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("multiple 'parameters' aliases found"));
    }

    #[test]
    fn test_get_parameter_shallow() {
        let yaml = "
parameters:
  model: random_forest
";
        let temp = TempDir::new().unwrap();
        let root_path = temp.path();
        fs::write(root_path.join("VERSION"), "v1.0.0").unwrap();
        fs::write(root_path.join("_proj.yml"), yaml).unwrap();
        let config = yml_read_from(root_path, false).unwrap();

        let val = get_parameter(&config, &["model"]).unwrap();
        assert_eq!(val, serde_yaml::Value::String("random_forest".to_string()));
    }

    #[test]
    fn test_get_parameter_deep() {
        let yaml = "
parameters:
  model:
    type: random_forest
    hyperparameters:
      trees: 100
      depth: 5
";
        let temp = TempDir::new().unwrap();
        let root_path = temp.path();
        fs::write(root_path.join("VERSION"), "v1.0.0").unwrap();
        fs::write(root_path.join("_proj.yml"), yaml).unwrap();
        let config = yml_read_from(root_path, false).unwrap();

        let val = get_parameter(&config, &["model", "hyperparameters", "trees"]).unwrap();
        assert_eq!(val, serde_yaml::Value::Number(100.into()));
    }

    #[test]
    fn test_get_parameter_missing() {
        let yaml = "
parameters:
  model: random_forest
";
        let temp = TempDir::new().unwrap();
        let root_path = temp.path();
        fs::write(root_path.join("VERSION"), "v1.0.0").unwrap();
        fs::write(root_path.join("_proj.yml"), yaml).unwrap();
        let config = yml_read_from(root_path, false).unwrap();

        let val = get_parameter(&config, &["model", "not_exist"]);
        assert!(val.is_none());

        let val2 = get_parameter(&config, &["not_exist"]);
        assert!(val2.is_none());
    }

    #[test]
    fn test_get_parameter_empty_query() {
        let yaml = "
parameters:
  model: random_forest
";
        let temp = TempDir::new().unwrap();
        let root_path = temp.path();
        fs::write(root_path.join("VERSION"), "v1.0.0").unwrap();
        fs::write(root_path.join("_proj.yml"), yaml).unwrap();
        let config = yml_read_from(root_path, false).unwrap();

        let val = get_parameter(&config, &[]).unwrap();
        let mut expected_map = serde_yaml::Mapping::new();
        expected_map.insert(
            serde_yaml::Value::String("model".to_string()),
            serde_yaml::Value::String("random_forest".to_string())
        );
        assert_eq!(val, serde_yaml::Value::Mapping(expected_map));
    }

    #[test]
    fn test_add_empty_parameters_block() {
        let temp = TempDir::new().unwrap();
        let root_path = temp.path();
        fs::write(root_path.join("VERSION"), "v1.0.0").unwrap();

        // No proj.yml
        let res = add_empty_parameters_block(root_path).unwrap();
        assert!(!res);

        // Existing empty
        fs::write(root_path.join("_proj.yml"), "directories:\n  raw:\n    ignore: all\n").unwrap();
        let res = add_empty_parameters_block(root_path).unwrap();
        assert!(res);
        let content = fs::read_to_string(root_path.join("_proj.yml")).unwrap();
        assert!(content.contains("parameters: {}"));

        // Existing with parameter
        let yaml_param = "
directories: {}
param:
  key: val
";
        fs::write(root_path.join("_proj.yml"), yaml_param).unwrap();
        let res = add_empty_parameters_block(root_path).unwrap();
        assert!(!res);
    }
}
