import re

with open('src/git.rs', 'r') as f:
    content = f.read()

content = content.replace("PathBuf::from(\".\")", "camino::Utf8PathBuf::from(\".\")")
content = content.replace("assert_eq!(e, \"System Git executable could not be resolved in the current environment PATH.\"),", "assert_eq!(e.to_string(), \"System Git executable could not be resolved in the current environment PATH.\"),")

with open('src/git.rs', 'w') as f:
    f.write(content)
