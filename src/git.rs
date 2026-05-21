use std::process::Command;
use std::io::Write;

pub trait GitProvider {
    fn is_available(&self) -> bool;
    fn get_user_name(&self) -> Result<String, String>;
    fn get_user_email(&self) -> Result<String, String>;
    fn commit_all(&self, message: &str) -> Result<(), String>;
    fn push(&self) -> Result<(), String>;
    fn is_behind_remote(&self) -> Result<bool, String>;
}

pub struct SystemGit {
    pub repo_path: std::path::PathBuf,
    pub token: Option<String>,
}

impl GitProvider for SystemGit {
    fn is_available(&self) -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    fn get_user_name(&self) -> Result<String, String> {
        let mut cmd = Command::new("git");
        cmd.args(["config", "--get", "user.name"]);
        cmd.current_dir(&self.repo_path);
        let output = cmd.output()
            .map_err(|e| format!("Failed to execute git config check for user.name: {}", e))?;

        if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim().is_empty() {
            return Err("Git configuration missing: 'user.name' is not set.\n\
                 Please configure it using: git config --global user.name \"Your Value\"".to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_user_email(&self) -> Result<String, String> {
        let mut cmd = Command::new("git");
        cmd.args(["config", "--get", "user.email"]);
        cmd.current_dir(&self.repo_path);
        let output = cmd.output()
            .map_err(|e| format!("Failed to execute git config check for user.email: {}", e))?;

        if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim().is_empty() {
            return Err("Git configuration missing: 'user.email' is not set.\n\
                 Please configure it using: git config --global user.email \"Your Value\"".to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn commit_all(&self, message: &str) -> Result<(), String> {
        // git add -A
        let mut add_cmd = Command::new("git");
        add_cmd.args(["add", "-A"]);
        add_cmd.current_dir(&self.repo_path);

        let add_output = add_cmd.output()
            .map_err(|e| format!("Failed to execute 'git add -A': {}", e))?;

        if !add_output.status.success() {
            return Err(format!("'git add -A' failed: {}", String::from_utf8_lossy(&add_output.stderr)));
        }

        // git commit -m message
        let mut commit_cmd = Command::new("git");
        commit_cmd.args(["commit", "-m", message]);
        commit_cmd.current_dir(&self.repo_path);

        let commit_output = commit_cmd.output()
            .map_err(|e| format!("Failed to execute 'git commit': {}", e))?;

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

    fn push(&self) -> Result<(), String> {
        if let Some(t) = &self.token {
            execute_authenticated_git(&["push"], t, Some(&self.repo_path))
        } else {
            let mut push_cmd = Command::new("git");
            push_cmd.arg("push");
            push_cmd.current_dir(&self.repo_path);

            let output = push_cmd.output()
                .map_err(|e| format!("Failed to execute 'git push': {}", e))?;

            if !output.status.success() {
                return Err(format!("'git push' failed: {}", String::from_utf8_lossy(&output.stderr)));
            }

            Ok(())
        }
    }

    fn is_behind_remote(&self) -> Result<bool, String> {
        // Perform fetch
        if let Some(t) = &self.token {
            execute_authenticated_git(&["fetch"], t, Some(&self.repo_path))?;
        } else {
            let mut fetch_cmd = Command::new("git");
            fetch_cmd.args(["fetch"]);
            fetch_cmd.current_dir(&self.repo_path);
            let fetch_out = fetch_cmd.output().map_err(|e| format!("Failed to fetch from remote: {}", e))?;
            if !fetch_out.status.success() {
                return Err(format!("Failed to fetch from remote: {}", String::from_utf8_lossy(&fetch_out.stderr)));
            }
        }

        // Check rev-list --count HEAD..@{u}
        let mut cmd = Command::new("git");
        cmd.args(["rev-list", "--count", "HEAD..@{u}"]);
        cmd.current_dir(&self.repo_path);
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
}

pub struct GixEngine {
    pub repo_path: std::path::PathBuf,
    pub token: Option<String>,
}

impl GitProvider for GixEngine {
    fn is_available(&self) -> bool {
        // gix is embedded in the Rust binary, so it's always "available"
        true
    }

    fn get_user_name(&self) -> Result<String, String> {
        let repo = gix::open(&self.repo_path)
            .map_err(|e| format!("Failed to open repo: {}", e))?;
        let config = repo.config_snapshot();
        if let Some(name) = config.string("user.name") {
            let name_str = name.to_string();
            if !name_str.trim().is_empty() {
                return Ok(name_str);
            }
        }
        Err("Git configuration missing: 'user.name' is not set.\n\
                 Please configure it using: git config --global user.name \"Your Value\"".to_string())
    }

    fn get_user_email(&self) -> Result<String, String> {
        let repo = gix::open(&self.repo_path)
            .map_err(|e| format!("Failed to open repo: {}", e))?;
        let config = repo.config_snapshot();
        if let Some(email) = config.string("user.email") {
            let email_str = email.to_string();
            if !email_str.trim().is_empty() {
                return Ok(email_str);
            }
        }
        Err("Git configuration missing: 'user.email' is not set.\n\
                 Please configure it using: git config --global user.email \"Your Value\"".to_string())
    }

    fn commit_all(&self, _message: &str) -> Result<(), String> {
        // Gix implementation for commit all
        // Because a full pure-gix implementation for 'add -A' and 'commit' is complex and requires
        // constructing an index tree, writing the tree to the odb, creating the commit,
        // and updating refs, and because `gix` crate is currently heavily geared towards read operations,
        // we'll attempt a minimal gix implementation. However, if system git is requested to be NOT used,
        // we strictly implement it using gix.

        // Let's implement pure gix add-and-commit here.
        Err("Gix pure Rust commit_all is not currently supported natively due to lack of high-level tree writing API in gix. A system git fallback is required or wait for gix updates.".to_string())
    }

    fn push(&self) -> Result<(), String> {
        // gix currently has very limited and low-level support for pushing,
        // and configuring HTTPS authentication is very involved.
        Err("Gix pure Rust push is not currently supported natively due to missing high-level remote push API. A system git fallback is required.".to_string())
    }

    fn is_behind_remote(&self) -> Result<bool, String> {
        let repo = gix::open(&self.repo_path)
            .map_err(|e| format!("Failed to open repo: {}", e))?;

        // Gix does not yet have a high-level API for network operations like "fetch".
        // Checking if behind a remote purely locally:
        // We find the HEAD commit and the tracking branch commit.

        let head = repo.head().map_err(|e| e.to_string())?;
        let _head_commit = head.into_peeled_id().map_err(|e| e.to_string())?;

        let _head_ref = repo.head_ref().map_err(|e| e.to_string())?.ok_or_else(|| "HEAD is detached".to_string())?;

        // This is pure local check, assuming fetch has happened, but gix network fetch is complex.
        Err("Gix pure Rust is_behind_remote is not fully supported for network fetch. A system git fallback is required.".to_string())
    }
}

use crate::yml::GitEngine;

pub fn create_git_provider(repo_path: &std::path::Path, engine: &GitEngine, token: Option<String>) -> Result<Box<dyn GitProvider>, String> {
    match engine {
        GitEngine::System => {
            let provider = SystemGit { repo_path: repo_path.to_path_buf(), token };
            if provider.is_available() {
                Ok(Box::new(provider))
            } else {
                Err("System Git is specified but not found on the system PATH.".to_string())
            }
        }
        GitEngine::Gix => {
            Ok(Box::new(GixEngine { repo_path: repo_path.to_path_buf(), token }))
        }
        GitEngine::Auto => {
            let system_provider = SystemGit { repo_path: repo_path.to_path_buf(), token: token.clone() };
            if system_provider.is_available() {
                Ok(Box::new(system_provider))
            } else {
                Ok(Box::new(GixEngine { repo_path: repo_path.to_path_buf(), token }))
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
    use crate::yml::GitEngine;

    // Helper to mock PATH to simulate missing git
    fn test_with_mock_path<F>(func: F)
    where
        F: FnOnce(),
    {
        // When checking is_available we are simply spawning 'git'. Since rust's Command searches PATH,
        // we can set PATH to an empty or non-existent path so that it cannot find the 'git' executable.
        let original_path = std::env::var("PATH").unwrap_or_default();
        unsafe { std::env::set_var("PATH", ""); }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            func();
        }));

        unsafe { std::env::set_var("PATH", original_path); }

        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    #[test]
    fn test_create_git_provider_auto_fallback() {
        test_with_mock_path(|| {
            let provider = create_git_provider(std::path::Path::new("."), &GitEngine::Auto, None).unwrap();

            // Should fallback to GixEngine since system git is not available in mock PATH
            // Note: Since GixEngine doesn't have a distinct struct we can easily downcast in test without any_downcast,
            // we can verify the behavior indirectly (it shouldn't return SystemGit which isn't available).
            // Actually, SystemGit checks `is_available()` via `git --version`, which will fail if git isn't on PATH.
            // If it returns Ok, it means Auto successfully fell back to GixEngine.
            assert!(provider.is_available(), "Provider should be available even when system git is missing (fallback to Gix)");
        });
    }

    #[test]
    fn test_create_git_provider_system_fails_without_git() {
        test_with_mock_path(|| {
            let result = create_git_provider(std::path::Path::new("."), &GitEngine::System, None);
            assert!(result.is_err(), "System engine should fail explicitly if git is missing");
        });
    }
}
