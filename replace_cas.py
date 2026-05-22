import re

with open('src/cas.rs', 'r') as f:
    content = f.read()

content = content.replace('scan_dir(&path, source_root, files)?;', 'scan_dir(&path, source_root, files)?;')

# Fix tests to use Utf8PathBuf
content = content.replace('let file_path = dir.path().join("test.txt");', 'let file_path = Utf8PathBuf::try_from(dir.path().join("test.txt")).unwrap();')
content = content.replace('let hash = hash_file(&file_path).unwrap();', 'let hash = hash_file(&file_path).unwrap();')

content = content.replace('let cas_remote = root.path().join("cas");', 'let cas_remote = Utf8PathBuf::try_from(root.path().join("cas")).unwrap();')
content = content.replace('let proj_root = root.path().join("proj");', 'let proj_root = Utf8PathBuf::try_from(root.path().join("proj")).unwrap();')
content = content.replace('let source_dir = proj_root.join("data").join("raw");', 'let source_dir = proj_root.join("data").join("raw");')

content = content.replace('fs::create_dir_all(&cas_remote).unwrap();', 'fs::create_dir_all(cas_remote.as_std_path()).unwrap();')
content = content.replace('fs::create_dir_all(&source_dir).unwrap();', 'fs::create_dir_all(source_dir.as_std_path()).unwrap();')


# We also need to fix `verify_remote_integrity`
content = content.replace('io::Error::new(io::ErrorKind::NotFound, "No remotes configured")', 'anyhow::anyhow!("No remotes configured")')
content = content.replace('io::Error::new(io::ErrorKind::NotFound, format!("Remote \'{}\' not found", remote_title))', 'anyhow::anyhow!("Remote \'{}\' not found", remote_title)')
content = content.replace('.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?', '?')
content = content.replace('return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid hash found"));', 'anyhow::bail!("Invalid hash found");')
content = content.replace('return Err(io::Error::new(io::ErrorKind::NotFound, format!("Missing file object for hash {}", file_entry.hash)));', 'anyhow::bail!("Missing file object for hash {}", file_entry.hash);')

with open('src/cas.rs', 'w') as f:
    f.write(content)
