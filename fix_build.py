import re

with open('src/build.rs', 'r') as f:
    content = f.read()

content = content.replace("check_git_profile(Some(project_root.as_std_path()))?", "// TODO: migrate to camino\n        check_git_profile(Some(project_root)).map_err(|e| e.to_string())?")
content = content.replace("git_commit_all(\"Snapshot pre-build\", Some(project_root.as_std_path()))?", "// TODO: migrate to camino\n        git_commit_all(\"Snapshot pre-build\", Some(project_root)).map_err(|e| e.to_string())?")
content = content.replace("create_git_provider(config.config.git.engine, project_root.as_std_path().to_path_buf())?", "// TODO: migrate to camino\n                create_git_provider(config.config.git.engine, project_root.to_path_buf()).map_err(|e| e.to_string())?")
content = content.replace("create_git_provider(config.git.engine, repo_dir.into_std_path_buf())?", "create_git_provider(config.git.engine, repo_dir).map_err(|e| e.to_string())?")
content = content.replace("provider.push(\"origin\", \"main\")?", "provider.push(\"origin\", \"main\").map_err(|e| e.to_string())?")
content = content.replace("provider.commit_all(\"chore: automated workspace build update [compiled asset tracking]\")?", "provider.commit_all(\"chore: automated workspace build update [compiled asset tracking]\").map_err(|e| e.to_string())?")
content = content.replace("provider.commit_all(&final_message)?", "provider.commit_all(&final_message).map_err(|e| e.to_string())?")

with open('src/build.rs', 'w') as f:
    f.write(content)

with open('src/build_pre.rs', 'r') as f:
    content = f.read()

content = content.replace("create_git_provider(config.git.engine, repo_dir)?", "// TODO: migrate to camino\n    create_git_provider(config.git.engine, camino::Utf8PathBuf::try_from(repo_dir).unwrap()).map_err(|e| e.to_string())?")
content = content.replace("get_github_token()?", "get_github_token().map_err(|e| e.to_string())?")
content = content.replace("execute_authenticated_git(&[\"fetch\"], t, Some(project_root.as_std_path()))?", "// TODO: migrate to camino\n        execute_authenticated_git(&[\"fetch\"], t, Some(project_root)).map_err(|e| e.to_string())?")

with open('src/build_pre.rs', 'w') as f:
    f.write(content)

with open('src/env.rs', 'r') as f:
    content = f.read()

content = content.replace("base_dir.as_std_path(),", "// TODO: migrate to camino\n                        base_dir,")

with open('src/env.rs', 'w') as f:
    f.write(content)

with open('src/init.rs', 'r') as f:
    content = f.read()

content = content.replace("default_config.validate_and_resolve(&project_root, false).unwrap()", "// TODO: migrate to camino\n        default_config.validate_and_resolve(project_root.as_std_path(), false).unwrap()")
content = content.replace("if let Ok(dir_path) = config.get_path(&project_root, label) {", "// TODO: migrate to camino\n        if let Ok(dir_path) = config.get_path(project_root.as_std_path(), label) {")
content = content.replace("println!(\"Creating directory: {}\", r_dir.display());", "println!(\"Creating directory: {}\", r_dir);")

with open('src/init.rs', 'w') as f:
    f.write(content)

with open('src/yml.rs', 'r') as f:
    content = f.read()

content = content.replace("update_ignores(project_root, &validated)", "// TODO: migrate to camino\n    update_ignores(camino::Utf8Path::from_path(project_root).unwrap(), &validated)")
content = content.replace("yml_read_from(&project_root, is_dev)", "// TODO: migrate to camino\n    yml_read_from(project_root.as_std_path(), is_dev)")

with open('src/yml.rs', 'w') as f:
    f.write(content)

with open('src/main.rs', 'r') as f:
    content = f.read()

content = content.replace("match config.get_path(&root, label) {", "// TODO: migrate to camino\n                    match config.get_path(root.as_std_path(), label) {")

with open('src/main.rs', 'w') as f:
    f.write(content)
