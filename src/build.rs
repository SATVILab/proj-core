use std::path::{Path, PathBuf};
use std::fs;
use crate::yml::ProjConfig;
use crate::version::{version_get_from, version_set_at};

/// Defines the mode of the build cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    /// Development run (`proj build-dev`)
    Dev,
    /// Production run, bumping major version (`proj build --major`)
    ProdMajor,
    /// Production run, bumping minor version (`proj build --minor`)
    ProdMinor,
    /// Production run, bumping patch version (`proj build --patch` or default)
    ProdPatch,
}

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
/// ```rust
/// use std::path::PathBuf;
/// use proj::build::{build_project, BuildMode};
/// use tempfile::TempDir;
///
/// let temp = TempDir::new().unwrap();
/// std::fs::write(temp.path().join("VERSION"), "Version: v1.0.0").unwrap();
/// // Create a dummy _proj.yml to avoid fallback searching which executes everything
/// // We set push: false so we don't trigger GitHub token lookups in CI without env vars
/// std::fs::write(temp.path().join("_proj.yml"), "build:\n  scripts: []\n  git:\n    commit: false\n    push: false\nconfig:\n  git:\n    use_proj_cred_helper: false").unwrap();
/// build_project(temp.path(), BuildMode::ProdPatch, None, None).unwrap();
/// ```
use crate::git::{is_git_installed, check_git_profile, git_commit_all, git_push};
use crate::yml::yml_read_from;
use crate::build_pre::run_pre_flight_checks;

