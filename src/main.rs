use clap::{Parser, Subcommand};

/// Cross-platform CLI for version-linked project builds.
///
/// This application manages environment variables, validates configuration files (`_proj.yml`),
/// and maintains project versioning across Rust, Python, and R environments.
///
/// # Panics
///
/// Commands might panic if standard streams (`stdout`/`stderr`) fail or if
/// configuration parsing encounters entirely corrupted filesystem states.
///
/// ```rust,ignore
/// use clap::Parser;
/// use proj::{Cli, Commands, VersionCommands};
/// let args = vec!["proj", "version", "get"];
/// let cli = Cli::parse_from(args);
/// ```
#[derive(Parser)]
#[command(name = "proj", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// The top-level commands available in the `proj` CLI.
///
/// This enum routes execution to specific domain logic such as YAML validation,
/// version string manipulation, or path resolution.
///
/// # Errors
///
/// Dispatching subcommands might fail if underlying methods return an `Err`
/// (e.g. unreadable configuration file).
///
/// ```rust,ignore
/// use proj::{Commands, YmlCommands};
/// let cmd = Commands::Yml { command: YmlCommands::Read };
/// ```
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

/// Operations specific to the `_proj.yml` configuration file.
///
/// Allows interacting with the YAML definition of the project, including
/// deep validation and parsing of directory labels.
///
/// # Errors
///
/// May fail if the YAML file contains structurally invalid labels or
/// malformed content.
///
/// ```rust,ignore
/// use proj::YmlCommands;
/// let cmd = YmlCommands::Read;
/// ```
#[derive(Subcommand)]
enum YmlCommands {
    /// Read the _proj.yml content and trigger validation.
    Read,
}

/// Path resolution commands.
///
/// Maps logical directory labels to concrete absolute paths within the current project.
///
/// # Errors
///
/// Might return an error if the project root cannot be found or the specified
/// label does not follow the required prefixed naming convention.
///
/// ```rust,ignore
/// use proj::PathCommands;
/// let cmd = PathCommands::Get { label: "cache-data".to_string() };
/// ```
#[derive(Subcommand)]
enum PathCommands {
    /// Get the resolved path for a given label
    Get {
        /// The label of the directory to get
        #[arg(long)]
        label: String,
    },
}

/// Commands for reading and updating the project version.
///
/// This manages the `VERSION` file, ensuring consistent formatting.
///
/// # Errors
///
/// Can fail if the file is missing, permissions are denied, or an invalid
/// version format is supplied during a `set` operation.
///
/// ```rust,ignore
/// use proj::VersionCommands;
/// let cmd = VersionCommands::Get { no_v: false, format: "text".to_string() };
/// ```
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
