#!/bin/bash
sed -i 's/Result<(), String>/anyhow::Result<()>/g' src/git.rs
sed -i 's/Result<bool, String>/anyhow::Result<bool>/g' src/git.rs
sed -i 's/Result<String, String>/anyhow::Result<String>/g' src/git.rs
sed -i 's/Result<Box<dyn GitProvider>, String>/anyhow::Result<Box<dyn GitProvider>>/g' src/git.rs
sed -i 's/std::path::PathBuf/camino::Utf8PathBuf/g' src/git.rs
sed -i 's/Option<&std::path::Path>/Option<\&camino::Utf8Path>/g' src/git.rs