pub fn build_project(project_root: &Path, mode: BuildMode, cli_profile: Option<&str>, description: Option<&str>) -> Result<(), String> {
    let is_dev = mode == BuildMode::Dev;
    let is_prod_run = matches!(mode, BuildMode::ProdMajor | BuildMode::ProdMinor | BuildMode::ProdPatch);

    // ==========================================
    // STEP B: Pre-Build Validation & Execution
    // ==========================================

    // 1. Config Audit
    let config = yml_read_from(project_root, is_dev)?;

    // 2. Git Capability Audit
    if config.git.commit {
        if !is_git_installed() {
            return Err("Git is required for commit but not found on system PATH.".to_string());
        }
        check_git_profile(Some(project_root))?;
    }

    // 3. Resolve configs and hooks
    let quarto_exists = project_root.join("_quarto.yml").exists();
    let bookdown_exists = project_root.join("_bookdown.yml").exists();

    let yml_path = project_root.join("_proj.yml");
    let mut build_scripts: Option<Vec<String>> = None;
    let mut profile: Option<String> = cli_profile.map(|s| s.to_string());
    let mut hooks_config: crate::yml::HooksConfig = crate::yml::HooksConfig::default();

    if yml_path.exists() {
        let content = fs::read_to_string(&yml_path).map_err(|e| e.to_string())?;
        if let Ok(proj_conf) = serde_yaml::from_str::<ProjConfig>(&content) {
            let build = proj_conf.build;
            let dev = proj_conf.dev;

            if is_dev {
                if let Some(dev_scripts) = dev.scripts {
                    if dev_scripts.is_empty() {
                        return Err("dev.scripts cannot be empty when running in dev mode. The purpose of dev mode is to run scripts.".to_string());
                    }
                    build_scripts = Some(dev_scripts);
                } else {
                    build_scripts = build.scripts;
                }

                if dev.hooks.is_some() {
                    hooks_config = dev.hooks.unwrap();
                } else {
                    hooks_config = build.hooks;
                }
            } else {
                build_scripts = build.scripts;
                hooks_config = build.hooks;
            }

            if profile.is_none() {
                profile = build.profile;
            }
        }
    }

    // Extract hooks paths
    let mut raw_hooks = Vec::new();
    if let Some(pre_hooks) = &hooks_config.pre { raw_hooks.extend(pre_hooks.iter().cloned()); }
    if let Some(post_hooks) = &hooks_config.post { raw_hooks.extend(post_hooks.iter().cloned()); }
    if let Some(both_hooks) = &hooks_config.both { raw_hooks.extend(both_hooks.iter().cloned()); }

    // Pre-flight Environment Verification
    let mut resolved_files = Vec::new();
    if let Some(scripts) = &build_scripts {
        resolved_files = resolve_explicit_scripts(project_root, scripts)?;
    } else {
        if !quarto_exists && !bookdown_exists {
            resolved_files = resolve_fallback_scripts(project_root)?;
        }
    }

    // Upfront check for hooks
    let resolved_hooks = resolve_explicit_scripts(project_root, &raw_hooks)?;

    for expected_hook in raw_hooks.iter() {
        if expected_hook.starts_with('!') { continue; } // Exclusions not validated directly here
        let mut found = false;
        // Simple validation: Ensure explicitly asked scripts/hooks map to at least one file.
        // It's covered by `resolve_explicit_scripts` failing on glob error, but we want to fail fast if explicitly missing.
        for h in &resolved_hooks {
            if h.to_string_lossy().contains(&expected_hook.replace('/', std::path::MAIN_SEPARATOR_STR)) {
                found = true;
                break;
            }
        }
        let target_pattern = project_root.join(expected_hook);
        if !found && !target_pattern.exists() && !target_pattern.parent().map(|p| p.exists()).unwrap_or(true) {
            // Best effort check for explicitly missed files
        }
    }

    // Fail fast if explicit targets are missing entirely (hooks or scripts without globs)
    let check_explicit_missing = |items: &[String]| -> Result<(), String> {
        for item in items {
            if item.starts_with('!') || item.contains('*') || item.contains('?') { continue; }
            let p = project_root.join(item);
            if !p.exists() {
                return Err(format!("Error: Explicitly defined script or hook file '{}' does not exist.", item));
            }
        }
        Ok(())
    };

    if let Some(scripts) = &build_scripts {
        check_explicit_missing(scripts)?;
    }
    check_explicit_missing(&raw_hooks)?;

    let mut validation_files = resolved_files.clone();
    validation_files.extend(resolved_hooks.clone());

    let (resolved_token, resolved_python_cmd) = run_pre_flight_checks(
        project_root,
        &config,
        is_prod_run,
        &validation_files,
        quarto_exists,
        bookdown_exists,
    )?;

    // 4. VERSION Initialization Check
    let mut initial_version = match version_get_from(project_root) {
        Some(v) => v,
        None => {
            let desc_path = project_root.join("DESCRIPTION");
            if desc_path.exists() {
                let content = fs::read_to_string(&desc_path).map_err(|e| e.to_string())?;
                let mut found_ver = None;
                for line in content.lines() {
                    if line.starts_with("Version:") {
                        let ver_str = line.trim_start_matches("Version:").trim();
                        if let Some(v) = crate::version::ProjVersion::parse(ver_str) {
                            found_ver = Some(v);
                            break;
                        }
                    }
                }
                found_ver.ok_or_else(|| "Could not extract a valid Version from DESCRIPTION file.".to_string())?
            } else {
                let v = crate::version::ProjVersion { major: 0, minor: 0, patch: 1, dev: 0 };
                version_set_at(project_root, &v.to_string(true))?;
                v
            }
        }
    };

    let version_before_build = initial_version.clone();

    // 4.5. Workspace Pre-clearing Hooks
    let current_version = initial_version.to_string(false);
    let clear_output_env = std::env::var("PROJR_CLEAR_OUTPUT").ok();
    let clear_output_val = clear_output_env.as_deref().or(config.clear_output.as_deref());

    crate::clear::clear_old(project_root, &current_version, is_dev, config.old_dev_remove);
    crate::clear::clear_pre(project_root, &current_version, &config, clear_output_val);

    // 5. Version Bump
    if is_dev {
        if initial_version.dev == 0 {
            initial_version.dev = 1;
            version_set_at(project_root, &initial_version.to_string(false))?;
        }
    } else {
        match mode {
            BuildMode::ProdMajor => {
                initial_version.major += 1;
                initial_version.minor = 0;
                initial_version.patch = 0;
                initial_version.dev = 0;
            }
            BuildMode::ProdMinor => {
                initial_version.minor += 1;
                initial_version.patch = 0;
                initial_version.dev = 0;
            }
            BuildMode::ProdPatch => {
                initial_version.patch += 1;
                initial_version.dev = 0;
            }
            _ => {}
        }
        version_set_at(project_root, &initial_version.to_string(false))?;
    }

    // 6. Git auto-ignore (already handled in yml_read_from via update_ignores)

    // Execute Pre-Build Hooks
    if let Some(pre_hooks) = &hooks_config.pre {
        let pre_resolved = resolve_explicit_scripts(project_root, pre_hooks)?;
        for hook in pre_resolved {
            execute_script(&hook, profile.as_deref(), resolved_python_cmd.as_deref())?;
        }
    }
    if let Some(both_hooks) = &hooks_config.both {
        let both_resolved = resolve_explicit_scripts(project_root, both_hooks)?;
        for hook in both_resolved {
            execute_script(&hook, profile.as_deref(), resolved_python_cmd.as_deref())?;
        }
    }

    // 6. Git Pre-Snapshot
    if config.git.commit {
        git_commit_all("Snapshot pre-build", Some(project_root))?;
    }

    // ==========================================
    // STEP C: Core Document Compilation
    // ==========================================

    // Isolate execution logic inside catch_unwind
    let pr = project_root.to_path_buf();
    let profile_clone = profile.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_build_pipeline(&pr, build_scripts, profile_clone, quarto_exists, bookdown_exists, resolved_python_cmd.as_deref())
    }));

    match result {
        Ok(Ok(())) => {
            // ==========================================
            // STEP D: Post-Build Operations
            // ==========================================

            let is_single_doc_engine = !quarto_exists && !bookdown_exists;
            let clear_output_env = std::env::var("PROJR_CLEAR_OUTPUT").ok();
            let clear_output_val = clear_output_env.as_deref().or(config.clear_output.as_deref());
            crate::clear::clear_post(project_root, is_dev, &config, clear_output_val, is_single_doc_engine);

            if config.git.commit {
                let ver_str = initial_version.to_string(false);
                let final_message = match description {
                    Some(desc) if !desc.trim().is_empty() => format!("Build v{}: {}", ver_str, desc.trim()),
                    _ => format!("Build v{}", ver_str),
                };

                git_commit_all(&final_message, Some(project_root))?;
            }

            // Execute Post-Build Hooks (after post-build commit, before push)
            if let Some(post_hooks) = &hooks_config.post {
                let post_resolved = resolve_explicit_scripts(project_root, post_hooks)?;
                for hook in post_resolved {
                    execute_script(&hook, profile.as_deref(), resolved_python_cmd.as_deref())?;
                }
            }
            if let Some(both_hooks) = &hooks_config.both {
                let both_resolved = resolve_explicit_scripts(project_root, both_hooks)?;
                for hook in both_resolved {
                    execute_script(&hook, profile.as_deref(), resolved_python_cmd.as_deref())?;
                }
            }

            if config.git.commit {
                if config.git.push {
                    git_push(Some(project_root), resolved_token.as_deref())?;
                }
            }
            Ok(())
        },
        Ok(Err(e)) => {
            if is_prod_run {
                // Revert to Development State of Previous Baseline
                let mut reverted = version_before_build.clone();
                if reverted.dev == 0 {
                    reverted.dev = 1;
                }
                let _ = version_set_at(project_root, &reverted.to_string(false));
            }
            Err(e)
        }
        Err(_) => {
            if is_prod_run {
                // Revert to Development State of Previous Baseline
                let mut reverted = version_before_build.clone();
                if reverted.dev == 0 {
                    reverted.dev = 1;
                }
                let _ = version_set_at(project_root, &reverted.to_string(false));
            }
            Err("Build pipeline panicked unexpectedly.".to_string())
        }
    }
}

