with open('src/git.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
skip = 0
for i, line in enumerate(lines):
    if skip > 0:
        skip -= 1
        continue
    if "return Err(format!(" in line and "Git configuration missing" in lines[i+1]:
        new_lines.append('            anyhow::bail!(\n')
        new_lines.append('                "Git configuration missing: \'{}\' is not set.\\nPlease configure it using: git config --global {} \\"Your Value\\"",\n')
        new_lines.append('                key, key\n')
        new_lines.append('            );\n')
        skip = 4 # Skip original `return Err(format!(`, `"Git config..."`, `Please configure...`, `key, key`, `));`
    else:
        new_lines.append(line)

with open('src/git.rs', 'w') as f:
    f.writelines(new_lines)
