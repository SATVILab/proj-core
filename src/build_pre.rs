use std::path::PathBuf;
use std::process::Command;
use std::io::{self, BufRead, IsTerminal};
use crate::git::{get_github_token, execute_authenticated_git, create_git_provider};
use crate::yml::GlobalConfig;

pub fn pre_flight_git_check(config: &GlobalConfig, repo_dir: PathBuf) -> Result<(), String> {
    let provider = create_git_provider(config.git.engine, repo_dir)?;

    // Check if user has context configurations mapped out
    let name = provider.get_user_name().unwrap_or_else(|| "Unknown".to_string());
    let email = provider.get_user_email().unwrap_or_else(|| "unknown@example.com".to_string());

    println!("Git context verified successfully for R workflow: {} <{}>", name, email);

    if provider.is_behind_remote("origin", "main").unwrap_or(false) {
        println!("⚠️ Warning: Current local HEAD is behind origin/main.");
    }

    Ok(())
}

/// Extracts the YAML frontmatter sequence from the beginning of a document.
pub fn extract_frontmatter(content: &str) -> Option<String> {
    let mut lines = content.lines();
    if let Some(first) = lines.next() {
        if first.trim() != "---" {
            return None;
        }
    } else {
        return None;
    }

    let mut frontmatter = String::new();
    for line in lines {
        if line.trim() == "---" {
            return Some(frontmatter);
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }
    None
}

/// Parses the frontmatter to extract the target format and output-file keys.
pub fn parse_frontmatter_options(content: &str) -> (Option<String>, Option<String>) {
    let mut format = None;
    let mut output_file = None;

    if let Some(fm) = extract_frontmatter(content) {
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&fm) {
            if let Some(map) = yaml.as_mapping() {
                if let Some(f) = map.get("format").and_then(|v| v.as_str()) {
                    format = Some(f.to_string());
                } else if let Some(o) = map.get("output") {
                    if let Some(s) = o.as_str() {
                        format = Some(s.to_string());
                    } else if let Some(omap) = o.as_mapping() {
                        if let Some(k) = omap.keys().next() {
                            if let Some(s) = k.as_str() {
                                format = Some(s.to_string());
                            }
                        }
                    }
                }

                if let Some(of) = map.get("output-file").or_else(|| map.get("output_file")) {
                    if let Some(s) = of.as_str() {
                        output_file = Some(s.to_string());
                    }
                }
            }
        }
    }

    (format, output_file)
}

/// Maps a document format to its output file extension.
pub fn map_format_to_extension(format: Option<&str>) -> String {
    let format_str = format.unwrap_or("html");
    match format_str {
        "html_notebook" => "nb.html".to_string(),
        "word_document" => "docx".to_string(),
        "beamer_presentation" | "typst" => "pdf".to_string(),
        _ => "html".to_string()
    }
}

pub fn find_python_command() -> Option<String> {
    let variants = ["python3", "python"];
    for cmd in variants {
        if let Ok(output) = Command::new(cmd).arg("--version").output() {
            if output.status.success() {
                return Some(cmd.to_string());
            }
        }
    }
    None
}

use crate::yml::ValidatedConfig;

