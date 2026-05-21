use std::env;
use std::path::PathBuf;
use std::fs;
use crate::yml::{ValidatedConfig, IgnoreConfig};

/// Synchronizes ignore rules dynamically across `.gitignore` and `.Rbuildignore`.
///
/// Modifies the ignore files in place within the demarcated regions defined by
/// `PROJ MANAGED` markers. It resolves the specific rules outlined in the `ValidatedConfig`.
///
/// # Errors
///
/// Returns an error if an ignore file cannot be created, written to, or if
/// the target physical directory doesn't have required permission layers.
///
/// ```rust
/// use std::path::PathBuf;
/// use std::collections::HashMap;
/// use tempfile::TempDir;
/// use proj::yml::{ValidatedConfig, ResolvedDir, IgnoreConfig};
/// use proj::ignore::update_ignores_for;
///
/// let temp = TempDir::new().unwrap();
/// let root = temp.path().to_path_buf();
///
/// use proj::yml::ResolvedGitConfig;
/// let mut dirs = HashMap::new();
/// dirs.insert("raw".to_string(), ResolvedDir {
///     path: root.join("_raw"),
///     ignore: IgnoreConfig::Single("all".to_string())
/// });
///
/// let validated = ValidatedConfig { directories: dirs, git: ResolvedGitConfig { commit: false, push: false } };
/// update_ignores_for(&root, &validated).unwrap();
///
/// assert!(root.join(".gitignore").exists());
/// ```
pub fn update_ignores_for(project_root: &std::path::Path, validated: &ValidatedConfig) -> Result<(), String> {
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

/// Discovers the conceptual project root directory via heuristic path traversal.
///
/// Searches upwards from the current working directory for a `VERSION` file.
/// When found, it verifies if a `_proj.yml` or `.git` directory exists alongside it.
/// If not, it continues ascending up to five directory levels to find a comprehensive root.
/// If no secondary markers are found within that limit, it falls back to the original directory containing the `VERSION` file.
///
/// # Panics
///
/// Should not panic as file traversal heavily checks boundary existence natively.
///
/// ```rust
/// use std::fs;
/// use tempfile::TempDir;
/// use proj::ignore::root_from;
///
/// let temp = TempDir::new().unwrap();
/// fs::write(temp.path().join("VERSION"), "v1.0.0").unwrap();
///
/// let result = root_from(temp.path()).unwrap();
/// assert!(result.exists());
/// ```
pub fn root_from(start_path: &std::path::Path) -> Option<PathBuf> {
    let mut current_path = start_path;

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

/// Thin production wrapper: discovers the conceptual project root directory via heuristic path traversal.
///
/// This uses the current directory as the starting search context.
///
/// # Errors
///
/// Returns `None` if the root cannot be located or if the environment's current working directory is invalid.
///
/// ```rust,ignore
/// use proj::ignore::root;
/// let result = root();
/// ```
pub fn root() -> Option<PathBuf> {
    let current_dir = env::current_dir().ok()?;
    root_from(&current_dir)
}

/// Thin production wrapper: synchronizes ignore rules dynamically across `.gitignore` and `.Rbuildignore`.
///
/// # Errors
///
/// Returns an error if the files cannot be updated or if required permissions are lacking.
///
/// ```rust,ignore
/// use proj::ignore::update_ignores;
/// update_ignores(project_root, validated_config).unwrap();
/// ```
pub fn update_ignores(project_root: &std::path::Path, validated: &ValidatedConfig) -> Result<(), String> {
    update_ignores_for(project_root, validated)
}
