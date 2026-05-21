#!/bin/bash
cat src/main.rs | sed -e '/pub enum Commands {/a\
    /// Init related operations\
    Init {\
        #[command(subcommand)]\
        command: Option<InitCommands>,\
    },' > src/main_patched.rs
mv src/main_patched.rs src/main.rs

cat << 'INNEREOF' >> src/main.rs

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

sed -i 's/        Commands::Version { command } => match command {/        Commands::Init { command } => match command {\n            Some(InitCommands::Version) => {\n                if let Err(e) = proj::init_version() {\n                    eprintln!("Initialization failed: {}", e);\n                }\n            }\n            Some(InitCommands::Directories) => {\n                if let Err(e) = proj::init_directories() {\n                    eprintln!("Initialization failed: {}", e);\n                }\n            }\n            Some(InitCommands::Readme { title, description }) => {\n                if let Err(e) = proj::init_readme(title.clone(), description.clone()) {\n                    eprintln!("Initialization failed: {}", e);\n                }\n            }\n            Some(InitCommands::License { license, first_name, last_name }) => {\n                if let Err(e) = proj::init_license(license.clone(), first_name.clone(), last_name.clone()) {\n                    eprintln!("Initialization failed: {}", e);\n                }\n            }\n            Some(InitCommands::Git { commit }) => {\n                if let Err(e) = proj::init_git(*commit) {\n                    eprintln!("Initialization failed: {}", e);\n                }\n            }\n            Some(InitCommands::Github { public }) => {\n                if let Err(e) = proj::init_github(*public) {\n                    eprintln!("Initialization failed: {}", e);\n                }\n            }\n            None => {\n                if let Err(e) = proj::init_full() {\n                    eprintln!("Initialization failed: {}", e);\n                }\n            }\n        },\n        Commands::Version { command } => match command {/g' src/main.rs

cargo check
