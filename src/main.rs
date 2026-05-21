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
}

#[derive(Subcommand)]
enum YmlCommands {
    /// Get the projr yml content
    Get,
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
    }
}
