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
}

#[derive(Subcommand)]
enum YmlCommands {
    /// Get the projr yml content
    Get,
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
            YmlCommands::Get => {
                let result = proj::yml_get();
                println!("{}", result);
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
