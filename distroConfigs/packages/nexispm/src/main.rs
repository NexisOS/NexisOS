use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "nexis", about = "Declarative system package manager for NexisOS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Rebuild the system according to declarative config
    Rebuild,

    /// Roll back to a previous generation
    Rollback {
        /// Generation number to roll back to; defaults to previous
        generation: Option<u32>,
    },

    /// Garbage collect unused generations
    Gc,

    /// Show current system generation info
    Info,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Rebuild => {
            nexispm::init_store();
            nexispm::rebuild_system();
        }
        Commands::Rollback { generation } => {
            nexispm::rollback(generation);
        }
        Commands::Gc => {
            nexispm::gc();
        }
        Commands::Info => {
            nexispm::show_info();
        }
    }
}
