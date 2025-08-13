mod cli;
mod manifest;
mod packages;
mod store;
mod rollback;
mod config;
mod util;
mod types;

use crate::cli::{Cli, Commands};
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            println!("Building packages...");
            // TODO: call build function here
        }
        Commands::Install => {
            println!("Installing profile...");
            // TODO: call install function here
        }
        Commands::Rollback => {
            println!("Rolling back...");
            // TODO: call rollback function here
        }
        Commands::Status => {
            println!("Showing status...");
            // TODO: call status function here
        }
    }

    Ok(())
}
