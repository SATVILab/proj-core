use std::process::Command;
use std::io::Write;

pub fn get_github_token() -> Result<String, String> {
    // Track A: Environment Context Inspection
    let env_vars = ["GITHUB_PAT", "GH_TOKEN", "GITHUB_TOKEN"];
    for var in &env_vars {
        if let Ok(val) = std::env::var(var) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
    }

    // Track B: System Git Credential Helper Interrogation
    let mut child = Command::new("git")
        .args(["credential", "fill"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn git credential fill: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"protocol=https\nhost=github.com\n\n");
    }

    let output = child.wait_with_output().map_err(|e| format!("Failed to wait on git credential fill: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(password) = line.strip_prefix("password=") {
                let trimmed = password.trim();
                if !trimmed.is_empty() {
                    return Ok(trimmed.to_string());
                }
            }
        }
    }

    // Track C: GitHub CLI Engine Query
    if let Ok(output) = Command::new("gh").args(["auth", "token"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let trimmed = stdout.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
    }

    // Output structured error if token is missing
    Err("Error: GitHub authentication token not found.
Authentication is required to interact with remote repositories via git push or fetch.

To resolve this, please execute one of the following options:
Option A (Environment Variable):
    Set the GITHUB_PAT environment variable in your active terminal profile.
Option B (GitHub CLI):
    Install the 'gh' utility and run 'gh auth login' to authenticate your host.
Option C (Git Helper Setup):
    Approve host access directly inside your local system credential helper:
    git credential approve < echo -e \"protocol=https\\nhost=github.com\\nusername=user\\npassword=YOUR_PAT\"".to_string())
}

pub fn execute_authenticated_git(args: &[&str], token: &str, current_dir: Option<&std::path::Path>) -> Result<(), String> {
    // Construct an inline script helper string that Git will execute to read the password.
    // Git credential helpers expect output formatted as key=value lines.
    let inline_helper = format!("!f() {{ echo \"password={}\"; }}; f", token);

    let mut cmd = Command::new("git");

    // Wipe out standard global credential helpers for this command context only
    cmd.arg("-c")
       .arg(format!("credential.helper={}", inline_helper))
       // Bind environment to fail fast instead of freezing on interactive prompts
       .env("GIT_TERMINAL_PROMPT", "0")
       // Append target arguments, e.g., ["push", "origin", "main"] or ["fetch"]
       .args(args);

    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }

    let status = cmd.status().map_err(|e| format!("Failed to execute system Git subprocess: {}", e))?;

    match status {
        s if s.success() => Ok(()),
        s => Err(format!("Git command exited with failure status code: {}", s)),
    }
}

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
pub fn check_git_profile(current_dir: Option<&std::path::Path>) -> Result<(), String> {
    let check_config = |key: &str| -> Result<(), String> {
        let mut cmd = Command::new("git");
        cmd.args(["config", "--get", key]);
        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }
        let output = cmd.output()
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
pub fn git_commit_all(message: &str, current_dir: Option<&std::path::Path>) -> Result<(), String> {
    // git add -A
    let mut add_cmd = Command::new("git");
    add_cmd.args(["add", "-A"]);
    if let Some(dir) = current_dir {
        add_cmd.current_dir(dir);
    }

    let add_output = add_cmd.output()
        .map_err(|e| format!("Failed to execute 'git add -A': {}", e))?;

    if !add_output.status.success() {
        return Err(format!("'git add -A' failed: {}", String::from_utf8_lossy(&add_output.stderr)));
    }

    // git commit -m message
    let mut commit_cmd = Command::new("git");
    commit_cmd.args(["commit", "-m", message]);
    if let Some(dir) = current_dir {
        commit_cmd.current_dir(dir);
    }

    let commit_output = commit_cmd.output()
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
pub fn git_push(current_dir: Option<&std::path::Path>, token: Option<&str>) -> Result<(), String> {
    if let Some(t) = token {
        execute_authenticated_git(&["push"], t, current_dir)
    } else {
        let mut push_cmd = Command::new("git");
        push_cmd.arg("push");
        if let Some(dir) = current_dir {
            push_cmd.current_dir(dir);
        }

        let output = push_cmd.output()
            .map_err(|e| format!("Failed to execute 'git push': {}", e))?;

        if !output.status.success() {
            return Err(format!("'git push' failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }
}
