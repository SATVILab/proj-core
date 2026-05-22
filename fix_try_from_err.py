with open('src/cas.rs', 'r') as f:
    content = f.read()

# Fix the issues from cargo check

# `verify_remote_integrity`: remote.path is std::path::PathBuf in yml.rs or cas.rs?
# Let's fix the fs::read_dir(dir.as_std_path()) block in scan_dir
content = content.replace('''        if dir.is_dir() {
            for entry in fs::read_dir(dir.as_std_path())? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, source_root, files)?;
                } else {
                    let hash = hash_file(&path)?;
                    // Calculate relative path
                    let rel_path = path.strip_prefix(source_root)?.to_string();

                    // Standardize path separators to forward slashes
                    let rel_path = rel_path.replace("\\\\", "/");

                    files.push(FileEntry { path: rel_path, hash });
                }
            }
        }''', '''        if dir.is_dir() {
            for entry in fs::read_dir(dir.as_std_path())? {
                let entry = entry?;
                let path = camino::Utf8PathBuf::try_from(entry.path())
                    .map_err(|_| anyhow::anyhow!("File system path is not valid UTF-8: {:?}", entry.path()))?;

                if path.is_dir() {
                    scan_dir(&path, source_root, files)?;
                } else {
                    let hash = hash_file(&path)?;
                    // Calculate relative path
                    let rel_path = path.strip_prefix(source_root)?.to_string();

                    // Standardize path separators to forward slashes
                    let rel_path = rel_path.replace("\\\\", "/");

                    files.push(FileEntry { path: rel_path, hash });
                }
            }
        }''')


with open('src/cas.rs', 'w') as f:
    f.write(content)
