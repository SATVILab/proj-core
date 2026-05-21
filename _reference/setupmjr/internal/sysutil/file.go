package sysutil

import (
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

// CopyEmbedFile copies a file from an embedded filesystem to the local disk.
func CopyEmbedFile(vfs fs.FS, src string, dst string, mode os.FileMode) error {
	srcFile, err := vfs.Open(src)
	if err != nil {
		return fmt.Errorf("open embedded file %s: %w", src, err)
	}
	defer srcFile.Close()

	if err := os.MkdirAll(filepath.Dir(dst), 0755); err != nil {
		return fmt.Errorf("create parent directory for %s: %w", dst, err)
	}

	dstFile, err := os.OpenFile(dst, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, mode)
	if err != nil {
		return fmt.Errorf("create destination file %s: %w", dst, err)
	}
	defer dstFile.Close()

	if _, err := io.Copy(dstFile, srcFile); err != nil {
		return fmt.Errorf("copy %s to %s: %w", src, dst, err)
	}

	return nil
}

// AppendToFile appends text to a file, creating it if it doesn't exist.
// It also ensures the file has the specified text; if not, it appends it.
func AppendToFileIfNotExists(dst string, content string, mode os.FileMode) error {
	if err := os.MkdirAll(filepath.Dir(dst), 0755); err != nil {
		return fmt.Errorf("create parent directory for %s: %w", dst, err)
	}

	existingContent, err := os.ReadFile(dst)
	if err == nil {
		if strings.Contains(string(existingContent), content) {
			return nil
		}
	}

	dstFile, err := os.OpenFile(dst, os.O_CREATE|os.O_APPEND|os.O_WRONLY, mode)
	if err != nil {
		return fmt.Errorf("open destination file %s: %w", dst, err)
	}
	defer dstFile.Close()

	// If file exists and doesn't end with a newline, add one before appending
	if len(existingContent) > 0 && existingContent[len(existingContent)-1] != '\n' {
		if _, err := dstFile.WriteString("\n"); err != nil {
			return fmt.Errorf("write newline to %s: %w", dst, err)
		}
	}

	if _, err := dstFile.WriteString(content); err != nil {
		return fmt.Errorf("append to %s: %w", dst, err)
	}

	return nil
}

// EnsureDirExists ensures that a directory exists, creating it if necessary.
func EnsureDirExists(dir string, mode os.FileMode) error {
	if err := os.MkdirAll(dir, mode); err != nil {
		return fmt.Errorf("create directory %s: %w", dir, err)
	}
	return nil
}

// HomeDir returns the user's home directory.
func HomeDir() (string, error) {
	return os.UserHomeDir()
}
