use std::path::Path;
use std::fs;
use crate::yml::ValidatedConfig;

/// Clears old development folders if configured.
pub fn clear_old(
    project_root: &Path,
    current_version: &str,
    is_dev: bool,
    old_dev_remove: Option<bool>,
) -> anyhow::Result<()> {
    if old_dev_remove != Some(true) {
        return Ok(());
    }

    let base_path = project_root.join("_tmp").join("projr");
    if !base_path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&base_path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            let dir_name = entry.file_name();
            let dir_name_str = dir_name.to_string_lossy();

            if is_dev {
                if dir_name_str != current_version {
                    fs::remove_dir_all(entry.path())?;
                }
            } else {
                if dir_name_str != "log" {
                    fs::remove_dir_all(entry.path())?;
                }
            }
        }
    }

    Ok(())
}

/// Clears pre-build caches and output folders.
pub fn clear_pre(
    project_root: &Path,
    current_version: &str,
    config: &ValidatedConfig,
    clear_output: Option<&str>,
) -> anyhow::Result<()> {
    // 1. Versioned cache path clearance
    let versioned_cache_path = project_root
        .join("_tmp")
        .join("projr")
        .join(current_version);

    if versioned_cache_path.exists() {
        for entry in fs::read_dir(&versioned_cache_path)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Exclude 'old' and 'docs' ? No, the reviewer said: "Update `clear_pre` to delete the contents of the versioned cache directory except for `old/`, correctly allowing `docs/` to be deleted during pre-build."
            if name_str != "old" {
                let metadata = entry.metadata()?;
                if metadata.is_dir() {
                    fs::remove_dir_all(entry.path())?;
                } else {
                    fs::remove_file(entry.path())?;
                }
            }
        }
    }

    // 2. For output directory clearing
    if clear_output == Some("never") {
        return Ok(());
    }

    let is_pre = clear_output == Some("pre");
    let cache_base = match config.get_path(project_root, "cache") {
        Ok(p) => p,
        Err(_) => project_root.join("_tmp"), // default fallback
    };

    for (label, _) in &config.directories {
        let lower_label = label.to_lowercase();
        if !(lower_label.starts_with("output") || lower_label == "data") {
            continue;
        }
        if lower_label == "docs" {
            continue;
        }

        // Construct the safe cache path
        // cache_base is typically _tmp. The safe path is <project_root>/_tmp/projr/<current_version>/<label>
        // But since we use cache_base, it's cache_base/projr/<current_version>/<label>
        let safe_cache_path = cache_base.join("projr").join(current_version).join(label);
        if safe_cache_path.exists() {
            for entry in fs::read_dir(&safe_cache_path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_dir() {
                    fs::remove_dir_all(entry.path())?;
                } else {
                    fs::remove_file(entry.path())?;
                }
            }
        }

        // If 'pre', clear the public unsafe path
        if is_pre {
            let unsafe_path = match config.get_path(project_root, label) {
                Ok(p) => p,
                Err(_) => continue,
            };

            if unsafe_path.exists() {
                for entry in fs::read_dir(&unsafe_path)? {
                    let entry = entry?;
                    let metadata = entry.metadata()?;
                    if metadata.is_dir() {
                        fs::remove_dir_all(entry.path())?;
                    } else {
                        fs::remove_file(entry.path())?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Clears post-build deployment folders.
pub fn clear_post(
    project_root: &Path,
    is_dev: bool,
    config: &ValidatedConfig,
    clear_output: Option<&str>,
    is_single_doc_engine: bool,
) -> anyhow::Result<()> {
    if !is_dev && is_single_doc_engine {
        if let Ok(docs_path) = config.get_path(project_root, "docs") {
            if docs_path.exists() {
                for entry in fs::read_dir(&docs_path)? {
                    let entry = entry?;
                    let metadata = entry.metadata()?;
                    if metadata.is_dir() {
                        fs::remove_dir_all(entry.path())?;
                    } else {
                        fs::remove_file(entry.path())?;
                    }
                }
            }
        }
    }

    if clear_output == Some("post") {
        for (label, _) in &config.directories {
            let lower_label = label.to_lowercase();
            if lower_label.starts_with("output") || lower_label == "data" {
                if let Ok(unsafe_path) = config.get_path(project_root, label) {
                    if unsafe_path.exists() {
                        for entry in fs::read_dir(&unsafe_path)? {
                            let entry = entry?;
                            let metadata = entry.metadata()?;
                            if metadata.is_dir() {
                                fs::remove_dir_all(entry.path())?;
                            } else {
                                fs::remove_file(entry.path())?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
