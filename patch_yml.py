import re
with open('src/yml.rs', 'r') as f:
    content = f.read()

content = content.replace("update_ignores(project_root, &validated)", "update_ignores(camino::Utf8Path::from_path(project_root).unwrap(), &validated)")
content = content.replace("yml_read_from(&project_root, is_dev)", "yml_read_from(project_root.as_std_path(), is_dev)")

with open('src/yml.rs', 'w') as f:
    f.write(content)
