with open('src/build.rs', 'r') as f:
    content = f.read()

old_str = """                                        crate::cas::ingest_directory(
                                            project_root,
                                            &remote.path,
                                            tag,
                                            &dir_path,
                                            &initial_version.to_string(false)
                                        ).map_err(|e| format!("Failed remote CAS export for {}: {}", tag, e))?;"""

new_str = """                                        crate::cas::ingest_directory(
                                            camino::Utf8Path::from_path(project_root).ok_or_else(|| format!("Invalid utf8 path"))?,
                                            camino::Utf8Path::from_path(&remote.path).ok_or_else(|| format!("Invalid utf8 path"))?,
                                            tag,
                                            camino::Utf8Path::from_path(&dir_path).ok_or_else(|| format!("Invalid utf8 path"))?,
                                            &initial_version.to_string(false)
                                        ).map_err(|e| format!("Failed remote CAS export for {}: {}", tag, e))?;"""

if old_str in content:
    content = content.replace(old_str, new_str)
else:
    print("Could not find the string to replace")

with open('src/build.rs', 'w') as f:
    f.write(content)
