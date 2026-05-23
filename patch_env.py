import re
with open('src/env.rs', 'r') as f:
    content = f.read()

content = content.replace("base_dir.as_std_path(),", "base_dir,")

with open('src/env.rs', 'w') as f:
    f.write(content)
