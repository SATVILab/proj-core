import re

with open('src/ignore.rs', 'r') as f:
    content = f.read()

# Fix `root` current_dir conversion
content = content.replace("root_from(&current_dir)", "root_from(camino::Utf8Path::from_path(&current_dir)?)")

with open('src/ignore.rs', 'w') as f:
    f.write(content)