pub fn run_pre_flight_checks(
    project_root: &std::path::Path,
    config: &ValidatedConfig,
    is_prod_run: bool,
    resolved_files: &[PathBuf],
    quarto_exists: bool,
    bookdown_exists: bool,
) -> Result<(Option<String>, Option<String>), String> {
    let mut resolved_token = None;
    let needs_remote = config.git.push || (is_prod_run && config.restrictions.not_behind == Some(true)) || (is_prod_run && config.restrictions.not_behind.is_none());

    if needs_remote && config.config.git.use_proj_cred_helper {
        let token = get_github_token()?;
        resolved_token = Some(token);
    }

    if is_prod_run {
        let is_git_repo = project_root.join(".git").exists();

        // Run the new pre_flight_git_check if it's a git repo and git is configured
        if is_git_repo {
            pre_flight_git_check(&config.config, project_root.to_path_buf())?;
        }

        let mut current_branch = None;
        if is_git_repo {
            let mut cmd = Command::new("git");
            cmd.args(["symbolic-ref", "--short", "HEAD"]);
            cmd.current_dir(project_root);
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    current_branch = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
        }

        // Branch constraints
        if let Some(only_branches) = &config.restrictions.only_branches {
            if !is_git_repo {
                return Err("Error: build.restrictions.only_branches is specified, but the project is not inside a Git repository.".to_string());
            }
            if let Some(branch) = &current_branch {
                if !only_branches.contains(branch) {
                    return Err(format!("Error: Output builds are restricted to branches {:?} as per build.restrictions.only_branches. Current branch is '{}'.", only_branches, branch));
                }
            } else {
                return Err("Error: build.restrictions.only_branches is specified, but could not determine current branch.".to_string());
            }
        }

        if let Some(not_branches) = &config.restrictions.not_branches {
            if let Some(branch) = &current_branch {
                if not_branches.contains(branch) {
                    return Err(format!("Error: Output builds are restricted on branch '{}' as per build.restrictions.not_branches.", branch));
                }
            }
        }

        // Upstream synchronicity
        let not_behind = config.restrictions.not_behind;
        if not_behind != Some(false) {
            if not_behind == Some(true) && (!is_git_repo || !has_tracking_remote(project_root)) {
                return Err("Error: build.restrictions.not_behind is explicitly set to true, but no Git remote is configured.".to_string());
            }

            if is_git_repo {
                if !has_tracking_remote(project_root) {
                    if not_behind.is_none() {
                        return Err("Error: Git repository detected but no upstream tracking remote is configured. Configure a remote or set build.restrictions.not_behind to false.".to_string());
                    }
                } else {
                    if is_behind_remote(project_root, resolved_token.as_deref())? {
                        return Err("Error: The local branch is behind its tracking remote. Please pull or merge changes before building.".to_string());
                    }
                }
            }
        }
    }

    let mut r_needed = false;
    let mut quarto_needed = quarto_exists;
    let mut python_needed = false;
    let mut rmd_present = false;
    if bookdown_exists {
        r_needed = true;
    }

    for file in resolved_files {
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "Rmd" | "rmd" => {
                r_needed = true;
                rmd_present = true;
            }
            "qmd" => {
                quarto_needed = true;
            }
            "R" | "r" => {
                r_needed = true;
            }
            "py" => {
                python_needed = true;
            }
            _ => {}
        }
    }

    // A. R Validation
    if r_needed {
        if Command::new("Rscript").arg("--version").output().is_err() {
            return Err("Error: 'Rscript' executable not found on the system PATH. A working R installation is required to build this project.".to_string());
        }

        let mut required_packages = Vec::new();
        if bookdown_exists {
            required_packages.push("bookdown");
        } else if rmd_present {
            required_packages.push("rmarkdown");
        }

        for package_name in required_packages {
            let status = Command::new("Rscript")
                .args(&[
                    "-e",
                    &format!("if (!requireNamespace('{}', quietly = TRUE)) q(status = 1)", package_name),
                ])
                .status();

            let is_installed = status.map(|s| s.success()).unwrap_or(false);

            if !is_installed {
                let auto_install = std::env::var("PROJR_AUTO_INSTALL").unwrap_or_default();
                let should_install = if auto_install == "true" {
                    true
                } else if auto_install == "false" {
                    false
                } else {
                    let is_terminal = io::stdin().is_terminal();
                    if is_terminal {
                        let mut input = String::new();
                        let stdin = io::stdin();
                        loop {
                            println!("Required R package '{}' is not installed. Would you like to install it now? (y/n): ", package_name);
                            input.clear();
                            match stdin.lock().read_line(&mut input) {
                                Ok(0) => break false, // EOF
                                Ok(_) => {
                                    let trimmed = input.trim().to_lowercase();
                                    if trimmed == "y" || trimmed == "yes" {
                                        break true;
                                    } else if trimmed == "n" || trimmed == "no" {
                                        break false;
                                    }
                                }
                                Err(_) => break false,
                            }
                        }
                    } else {
                        false
                    }
                };

                if should_install {
                    let install_status = Command::new("Rscript")
                        .args(&[
                            "-e",
                            &format!("install.packages('{}', repos='https://cloud.r-project.org')", package_name),
                        ])
                        .status()
                        .map_err(|e| format!("Failed to execute Rscript for installation: {}", e))?;

                    if !install_status.success() {
                        return Err(format!("Error: Failed to install R package '{}'.", package_name));
                    }
                } else {
                    return Err(format!(
                        "Error: Required R package '{}' is not installed.\n\
                        To install this dependency programmatically, run:\n    \
                        Rscript -e \"install.packages('{}', repos='https://cloud.r-project.org')\"",
                        package_name, package_name
                    ));
                }
            }
        }
    }

    // B. Quarto Validation
    if quarto_needed {
        if Command::new("quarto").arg("--version").output().is_err() {
            return Err("Error: 'quarto' binary not found on the system PATH. \n\
                        Please download and install Quarto before proceeding: https://quarto.org/docs/get-started/".to_string());
        }
    }

    // C. Python Validation
    let mut resolved_python_cmd = None;
    if python_needed {
        match find_python_command() {
            Some(cmd) => {
                resolved_python_cmd = Some(cmd);
            }
            None => {
                return Err("Error: Python interpreter not found on the system PATH. \
                            Please ensure either 'python3' or 'python' is installed and accessible.".to_string());
            }
        }
    }

    Ok((resolved_token, resolved_python_cmd))
}

