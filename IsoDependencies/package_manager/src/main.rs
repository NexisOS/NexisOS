mod cli;
mod config;
mod manifest;
mod packages;
mod rollback;
mod store;
mod types;

use anyhow::Result;
use cli::{Cli, Commands};
use clap::Parser;
use env_logger::Env;
use log::{debug, info};

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logger — default INFO, switch to DEBUG if --verbose passed
    let log_level = if cli.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(Env::default().default_filter_or(log_level)).init();

    debug!("Verbose logging enabled");
    info!("Starting nexpm");

    match cli.command {
        Commands::Apply { config } => packages::apply(config.as_deref())?,
        Commands::Install { name } => packages::install_single(&name)?,
        Commands::Status => packages::status()?,
        Commands::Rollback { steps } => rollback::rollback(steps)?,
        Commands::ListGenerations => rollback::list_generations()?,
        Commands::DeleteGenerations { ids } => {
            for id in ids {
                rollback::delete_generation(id)?;
            }
        }
    }

    Ok(())
}
