use std::path::{Path, PathBuf};
use std::fs;
use crate::yml::ProjConfig;

/// Runs the build pipeline for the project within the provided `project_root`.
///
/// Ensures mutual exclusion rules are enforced regarding `_quarto.yml` and `_bookdown.yml`,
/// resolves targets using explicit `build.scripts` configuration or fallback auto-discovery,
/// reorders execution according to constraints, and delegates script execution.
///
/// # Errors
///
/// Returns an error if structural validation constraints fail (e.g., both Quarto and Bookdown configs are present and script configs conflict),
/// if the scripts fail to execute, or if IO operations fail.
///
/// ```rust,no_run
/// use std::path::PathBuf;
/// use proj::build::build_project;
///
/// // build_project(&PathBuf::from("/mock/root"), None).unwrap();
/// ```
pub fn build_project(project_root: &Path, cli_profile: Option<&str>) -> Result<(), String> {
    let quarto_exists = project_root.join("_quarto.yml").exists();
    let bookdown_exists = project_root.join("_bookdown.yml").exists();

    let yml_path = project_root.join("_proj.yml");
    let mut build_scripts: Option<Vec<String>> = None;
    let mut profile: Option<String> = cli_profile.map(|s| s.to_string());

    if yml_path.exists() {
        let content = fs::read_to_string(&yml_path).map_err(|e| e.to_string())?;
        if let Ok(config) = serde_yaml::from_str::<ProjConfig>(&content) {
            if let Some(build) = config.build {
                build_scripts = build.scripts;
                if profile.is_none() {
                    profile = build.profile;
                }
            }
        }
    }

    if let Some(scripts) = build_scripts {
        // Path A: Explicit Configuration via build.scripts
        let resolved_files = resolve_explicit_scripts(project_root, &scripts)?;

        // Mutual Exclusion Error (Rule 1 & 2)
        if quarto_exists && bookdown_exists {
            let has_docs = resolved_files.iter().any(|f| {
                let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
                ext == "qmd" || ext == "Rmd"
            });
            if has_docs {
                return Err("Mutual Exclusion Error: Both _quarto.yml and _bookdown.yml exist in the workspace, and the build.scripts key in _proj.yml is omitted or includes .qmd or .Rmd files. Aborting build.".to_string());
            }
        }

        if resolved_files.is_empty() {
            // The Explicit Empty Exception ("Don't Run Anything")
            return Ok(());
        }

        let mut doc_files = Vec::new();
        for file in &resolved_files {
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "qmd" || ext == "Rmd" {
                doc_files.push(file.clone());
            }
        }

        if quarto_exists && !doc_files.is_empty() {
            rewrite_quarto_yml(project_root, &doc_files)?;
        } else if bookdown_exists && !doc_files.is_empty() {
            rewrite_bookdown_yml(project_root, &doc_files)?;
        }

        // Reordering constraint
        if quarto_exists || bookdown_exists {
            let mut pre_scripts = Vec::new();
            let mut post_scripts = Vec::new();
            let mut seen_doc = false;

            for file in resolved_files {
                let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "qmd" || ext == "Rmd" {
                    seen_doc = true;
                } else if ext == "R" || ext == "py" {
                    if seen_doc {
                        post_scripts.push(file);
                    } else {
                        pre_scripts.push(file);
                    }
                }
            }

            // Execute pre-scripts
            for script in pre_scripts {
                let p = profile.clone();
                execute_script(&script, p.as_deref())?;
            }

            // Execute doc clusters
            if quarto_exists && !doc_files.is_empty() {
                let p = profile.clone();
                execute_project_render(project_root, true, p.as_deref())?;
            } else if bookdown_exists && !doc_files.is_empty() {
                let p = profile.clone();
                execute_project_render(project_root, false, p.as_deref())?;
            }

            // Execute post-scripts
            for script in post_scripts {
                let p = profile.clone();
                execute_script(&script, p.as_deref())?;
            }
        } else {
            // No project files, just execute in exact relative order
            for file in resolved_files {
                let p = profile.clone();
                execute_script(&file, p.as_deref())?;
            }
        }

    } else {
        // Mutual Exclusion Error (Rule 1 & 2)
        if quarto_exists && bookdown_exists {
            return Err("Mutual Exclusion Error: Both _quarto.yml and _bookdown.yml exist in the workspace, and the build.scripts key in _proj.yml is omitted or includes .qmd or .Rmd files. Aborting build.".to_string());
        }

        // Path B: Fallback Automated Auto-Discovery
        if quarto_exists {
            execute_project_render(project_root, true, profile.as_deref())?;
        } else if bookdown_exists {
            execute_project_render(project_root, false, profile.as_deref())?;
        } else {
            let resolved_files = resolve_fallback_scripts(project_root)?;
            for file in resolved_files {
                execute_script(&file, profile.as_deref())?;
            }
        }
    }

    Ok(())
}

