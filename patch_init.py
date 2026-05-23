import re
with open('src/init.rs', 'r') as f:
    content = f.read()

content = content.replace("default_config.validate_and_resolve(&project_root, false).unwrap()", "default_config.validate_and_resolve(project_root.as_std_path(), false).unwrap()")
content = content.replace("config.get_path(&project_root, label)", "config.get_path(project_root.as_std_path(), label)")
content = content.replace("r_dir.display()", "r_dir.as_str()")

with open('src/init.rs', 'w') as f:
    f.write(content)
