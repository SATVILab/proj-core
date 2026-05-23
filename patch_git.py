with open('src/git.rs', 'r') as f:
    content = f.read()

content = content.replace("repo_path: PathBuf,", "repo_path: camino::Utf8PathBuf,")
content = content.replace("pub fn new(repo_path: PathBuf) -> Self {", "pub fn new(repo_path: camino::Utf8PathBuf) -> Self {")
content = content.replace("pub fn create_git_provider(engine: crate::yml::GitEngine, repo_path: PathBuf) -> anyhow::Result<Box<dyn GitProvider>> {", "pub fn create_git_provider(engine: crate::yml::GitEngine, repo_path: camino::Utf8PathBuf) -> anyhow::Result<Box<dyn GitProvider>> {")
content = content.replace("use std::path::PathBuf;", "use std::path::PathBuf;\nuse camino::Utf8PathBuf;")

with open('src/git.rs', 'w') as f:
    f.write(content)
