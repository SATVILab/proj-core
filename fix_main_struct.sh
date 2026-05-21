#!/bin/bash
cat src/main.rs | sed -e '/pub enum InitCommands {/,$d' | sed -e '/#\[derive(Subcommand)\]/,$d' > src/main_patched.rs

cat << 'INNEREOF' >> src/main_patched.rs
/// Operations for initializing a project.
///
/// Sets up the project structure, versions, git, github, etc.
///
/// # Errors
///
/// May fail if an initialization step encounters an error (e.g., IO, permission).
/// ```rust
/// // use proj::InitCommands;
/// ```
#[derive(Subcommand)]
pub enum InitCommands {
    /// Initialize version
    Version,
    /// Initialize directories
    Directories,
    /// Initialize README
    Readme {
        /// Project Title
        #[arg(long)]
        title: Option<String>,
        /// Project Description
        #[arg(long)]
        description: Option<String>,
    },
    /// Initialize License
    License {
        /// License type (ccby, apache, cc0, proprietary)
        #[arg(long)]
        license: Option<String>,
        /// First name (for proprietary)
        #[arg(long)]
        first_name: Option<String>,
        /// Last name (for proprietary)
        #[arg(long)]
        last_name: Option<String>,
    },
    /// Initialize Git
    Git {
        /// Whether to commit initial changes
        #[arg(long)]
        commit: Option<bool>,
    },
    /// Initialize GitHub
    Github {
        /// Make repository public (default is private)
        #[arg(long)]
        public: Option<bool>,
    },
}
INNEREOF
mv src/main_patched.rs src/main.rs

sed -i 's/            Some(InitCommands::Readme) => {/            Some(InitCommands::Readme { title: _, description: _ }) => {/g' src/main.rs
sed -i 's/            Some(InitCommands::License) => {/            Some(InitCommands::License { license: _, first_name: _, last_name: _ }) => {/g' src/main.rs
sed -i 's/            Some(InitCommands::Git) => {/            Some(InitCommands::Git { commit: _ }) => {/g' src/main.rs
sed -i 's/            Some(InitCommands::Github) => {/            Some(InitCommands::Github { public: _ }) => {/g' src/main.rs

cargo check
