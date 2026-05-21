#!/bin/bash

# Patch init_readme
sed -i 's/pub fn init_readme() -> Result<(), String> {/pub fn init_readme(title_opt: Option<String>, description_opt: Option<String>) -> Result<(), String> {/' src/init.rs
sed -i 's/let title = ask_string("Project Title", Some("My Project"));/let title = title_opt.unwrap_or_else(|| ask_string("Project Title", Some("My Project")));/' src/init.rs
sed -i 's/let description = ask_string("Project Description", Some("A projr project."));/let description = description_opt.unwrap_or_else(|| ask_string("Project Description", Some("A projr project.")));/' src/init.rs

# Patch init_license
sed -i 's/pub fn init_license() -> Result<(), String> {/pub fn init_license(license_opt: Option<String>, first_name_opt: Option<String>, last_name_opt: Option<String>) -> Result<(), String> {/' src/init.rs
sed -i 's/let choice = ask_choice("Choose a license", &\["ccby", "apache", "cc0", "proprietary"\], Some("ccby"));/let choice = license_opt.unwrap_or_else(|| ask_choice("Choose a license", \&["ccby", "apache", "cc0", "proprietary"], Some("ccby")));/' src/init.rs
sed -i 's/let first_name = ask_string("First Name", None);/let first_name = first_name_opt.unwrap_or_else(|| ask_string("First Name", None));/' src/init.rs
sed -i 's/let last_name = ask_string("Last Name", None);/let last_name = last_name_opt.unwrap_or_else(|| ask_string("Last Name", None));/' src/init.rs

# Patch init_git
sed -i 's/pub fn init_git() -> Result<(), String> {/pub fn init_git(commit_opt: Option<bool>) -> Result<(), String> {/' src/init.rs
sed -i 's/if is_new && ask_yes_no("Commit initial changes?", true) {/let should_commit = commit_opt.unwrap_or_else(|| ask_yes_no("Commit initial changes?", true));\n    if is_new \&\& should_commit {/' src/init.rs

# Patch init_github
sed -i 's/pub fn init_github() -> Result<(), String> {/pub fn init_github(public_opt: Option<bool>) -> Result<(), String> {/' src/init.rs
sed -i 's/let visibility = ask_choice("Visibility", &\["public", "private"\], Some("private"));/let is_public = public_opt.unwrap_or_else(|| ask_choice("Visibility", \&["public", "private"], Some("private")) == "public");/' src/init.rs
sed -i '/let is_public = visibility == "public";/d' src/init.rs

# Patch init_full
sed -i 's/init_readme()?;/init_readme(None, None)?;/g' src/init.rs
sed -i 's/init_license()?;/init_license(None, None, None)?;/g' src/init.rs
sed -i 's/init_git()?;/init_git(None)?;/g' src/init.rs
sed -i 's/init_github()?;/init_github(None)?;/g' src/init.rs

cargo check
