use std::process::Command;

/// Returns true if executing `git --version` succeeds.
pub fn is_git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Queries local, global, and system variables via `git config --get user.name` and `git config --get user.email`.
/// If either stdout buffer returns blank or throws an error, return an explicit error string detailing exactly what config parameter is missing and how the user can configure it.
pub fn check_git_profile() -> Result<(), String> {
    let check_config = |key: &str| -> Result<(), String> {
        let output = Command::new("git")
            .args(["config", "--get", key])
            .output()
            .map_err(|e| format!("Failed to execute git config check for {}: {}", key, e))?;

        if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim().is_empty() {
            return Err(format!(
                "Git configuration missing: '{}' is not set.\n\
                 Please configure it using: git config --global {} \"Your Value\"",
                key, key
            ));
        }
        Ok(())
    };

    check_config("user.name")?;
    check_config("user.email")?;

    Ok(())
}

/// Runs `git add -A` to stage modified and untracked changes.
/// Runs `git commit -m "<message>"`. Squelch errors gracefully if there are no modifications staged to be committed.
pub fn git_commit_all(message: &str) -> Result<(), String> {
    // git add -A
    let add_output = Command::new("git")
        .args(["add", "-A"])
        .output()
        .map_err(|e| format!("Failed to execute 'git add -A': {}", e))?;

    if !add_output.status.success() {
        return Err(format!("'git add -A' failed: {}", String::from_utf8_lossy(&add_output.stderr)));
    }

    // git commit -m message
    let commit_output = Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .map_err(|e| format!("Failed to execute 'git commit': {}", e))?;

    // Squelch errors gracefully if there are no modifications staged to be committed
    let stdout = String::from_utf8_lossy(&commit_output.stdout);
    let stderr = String::from_utf8_lossy(&commit_output.stderr);

    if !commit_output.status.success() {
        let combined_output = format!("{}\n{}", stdout, stderr).to_lowercase();
        // Check for typical "nothing to commit" messages
        if combined_output.contains("nothing to commit") ||
           combined_output.contains("working tree clean") ||
           combined_output.contains("no changes added to commit") {
            return Ok(()); // Squelch error
        }
        return Err(format!("'git commit' failed: {}", stderr));
    }

    Ok(())
}

/// Runs `git push` to upload tracking offsets upstream.
pub fn git_push() -> Result<(), String> {
    let output = Command::new("git")
        .arg("push")
        .output()
        .map_err(|e| format!("Failed to execute 'git push': {}", e))?;

    if !output.status.success() {
        return Err(format!("'git push' failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(())
}
