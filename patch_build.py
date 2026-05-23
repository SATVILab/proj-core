import re
with open('src/build.rs', 'r') as f:
    content = f.read()
# Add the // TODO comment
content = content.replace("check_git_profile(Some(project_root)).map_err(|e| e.to_string())?", "// TODO: migrate to camino\n        check_git_profile(Some(project_root)).map_err(|e| e.to_string())?")
with open('src/build.rs', 'w') as f:
    f.write(content)
