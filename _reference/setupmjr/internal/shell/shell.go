package shell

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/MiguelRodo/setupmjr/internal/assets"
	"github.com/MiguelRodo/setupmjr/internal/sysutil"
)

// SetupShellRCD ensures ~/.<shell>rc.d is sourced by adding sourcing block to ~/.<shell>rc.
func SetupShellRCD(shellName string) error {
	home, err := sysutil.HomeDir()
	if err != nil {
		return err
	}

	rcFile := fmt.Sprintf(".%src", shellName)
	rcDir := fmt.Sprintf(".%src.d", shellName)

	// 1. Add sourcing block to ~/.<shell>rc
	rcPath := filepath.Join(home, rcFile)
	sourcingBlock := fmt.Sprintf(`for i in $HOME/%s/*; do [ -r "$i" ] && source "$i"; done`, rcDir)

	b, err := os.ReadFile(rcPath)
	if err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("read %s: %w", rcFile, err)
	}

	hasRCD := false
	for _, line := range strings.Split(string(b), "\n") {
		trimmed := strings.TrimSpace(line)
		if !strings.HasPrefix(trimmed, "#") && strings.Contains(trimmed, rcDir) {
			hasRCD = true
			break
		}
	}

	if !hasRCD {
		f, err := os.OpenFile(rcPath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
		if err != nil {
			return fmt.Errorf("open %s: %w", rcFile, err)
		}
		defer f.Close()

		if _, err := f.WriteString("\n" + sourcingBlock + "\n"); err != nil {
			return fmt.Errorf("write to %s: %w", rcFile, err)
		}
		fmt.Printf("Added %s sourcing block to ~/%s\n", rcDir, rcFile)
	} else {
		fmt.Printf("~/%s already sources %s\n", rcFile, rcDir)
	}

	// 2. Ensure ~/.<shell>rc.d exists
	rcdPath := filepath.Join(home, rcDir)
	if err := sysutil.EnsureDirExists(rcdPath, 0755); err != nil {
		return err
	}

	return nil
}

// SetupShellPath copies path-executables.sh to ~/.<shell>rc.d.
func SetupShellPath(shellName string) error {
	if err := SetupShellRCD(shellName); err != nil {
		return err
	}

	home, err := sysutil.HomeDir()
	if err != nil {
		return err
	}

	rcDir := fmt.Sprintf(".%src.d", shellName)
	rcdPath := filepath.Join(home, rcDir)
	dst := filepath.Join(rcdPath, "path-executables.sh")

	// Assuming bashrc.d acts as the generic template source
	if err := sysutil.CopyEmbedFile(assets.FS, "bashrc.d/path-executables.sh", dst, 0755); err != nil {
		return fmt.Errorf("copy path-executables.sh: %w", err)
	}
	fmt.Printf("Copied path-executables.sh to %s\n", dst)

	return nil
}

// SetupShellLogin copies login.sh to ~/.<shell>rc.d.
func SetupShellLogin(shellName string) error {
	home, err := sysutil.HomeDir()
	if err != nil {
		return err
	}

	rcDir := fmt.Sprintf(".%src.d", shellName)
	rcdPath := filepath.Join(home, rcDir)
	if err := sysutil.EnsureDirExists(rcdPath, 0755); err != nil {
		return err
	}

	dst := filepath.Join(rcdPath, "login.sh")
	// Check if login.sh already exists, if so skip overwriting it
	if _, err := os.Stat(dst); err == nil {
		fmt.Printf("login.sh already exists at %s, skipping copy\n", dst)
		return nil
	}

	if err := sysutil.CopyEmbedFile(assets.FS, "bashrc.d/login.sh", dst, 0755); err != nil {
		return fmt.Errorf("copy login.sh: %w", err)
	}
	fmt.Printf("Copied login.sh to %s\n", dst)

	return nil
}