fn execute_build_pipeline(
    project_root: &Path,
    build_scripts: Option<Vec<String>>,
    profile: Option<String>,
    quarto_exists: bool,
    bookdown_exists: bool,
    resolved_python_cmd: Option<&str>,
) -> Result<(), String> {
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
                execute_script(&script, p.as_deref(), resolved_python_cmd)?;
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
                execute_script(&script, p.as_deref(), resolved_python_cmd)?;
            }
        } else {
            // No project files, just execute in exact relative order
            for file in resolved_files {
                let p = profile.clone();
                execute_script(&file, p.as_deref(), resolved_python_cmd)?;
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
                execute_script(&file, profile.as_deref(), resolved_python_cmd)?;
            }
        }
    }

    Ok(())
}

/// Evaluates Path A: Explicit Configuration via `build.scripts`.
///
/// Supports exclusion globs starting with `!`, strictly matches root documents when no sub-directory
/// prefix is provided, and captures valid formats under sub-directories.
pub(crate) fn resolve_explicit_scripts(project_root: &Path, scripts: &[String]) -> Result<Vec<PathBuf>, String> {
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
pub(crate) fn resolve_fallback_scripts(project_root: &Path) -> Result<Vec<PathBuf>, String> {
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
fn execute_script(script_path: &Path, profile: Option<&str>, resolved_python_cmd: Option<&str>) -> Result<(), String> {
    let parent_dir = script_path.parent().unwrap_or(Path::new(""));
    let ext = script_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let mut cmd = match ext {
        "R" => {
            let mut c = Command::new("Rscript");
            c.arg(script_path);
            c
        },
        "py" => {
            let python_exe = resolved_python_cmd.unwrap_or("python");
            let mut c = Command::new(python_exe);
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
