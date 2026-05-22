import re

with open('src/cas.rs', 'r') as f:
    content = f.read()

content = content.replace(
'''#[cfg(test)]
mod tests {
    use super::*;''',
'''#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;'''
)

with open('src/cas.rs', 'w') as f:
    f.write(content)
