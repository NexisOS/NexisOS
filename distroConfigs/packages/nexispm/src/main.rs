use clap::Parser;
use nexispm::cli::{Cli, Commands};
use nexispm::utils::logging;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    logging::init()?;
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Build(args) => nexispm::cli::commands::build::execute(args).await,
        Commands::Switch(args) => nexispm::cli::commands::switch::execute(args).await,
        Commands::Rollback(args) => nexispm::cli::commands::rollback::execute(args).await,
        Commands::Gc(args) => nexispm::cli::commands::gc::execute(args).await,
        // ... other commands
    }
}
