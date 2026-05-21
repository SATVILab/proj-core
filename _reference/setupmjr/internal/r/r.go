package r

import (
	"fmt"
	"path/filepath"

	"github.com/MiguelRodo/setupmjr/internal/sysutil"
)

// SetupR copies or appends to .radian_profile and .lintr in the home directory.
func SetupR(notRadian bool, notLintr bool) error {
	home, err := sysutil.HomeDir()
	if err != nil {
		return err
	}

	if !notRadian {
		radianContent := "options(radian.auto_match = FALSE)\n"
		dstRadian := filepath.Join(home, ".radian_profile")
		if err := sysutil.AppendToFileIfNotExists(dstRadian, radianContent, 0644); err != nil {
			return fmt.Errorf("append to .radian_profile: %w", err)
		}
		fmt.Printf("Configured .radian_profile at %s\n", dstRadian)
	}

	if !notLintr {
		lintrContent := `linters: linters_with_defaults(
    object_length_linter = NULL,
    object_name_linter = NULL)
`
		dstLintr := filepath.Join(home, ".lintr")
		if err := sysutil.AppendToFileIfNotExists(dstLintr, lintrContent, 0644); err != nil {
			return fmt.Errorf("append to .lintr: %w", err)
		}
		fmt.Printf("Configured .lintr at %s\n", dstLintr)
	}

	return nil
}
