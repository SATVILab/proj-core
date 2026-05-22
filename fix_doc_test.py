import re

with open('src/cas.rs', 'r') as f:
    content = f.read()

content = content.replace(
'''/// let hash = hash_file(file.path()).unwrap();''',
'''/// let path = camino::Utf8Path::from_path(file.path()).unwrap();
/// let hash = hash_file(path).unwrap();'''
)

content = content.replace(
'''use camino::{Utf8Path, Utf8PathBuf};''',
'''use camino::Utf8Path;'''
)

with open('src/cas.rs', 'w') as f:
    f.write(content)