/// Evaluates Path A: Explicit Configuration via `build.scripts`.
///
/// Supports exclusion globs starting with `!`, strictly matches root documents when no sub-directory
/// prefix is provided, and captures valid formats under sub-directories.
fn resolve_explicit_scripts(project_root: &Path, scripts: &[String]) -> Result<Vec<PathBuf>, String> {
    if scripts.is_empty() {
        return Ok(Vec::new()); // The Explicit Empty Exception ("Don't Run Anything")
    }

    let mut includes = Vec::new();
    let mut excludes = Vec::new();

    for script in scripts {
        if script.starts_with('!') {
            excludes.push(&script[1..]);
        } else {
            includes.push(script.clone());
        }
    }

    // Convert patterns to absolute globbing targets
    let mut matched_files = Vec::new();
    for pattern in &includes {
        // If there's no directory prefix (e.g. *.qmd), it should only match at the root.
        let is_root_only = !pattern.contains('/');
        let target_pattern = if is_root_only {
            project_root.join(pattern).to_string_lossy().to_string()
        } else {
            project_root.join(pattern).to_string_lossy().to_string()
        };

        let paths = glob::glob(&target_pattern)
            .map_err(|e| format!("Invalid glob pattern '{}': {}", target_pattern, e))?;

        for entry in paths {
            if let Ok(path) = entry {
                if path.is_file() {
                    matched_files.push(path);
                }
            }
        }
    }

    // Process excludes
    let mut final_files = Vec::new();
    for file in matched_files {
        let mut is_excluded = false;

        for exclude in &excludes {
            // Very simple matcher for excludes
            let is_root_only = !exclude.contains('/');
            let exclude_pattern = if is_root_only {
                project_root.join(exclude).to_string_lossy().to_string()
            } else {
                project_root.join(exclude).to_string_lossy().to_string()
            };

            if let Ok(ex_pattern) = glob::Pattern::new(&exclude_pattern) {
                if ex_pattern.matches_path(&file) {
                    is_excluded = true;
                    break;
                }
            }
        }

        if !is_excluded {
            if !final_files.contains(&file) {
                final_files.push(file);
            }
        }
    }

    // Keep the relative order of elements provided in build.scripts roughly based on include pattern order.
    // As multiple patterns might overlap, we just filter unique ordered by first match.
    Ok(final_files)
}

