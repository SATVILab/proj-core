package sysutil

import (
	"os"

	"golang.org/x/term"
)

// IsInteractive returns true if standard input is a terminal.
func IsInteractive() bool {
	return term.IsTerminal(int(os.Stdin.Fd()))
}
