import re
with open('src/main.rs', 'r') as f:
    content = f.read()

content = content.replace("config.get_path(&root, label)", "config.get_path(root.as_std_path(), label)")

with open('src/main.rs', 'w') as f:
    f.write(content)
