#!/bin/bash
sed -i 's/std::path::PathBuf/camino::Utf8PathBuf/g' src/ignore.rs
sed -i 's/std::path::Path/camino::Utf8Path/g' src/ignore.rs
sed -i 's/Option<PathBuf>/Option<camino::Utf8PathBuf>/g' src/ignore.rs

# Let's clean up duplicate imports if any
sed -i 's/use camino::Utf8PathBuf;//g' src/ignore.rs
sed -i '1i use camino::{Utf8PathBuf, Utf8Path};' src/ignore.rs

# Fix root_from path traversal
