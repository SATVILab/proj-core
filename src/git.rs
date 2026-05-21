use std::process::Command;
use std::path::PathBuf;
use std::io::Write;

pub trait GitProvider {
    fn is_available(&self) -> bool;
    fn get_user_name(&self) -> Option<String>;
    fn get_user_email(&self) -> Option<String>;
    fn commit_all(&self, message: &str) -> Result<(), String>;
    fn push(&self, remote: &str, branch: &str) -> Result<(), String>;
    fn is_behind_remote(&self, remote: &str, branch: &str) -> Result<bool, String>;
}

pub struct SystemGit {
    repo_path: PathBuf,
}

impl SystemGit {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    // Helper to execute standard git actions safely with uniform string handling
    fn run_cmd(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute system git process: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

impl GitProvider for SystemGit {
    fn is_available(&self) -> bool {
        // Quick check to see if the executable exists in PATH and runs
        Command::new("git").arg("--version").output().is_ok()
    }

    fn get_user_name(&self) -> Option<String> {
        self.run_cmd(&["config", "user.name"]).ok()
    }

    fn get_user_email(&self) -> Option<String> {
        self.run_cmd(&["config", "user.email"]).ok()
    }

    fn commit_all(&self, message: &str) -> Result<(), String> {
        self.run_cmd(&["add", "-A"])?;
        self.run_cmd(&["commit", "-m", message])?;
        Ok(())
    }

    fn push(&self, remote: &str, branch: &str) -> Result<(), String> {
        self.run_cmd(&["push", remote, branch])?;
        Ok(())
    }

    fn is_behind_remote(&self, remote: &str, branch: &str) -> Result<bool, String> {
        // Fetch tracking info silently first
        let _ = self.run_cmd(&["fetch", remote]);

        // Count the commits the local branch is behind the remote tracking branch
        let remote_target = format!("{}/{}", remote, branch);
        let count_str = self.run_cmd(&["rev-list", "--count", &format!("HEAD..{}", remote_target)])?;

        let count: usize = count_str.parse().unwrap_or(0);
        Ok(count > 0)
    }
}

// Unified Factory pattern isolated away from your pipeline logic
pub fn create_git_provider(engine: crate::yml::GitEngine, repo_path: PathBuf) -> Result<Box<dyn GitProvider>, String> {
    let provider = Box::new(SystemGit::new(repo_path));

    match engine {
        crate::yml::GitEngine::System | crate::yml::GitEngine::Auto => {
            if provider.is_available() {
                Ok(provider)
            } else {
                Err("System Git executable could not be resolved in the current environment PATH.".to_string())
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use crate::yml::GitEngine;

    #[test]
    fn test_system_git_fails_gracefully_without_path() {
        // Temporarily clear path variables to simulate an environment missing git
        let original_path = env::var("PATH").unwrap_or_default();
        unsafe { env::set_var("PATH", "") };

        let result = create_git_provider(GitEngine::Auto, PathBuf::from("."));

        // Restore environment safety
        unsafe { env::set_var("PATH", original_path) };

        assert!(result.is_err());
        match result {
            Err(e) => assert_eq!(e, "System Git executable could not be resolved in the current environment PATH."),
            Ok(_) => panic!("Expected an error when git is not in PATH"),
        }
    }
}
