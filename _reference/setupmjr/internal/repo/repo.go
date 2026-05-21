package repo

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/MiguelRodo/setupmjr/internal/sysutil"
)

func getLatestGitHubReleaseTag(ownerRepo string) (string, error) {
	url := fmt.Sprintf("https://api.github.com/repos/%s/releases/latest", ownerRepo)
	resp, err := http.Get(url)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("failed to fetch latest release for %s: %s", ownerRepo, resp.Status)
	}

	var release struct {
		TagName string `json:"tag_name"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&release); err != nil {
		return "", err
	}

	return release.TagName, nil
}

func SetupRepoReadme() error {
	reader := bufio.NewReader(os.Stdin)

	fmt.Print("Enter title: ")
	title, _ := reader.ReadString('\n')
	title = strings.TrimSpace(title)

	fmt.Print("Complete this sentence describing the purpose of the repo: This repository... and we'll ensure a single white space after this repository: ")
	purpose, _ := reader.ReadString('\n')
	purpose = strings.TrimSpace(purpose)

	fmt.Print("Enter Contact 1 Full Name: ")
	name1, _ := reader.ReadString('\n')
	name1 = strings.TrimSpace(name1)

	fmt.Print("Enter Contact 1 Email: ")
	email1, _ := reader.ReadString('\n')
	email1 = strings.TrimSpace(email1)

	fmt.Print("Enter Contact 2 Full Name: ")
	name2, _ := reader.ReadString('\n')
	name2 = strings.TrimSpace(name2)

	fmt.Print("Enter Contact 2 Email: ")
	email2, _ := reader.ReadString('\n')
	email2 = strings.TrimSpace(email2)

	readmeContent := fmt.Sprintf(`# %s

This repository %s

## Contact

For more information, please contact:
- %s:
  - %s
- %s:
  - %s

## Links

- [URLs to data sources (e.g. OneDrive), GitHub repositories, publications, etc.]

## Details

[Methods, timeline, team, data sources, software/tools, etc.]
`, title, purpose, name1, email1, name2, email2)

	err := os.WriteFile("README.md", []byte(readmeContent), 0644)
	if err != nil {
		return fmt.Errorf("failed to write README.md: %w", err)
	}

	fmt.Println("Successfully created README.md")
	return nil
}

func SetupRepoDevcontainer(repo, branch string, build bool) error {
	if repo == "" {
		repo = "MiguelRodo/comp"
	}
	if branch == "" {
		branch = "main"
	}

	tarUrl := fmt.Sprintf("https://github.com/%s/archive/refs/heads/%s.tar.gz", repo, branch)
	repoNameParts := strings.Split(repo, "/")
	if len(repoNameParts) != 2 {
		return fmt.Errorf("invalid repo format, expected owner/name")
	}
	repoDirName := repoNameParts[1] + "-" + branch

	cmdStr := fmt.Sprintf("curl -sL %s | tar xz --strip-components=1 -C . \"%s/.devcontainer\"", tarUrl, repoDirName)
	cmd := exec.Command("sh", "-c", cmdStr)

	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("failed to download .devcontainer from %s: %v, output: %s", repo, err, string(output))
	}

	if build {
		if err := sysutil.EnsureDirExists(".github/workflows", 0755); err != nil {
			return err
		}
		tag, err := getLatestGitHubReleaseTag("MiguelRodo/actions")
		if err != nil {
			return fmt.Errorf("failed to fetch latest actions release: %w", err)
		}
		url := fmt.Sprintf("https://raw.githubusercontent.com/MiguelRodo/actions/%s/examples/prebuild-devcontainer.yml", tag)
		dest := filepath.Join(".github", "workflows", "prebuild-devcontainer.yml")
		if err := downloadFile(url, dest); err != nil {
			return fmt.Errorf("failed to download prebuild-devcontainer.yml: %w", err)
		}
		fmt.Println("Successfully downloaded prebuild-devcontainer.yml")
	}

	fmt.Println("Successfully copied .devcontainer")
	return nil
}

func SetupRepoAction(actionName string) error {
	if err := sysutil.EnsureDirExists(".github/workflows", 0755); err != nil {
		return err
	}

	dest := filepath.Join(".github", "workflows", actionName+".yml")

	if content, exists := actionWorkflows[actionName]; exists {
		if err := os.WriteFile(dest, []byte(content), 0644); err != nil {
			return fmt.Errorf("failed to write action %s: %w", actionName, err)
		}
	} else {
		url := fmt.Sprintf("https://raw.githubusercontent.com/MiguelRodo/actions/main/examples/%s.yml", actionName)
		if err := downloadFile(url, dest); err != nil {
			return fmt.Errorf("failed to download action %s: %w", actionName, err)
		}
	}

	fmt.Printf("Successfully added %s to .github/workflows\n", actionName)
	return nil
}

func RunReposCommand(args []string) error {
	reposCmd := "repos"
	_, err := exec.LookPath(reposCmd)
	if err != nil {
		home, errHome := sysutil.HomeDir()
		if errHome == nil {
			localRepos := filepath.Join(home, ".local", "bin", "repos")
			if runtime.GOOS == "windows" {
				localRepos += ".exe"
			}
			if _, errStat := os.Stat(localRepos); errStat == nil {
				reposCmd = localRepos
			} else {
				fmt.Println("repos command not found. Installing...")
				if errInstall := SetupRepoInstallRepos(); errInstall != nil {
					return fmt.Errorf("failed to install repos: %w", errInstall)
				}
				reposCmd = localRepos
			}
		} else {
			return fmt.Errorf("repos not found and could not install automatically")
		}
	}

	cmd := exec.Command(reposCmd, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Stdin = os.Stdin

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("repos command failed: %w", err)
	}
	return nil
}

func SetupRepoInstallRepos() error {
	tag, err := getLatestGitHubReleaseTag("MiguelRodo/repos")
	if err != nil {
		return fmt.Errorf("failed to fetch latest repos release: %w", err)
	}
	version := strings.TrimPrefix(tag, "v")

	goos := runtime.GOOS
	goarch := runtime.GOARCH

	var ext string
	if goos == "windows" {
		ext = "zip"
	} else if goos == "linux" {
		ext = "tar.gz" // Safer default for all Linux distros
	} else {
		ext = "tar.gz"
	}

	assetName := fmt.Sprintf("repos_%s_%s_%s.%s", version, goos, goarch, ext)

	url := fmt.Sprintf("https://github.com/MiguelRodo/repos/releases/download/v%s/%s", version, assetName)

	tmpDir, err := os.MkdirTemp("", "setupmjr-repos-*")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tmpDir)

	tmpFile := filepath.Join(tmpDir, assetName)
	if err := downloadFile(url, tmpFile); err != nil {
		return fmt.Errorf("failed to download %s: %w", url, err)
	}

	home, err := sysutil.HomeDir()
	if err != nil {
		return err
	}

	binDir := filepath.Join(home, ".local", "bin")
	if err := sysutil.EnsureDirExists(binDir, 0755); err != nil {
		return err
	}

	destPath := filepath.Join(binDir, "repos")
	if goos == "windows" {
		destPath += ".exe"
	}

	if ext == "tar.gz" {
		cmd := exec.Command("tar", "xzf", tmpFile, "-C", tmpDir)
		if err := cmd.Run(); err != nil {
			return fmt.Errorf("tar extraction failed: %w", err)
		}

		extractedBin := filepath.Join(tmpDir, "repos")
		if err := os.Rename(extractedBin, destPath); err != nil {
			return fmt.Errorf("failed to move binary to %s: %w", destPath, err)
		}
	} else if ext == "zip" {
		cmd := exec.Command("unzip", "-o", tmpFile, "-d", tmpDir)
		if err := cmd.Run(); err != nil {
			return fmt.Errorf("unzip failed: %w", err)
		}
		extractedBin := filepath.Join(tmpDir, "repos.exe")
		if err := os.Rename(extractedBin, destPath); err != nil {
			return fmt.Errorf("failed to move binary to %s: %w", destPath, err)
		}
	}

	if err := os.Chmod(destPath, 0755); err != nil {
		return fmt.Errorf("failed to make repos executable: %w", err)
	}

	fmt.Printf("Successfully installed repos to %s\n", destPath)
	return nil
}

func downloadFile(url, dest string) error {
	resp, err := http.Get(url)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("bad status: %s", resp.Status)
	}

	out, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, resp.Body)
	return err
}
