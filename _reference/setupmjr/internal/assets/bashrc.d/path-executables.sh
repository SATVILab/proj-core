#!/usr/bin/env bash

# This script ensures that ~/bin and ~/.local/bin are included in the user's PATH.
# It checks if these directories are in the PATH and adds them if they are not.

PATH="${PATH:-}"

for dir in "$HOME/bin" "$HOME/.local/bin"; do
  case ":$PATH:" in
    *":$dir:"*) ;;
    *) export PATH="$dir:$PATH" ;;
  esac
done