fn has_tracking_remote(project_root: &std::path::Path) -> bool {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
    cmd.current_dir(project_root);
    if let Ok(output) = cmd.output() {
        output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
    } else {
        false
    }
}

fn is_behind_remote(project_root: &std::path::Path, token: Option<&str>) -> Result<bool, String> {
    // Perform fetch
    if let Some(t) = token {
        execute_authenticated_git(&["fetch"], t, Some(project_root))?;
    } else {
        let mut fetch_cmd = Command::new("git");
        fetch_cmd.args(["fetch"]);
        fetch_cmd.current_dir(project_root);
        let fetch_out = fetch_cmd.output().map_err(|e| format!("Failed to fetch from remote: {}", e))?;
        if !fetch_out.status.success() {
            return Err(format!("Failed to fetch from remote: {}", String::from_utf8_lossy(&fetch_out.stderr)));
        }
    }

    // Check rev-list --count HEAD..@{u}
    let mut cmd = Command::new("git");
    cmd.args(["rev-list", "--count", "HEAD..@{u}"]);
    cmd.current_dir(project_root);
    let output = cmd.output().map_err(|e| format!("Failed to check if behind remote: {}", e))?;

    if output.status.success() {
        let count_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(count) = count_str.trim().parse::<u32>() {
            return Ok(count > 0);
        }
    }

    // If the above fails (e.g., no upstream configured, though we check it prior), assume not behind or return error
    Err("Failed to determine if the local branch is behind the remote.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let content = "---\ntitle: abc\nformat: html_notebook\n---\nBody";
        assert_eq!(extract_frontmatter(content).unwrap(), "title: abc\nformat: html_notebook\n");

        let content2 = "No frontmatter";
        assert!(extract_frontmatter(content2).is_none());
    }

    #[test]
    fn test_parse_frontmatter_options() {
        let content = "---\ntitle: abc\nformat: html_notebook\noutput-file: out.html\n---\nBody";
        let (fmt, of) = parse_frontmatter_options(content);
        assert_eq!(fmt, Some("html_notebook".to_string()));
        assert_eq!(of, Some("out.html".to_string()));

        let content_rmd = "---\noutput:\n  word_document: default\noutput_file: test.docx\n---\nBody";
        let (fmt, of) = parse_frontmatter_options(content_rmd);
        assert_eq!(fmt, Some("word_document".to_string()));
        assert_eq!(of, Some("test.docx".to_string()));
    }
}
