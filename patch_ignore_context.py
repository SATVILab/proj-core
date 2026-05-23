import re

with open('src/ignore.rs', 'r') as f:
    content = f.read()

# Make sure anyhow strings are sentence-cased inside any map_err if any left
content = re.sub(r'\.map_err\(\|e\| format!\("Failed to ([^"]+): \{\}", e\)\)', r'.context("Failed to \1")', content)

with open('src/ignore.rs', 'w') as f:
    f.write(content)
