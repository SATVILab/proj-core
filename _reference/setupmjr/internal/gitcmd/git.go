package gitcmd

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/MiguelRodo/setupmjr/internal/shell"
	"github.com/MiguelRodo/setupmjr/internal/sysutil"
)

// RunCommand runs a command and returns its standard output and error.
func RunCommand(name string, args ...string) (string, error) {
	cmd := exec.Command(name, args...)
	out, err := cmd.Output()
	return strings.TrimSpace(string(out)), err
}

func SetupGitProfile() error {
	name, _ := RunCommand("git", "config", "--global", "user.name")
	email, _ := RunCommand("git", "config", "--global", "user.email")

	reader := bufio.NewReader(os.Stdin)

	if name == "" {
		fmt.Print("Enter Git user.name: ")
		nameInput, _ := reader.ReadString('\n')
		nameInput = strings.TrimSpace(nameInput)
		if nameInput != "" {
			if _, err := RunCommand("git", "config", "--global", "user.name", nameInput); err != nil {
				return fmt.Errorf("set git user.name: %w", err)
			}
			fmt.Printf("Set git user.name to %s\n", nameInput)
		}
	} else {
		fmt.Printf("Git user.name already set to %s\n", name)
	}

	if email == "" {
		fmt.Print("Enter Git user.email: ")
		emailInput, _ := reader.ReadString('\n')
		emailInput = strings.TrimSpace(emailInput)
		if emailInput != "" {
			if _, err := RunCommand("git", "config", "--global", "user.email", emailInput); err != nil {
				return fmt.Errorf("set git user.email: %w", err)
			}
			fmt.Printf("Set git user.email to %s\n", emailInput)
		}
	} else {
		fmt.Printf("Git user.email already set to %s\n", email)
	}

	return nil
}

func SetupGitAuthText() error {
	helperScript := `!f() { \
        sleep 1; \
        echo username="${GITHUB_USER:-TOKEN}"; \
        echo password="${GH_TOKEN:-$GITHUB_TOKEN}"; \
        }; f`

	cmd := exec.Command("git", "config", "--global", "credential.https://github.com.helper", helperScript)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("configure github credential helper: %w", err)
	}
	fmt.Println("Configured git credential helper for https://github.com")

	hfHelperScript := `!f() { \
        sleep 1; \
        echo username="${HF_USER:-${HUGGINGFACE_USER:-TOKEN}}"; \
        echo password="${HF_TOKEN:-$HUGGINGFACE_TOKEN}"; \
        }; f`

	hfCmd := exec.Command("git", "config", "--global", "credential.https://huggingface.co.helper", hfHelperScript)
	if err := hfCmd.Run(); err != nil {
		return fmt.Errorf("configure huggingface credential helper: %w", err)
	}
	fmt.Println("Configured git credential helper for https://huggingface.co")

	reader := bufio.NewReader(os.Stdin)

	fmt.Print("Enter GitHub username: ")
	user, _ := reader.ReadString('\n')
	user = strings.TrimSpace(user)

	fmt.Print("Enter GitHub token: ")
	token, _ := reader.ReadString('\n')
	token = strings.TrimSpace(token)

	home, err := sysutil.HomeDir()
	if err != nil {
		return err
	}

	if err := shell.SetupShellRCD("bash"); err != nil {
		return err
	}

	if err := shell.SetupShellPath("bash"); err != nil {
		return err
	}

	// Make sure login.sh exists
	if err := shell.SetupShellLogin("bash"); err != nil {
		return err
	}

	if err := shell.SetupShellRCD("zsh"); err != nil {
		return err
	}
	if err := shell.SetupShellPath("zsh"); err != nil {
		return err
	}
	if err := shell.SetupShellLogin("zsh"); err != nil {
		return err
	}

	bashLoginPath := filepath.Join(home, ".bashrc.d", "login.sh")
	zshLoginPath := filepath.Join(home, ".zshrc.d", "login.sh")

	for _, loginPath := range []string{bashLoginPath, zshLoginPath} {
		if user != "" || token != "" {
			b, err := os.ReadFile(loginPath)
			if err != nil {
				return fmt.Errorf("read %s: %w", loginPath, err)
			}
			content := string(b)

			if user != "" {
				content = injectVar(content, "GITHUB_USERNAME", user)
				content = injectVar(content, "GITHUB_USER", user)
			}
			if token != "" {
				content = injectVar(content, "GH_TOKEN", token)
				content = injectVar(content, "GITHUB_TOKEN", token)
				content = injectVar(content, "GITHUB_PAT", token)
			}

			if err := os.WriteFile(loginPath, []byte(content), 0755); err != nil {
				return fmt.Errorf("write %s: %w", loginPath, err)
			}
			fmt.Printf("Injected credentials into %s\n", loginPath)
		}
	}

	return nil
}

func injectVar(content, varName, val string) string {
	pattern := regexp.MustCompile(`(?m)^#?\s*` + regexp.QuoteMeta(varName) + `=.*$`)
	replacement := fmt.Sprintf(`%s="%s"`, varName, val)

	if pattern.MatchString(content) {
		return pattern.ReplaceAllString(content, replacement)
	}

	// If not found, append it
	return content + "\n" + replacement + "\n"
}

func SetupGitAuthCache() error {
	if _, err := RunCommand("git", "config", "--global", "credential.helper", "cache"); err != nil {
		return fmt.Errorf("set credential.helper cache: %w", err)
	}
	fmt.Println("Configured git to use cache credential helper")
	return nil
}

func SetupGitAuthMngr() error {
	// Simple approach: we'll just try to set 'manager'
	if _, err := RunCommand("git", "config", "--global", "credential.helper", "manager"); err != nil {
		fmt.Printf("Warning: failed to set manager credential helper: %v\n", err)
	} else {
		fmt.Println("Configured git to use manager credential helper")
	}
	return nil
}

func SetupGitAuth() error {
	if err := SetupGitAuthText(); err != nil {
		return err
	}
	if err := SetupGitAuthCache(); err != nil {
		return err
	}
	if err := SetupGitAuthMngr(); err != nil {
		return err
	}
	return nil
}

func SetupGit() error {
	if err := SetupGitProfile(); err != nil {
		return err
	}
	if err := SetupGitAuth(); err != nil {
		return err
	}
	return nil
}
