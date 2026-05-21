#!/usr/bin/env bash

# ================================================
# Environment Variables Setup for GitHub and HF
# ================================================
#
# This script sets environment variables for GitHub and Hugging Face credentials.
# Users can optionally define their own credentials by uncommenting and setting the variables below.
# If a variable is not set here, the script will retain any existing environment variable values.
#
# SECURITY NOTICE:
# Avoid hardcoding sensitive information in scripts, especially if the script is stored in version control.
# Consider using secure methods to handle credentials, such as sourcing from a separate, secure `.env` file.

# ============================
# Optional User-Defined Credentials
# ============================

# Uncomment and set your GitHub Username if you want to override the existing GITHUB_USERNAME
# GITHUB_USERNAME="your_github_username"

# Uncomment and set your GitHub Personal Access Token (PAT) if you want to override the existing GH_TOKEN
# Create a classic GitHub PAT here: https://github.com/settings/tokens/new
# GH_TOKEN="your_github_pat"

# Uncomment and set your Hugging Face Token (only needed if using Hugging Face) to override the existing HF_PAT
# HF_PAT="your_huggingface_pat"

# ============================
# Export Environment Variables
# ============================

# Function to export variables only if user-defined variables are provided
export_if_set() {
    local export_name="$1"
    local export_value="$2"

    if [ -n "$export_value" ]; then
        export "$export_name"="$export_value"
    fi
}

# Export GitHub Username
export_if_set "GITHUB_USERNAME" "$GITHUB_USERNAME"
export_if_set "GITHUB_USER" "$GITHUB_USERNAME"

# Export GitHub Token and related variables.
# GH_TOKEN takes precedence if set,
# as we have then manually added it
# and so we want to ensure it is used.
# GITHUB_TOKEN is automatically added by e.g.
# GH cli and often has too few permissions.
export_if_set "GH_TOKEN" "$GH_TOKEN"
export_if_set "GITHUB_TOKEN" "$GH_TOKEN"
export_if_set "GITHUB_PAT" "$GH_TOKEN"

# Export Hugging Face Token
export_if_set "HF_PAT" "$HF_PAT"
export_if_set "HF_TOKEN" "$HF_PAT"
