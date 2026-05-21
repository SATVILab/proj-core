use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "proj", version, about = "Cross-platform CLI for version-linked project builds")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// YML related operations
    Yml {
        #[command(subcommand)]
        command: YmlCommands,
    },
    /// Version related operations
    Version {
        #[command(subcommand)]
        command: VersionCommands,
    },
    /// Path related operations
    Path {
        #[command(subcommand)]
        command: PathCommands,
    },
}

#[derive(Subcommand)]
enum YmlCommands {
    /// Read the projr yml content and trigger validation
    Read,
}

#[derive(Subcommand)]
enum PathCommands {
    /// Get the resolved path for a given label
    Get {
        /// The label of the directory to get
        #[arg(long)]
        label: String,
    },
}

#[derive(Subcommand)]
enum VersionCommands {
    /// Get the project version
    Get {
        /// Omit the 'v' prefix
        #[arg(long, default_value_t = false)]
        no_v: bool,

        /// Output format: "text" or "json"
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Set the project version
    Set {
        /// The new version string (e.g. v1.2.3.4, 1.2.3, or JSON)
        version: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Yml { command } => match command {
            YmlCommands::Read => {
                match proj::yml_read() {
                    Ok(_) => println!("Successfully read and validated _proj.yml"),
                    Err(e) => eprintln!("Error reading _proj.yml: {}", e),
                }
            }
        },
        Commands::Path { command } => match command {
            PathCommands::Get { label } => {
                match proj::yml_read() {
                    Ok(config) => {
                        if let Some(root) = proj::root() {
                            match config.get_path(&root, label) {
                                Ok(path) => println!("{}", path.display()),
                                Err(e) => eprintln!("Error getting path: {}", e),
                            }
                        } else {
                            eprintln!("Error finding project root");
                        }
                    }
                    Err(e) => eprintln!("Error loading configuration: {}", e),
                }
            }
        },
        Commands::Version { command } => match command {
            VersionCommands::Get { no_v, format } => {
                if let Some(version) = proj::version_get() {
                    if format.to_lowercase() == "json" {
                        if let Ok(json) = serde_json::to_string(&version) {
                            println!("{}", json);
                        } else {
                            eprintln!("Failed to format version as JSON");
                        }
                    } else {
                        println!("{}", version.to_string(!no_v));
                    }
                } else {
                    eprintln!("Failed to get version from VERSION file");
                }
            }
            VersionCommands::Set { version } => {
                if let Err(e) = proj::version_set(version) {
                    eprintln!("Error setting version: {}", e);
                }
            }
        },
    }
}
