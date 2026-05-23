import re

with open('src/build_pre.rs', 'r') as f:
    content = f.read()

content = re.sub(r'create_git_provider\(config\.git\.engine, repo_dir\)\?', r'create_git_provider(config.git.engine, camino::Utf8PathBuf::try_from(repo_dir).unwrap()).map_err(|e| e.to_string())?', content)

content = re.sub(r'get_github_token\(\)\?', r'get_github_token().map_err(|e| e.to_string())?', content)
content = re.sub(r'execute_authenticated_git\(&\["fetch"\], t, Some\(project_root\.as_std_path\(\)\)\)\?', r'execute_authenticated_git(&["fetch"], t, Some(project_root)).map_err(|e| e.to_string())?', content)

with open('src/build_pre.rs', 'w') as f:
    f.write(content)