/// Dynamically modifies the `project.render` array in `_quarto.yml`.
fn rewrite_quarto_yml(project_root: &Path, qmd_files: &[PathBuf]) -> Result<(), String> {
    let qmd_paths: Vec<String> = qmd_files.iter()
        .map(|p| p.strip_prefix(project_root).unwrap_or(p).to_string_lossy().to_string())
        .collect();

    let quarto_path = project_root.join("_quarto.yml");
    let content = fs::read_to_string(&quarto_path).map_err(|e| e.to_string())?;

    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

    if let serde_yaml::Value::Mapping(ref mut map) = yaml {
        let proj_key = serde_yaml::Value::String("project".to_string());
        if !map.contains_key(&proj_key) {
            map.insert(proj_key.clone(), serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
        }

        if let Some(serde_yaml::Value::Mapping(proj_map)) = map.get_mut(&proj_key) {
            let render_key = serde_yaml::Value::String("render".to_string());

            let seq: Vec<serde_yaml::Value> = qmd_paths.into_iter().map(|s| serde_yaml::Value::String(s)).collect();
            proj_map.insert(render_key, serde_yaml::Value::Sequence(seq));
        }
    }

    let out_content = serde_yaml::to_string(&yaml).map_err(|e| e.to_string())?;
    fs::write(&quarto_path, out_content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Dynamically modifies the `rmd_files` array in `_bookdown.yml`.
fn rewrite_bookdown_yml(project_root: &Path, rmd_files: &[PathBuf]) -> Result<(), String> {
    let rmd_paths: Vec<String> = rmd_files.iter()
        .map(|p| p.strip_prefix(project_root).unwrap_or(p).to_string_lossy().to_string())
        .collect();

    let bookdown_path = project_root.join("_bookdown.yml");
    let content = fs::read_to_string(&bookdown_path).map_err(|e| e.to_string())?;

    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

    if let serde_yaml::Value::Mapping(ref mut map) = yaml {
        let key = serde_yaml::Value::String("rmd_files".to_string());
        let seq: Vec<serde_yaml::Value> = rmd_paths.into_iter().map(|s| serde_yaml::Value::String(s)).collect();
        map.insert(key, serde_yaml::Value::Sequence(seq));
    }

    let out_content = serde_yaml::to_string(&yaml).map_err(|e| e.to_string())?;
    fs::write(&bookdown_path, out_content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Evaluates Path B: Fallback Automated Auto-Discovery
///
/// Sweeps for `.qmd`, `.Rmd`, `.R`, `.py` in that strict order.
fn resolve_fallback_scripts(project_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    // Shallow directory sweep
    if let Ok(entries) = fs::read_dir(project_root) {
        let mut qmd = Vec::new();
        let mut rmd = Vec::new();
        let mut r = Vec::new();
        let mut py = Vec::new();

        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let path = entry.path();
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        match ext {
                            "qmd" => qmd.push(path),
                            "Rmd" => rmd.push(path),
                            "R" => r.push(path),
                            "py" => py.push(path),
                            _ => {}
                        }
                    }
                }
            }
        }

        // Sort alphabetically to be deterministic
        qmd.sort();
        rmd.sort();
        r.sort();
        py.sort();

        files.extend(qmd);
        files.extend(rmd);
        files.extend(r);
        files.extend(py);
    }

    Ok(files)
}

use std::process::Command;

/// Executes a single script in an isolated subprocess.
fn execute_script(script_path: &Path, profile: Option<&str>) -> Result<(), String> {
    let parent_dir = script_path.parent().unwrap_or(Path::new(""));
    let ext = script_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let mut cmd = match ext {
        "R" => {
            let mut c = Command::new("Rscript");
            c.arg(script_path);
            c
        },
        "py" => {
            let mut c = Command::new("python");
            c.arg(script_path);
            c
        },
        "qmd" => {
            let mut c = Command::new("quarto");
            c.arg("render").arg(script_path);
            c
        },
        "Rmd" => {
            let mut c = Command::new("Rscript");
            c.arg("-e").arg(format!("rmarkdown::render('{}')", script_path.file_name().unwrap().to_string_lossy()));
            c
        },
        _ => {
            return Err(format!("Unsupported script extension for execution: {}", script_path.display()));
        }
    };

    cmd.current_dir(parent_dir);
    if let Some(p) = profile {
        cmd.env("PROJR_PROFILE", p);
    }

    let status = cmd.status().map_err(|e| format!("Failed to execute script {}: {}", script_path.display(), e))?;
    if !status.success() {
        return Err(format!("Script {} failed with status: {}", script_path.display(), status));
    }

    Ok(())
}

/// Executes a quarto or bookdown project render.
fn execute_project_render(project_root: &Path, is_quarto: bool, profile: Option<&str>) -> Result<(), String> {
    let mut cmd = if is_quarto {
        let mut c = Command::new("quarto");
        c.arg("render");
        c
    } else {
        let mut c = Command::new("Rscript");
        c.arg("-e").arg("rmarkdown::render_site()");
        c
    };

    cmd.current_dir(project_root);
    if let Some(p) = profile {
        cmd.env("PROJR_PROFILE", p);
    }

    let status = cmd.status().map_err(|e| format!("Failed to execute project render: {}", e))?;
    if !status.success() {
        return Err(format!("Project render failed with status: {}", status));
    }

    Ok(())
}
