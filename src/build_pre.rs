use std::path::PathBuf;
use std::process::Command;
use std::io::{self, BufRead, IsTerminal};

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

pub fn run_pre_flight_checks(
    resolved_files: &[PathBuf],
    quarto_exists: bool,
    bookdown_exists: bool,
) -> Result<Option<String>, String> {
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

    Ok(resolved_python_cmd)
}
