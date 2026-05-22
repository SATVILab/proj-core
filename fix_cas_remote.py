with open('src/cas.rs', 'r') as f:
    content = f.read()

# In verify_remote_integrity, remote.path is std::path::PathBuf or similar, so we need to convert it or its children.
# But remote.path comes from `ValidatedConfig`. Let's just fix the usage of `as_std_path()` on PathBuf.
content = content.replace('fs::read_dir(manifests_dir.as_std_path())', 'fs::read_dir(manifests_dir.as_std_path())')
# Oh wait, `manifests_dir` is `remote.path.join("manifests")`.
# We shouldn't use `as_std_path()` if it's already a `std::path::PathBuf`.
content = content.replace('fs::read_dir(manifests_dir.as_std_path())?', 'fs::read_dir(manifests_dir)?')
content = content.replace('fs::read_to_string(manifest_path.as_std_path())?', 'fs::read_to_string(manifest_path)?')

with open('src/cas.rs', 'w') as f:
    f.write(content)
