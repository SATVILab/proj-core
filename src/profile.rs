use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;

/// Extracts and normalizes profile tags from PROJR_PROFILE environment variable.
///
/// Splits by ',' or ';', strips whitespace, and rejects 'default' and 'local'.
pub fn get_active_profiles() -> Vec<String> {
    if let Ok(val) = env::var("PROJR_PROFILE") {
        val.split(|c| c == ',' || c == ';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "default" && s != "local" && is_valid_profile_name(s))
            .collect()
    } else {
        Vec::new()
    }
}

fn is_valid_profile_name(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Spawns a clean `_projr-<name>.yml` workspace node.
pub fn create_profile(name: &str, project_root: &Path) -> Result<PathBuf, String> {
    if name == "default" || name == "local" || !is_valid_profile_name(name) {
        return Err(format!("Invalid profile name: {}", name));
    }
    let filename = format!("_projr-{}.yml", name);
    let path = project_root.join(filename);
    if path.exists() {
        return Err(format!("Profile {} already exists", name));
    }
    fs::write(&path, "").map_err(|e| e.to_string())?;
    Ok(path)
}

/// Recursively sets all leaf values to null
fn nullify_values(val: &mut Value) {
    match val {
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                nullify_values(v);
            }
        }
        Value::Array(_arr) => {
            // Arrays are typically replaced entirely, but if we need structural clone, we just nullify elements
            // Actually, for local profile it might be easier to just make it an empty array or null
            *val = Value::Null;
        }
        _ => {
            *val = Value::Null;
        }
    }
}

/// Scans the baseline `_proj.yml` structure and clones its layout into `_projr-local.yml` with all assignments mapped to null.
pub fn create_local_profile(project_root: &Path) -> Result<PathBuf, String> {
    let baseline_path = project_root.join("_proj.yml");
    let local_path = project_root.join("_projr-local.yml");

    if local_path.exists() {
        return Err("Local profile _projr-local.yml already exists".to_string());
    }

    if baseline_path.exists() {
        let content = fs::read_to_string(&baseline_path).map_err(|e| e.to_string())?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

        // Convert to serde_json::Value for processing
        let mut json_value: serde_json::Value = serde_json::to_value(yaml_value).map_err(|e| e.to_string())?;
        nullify_values(&mut json_value);

        let out_yaml: serde_yaml::Value = serde_json::from_value(json_value).map_err(|e| e.to_string())?;
        let out_str = serde_yaml::to_string(&out_yaml).map_err(|e| e.to_string())?;
        fs::write(&local_path, out_str).map_err(|e| e.to_string())?;
    } else {
        fs::write(&local_path, "").map_err(|e| e.to_string())?;
    }

    Ok(local_path)
}

/// Permanently deletes the matching config node.
pub fn delete_profile(name: &str, project_root: &Path) -> Result<(), String> {
    let filename = format!("_projr-{}.yml", name);
    let path = project_root.join(filename);
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Profile {} does not exist", name))
    }
}
