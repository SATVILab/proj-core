package assets

import (
	"embed"
)

//go:embed scripts/* bashrc.d/*
var FS embed.FS
