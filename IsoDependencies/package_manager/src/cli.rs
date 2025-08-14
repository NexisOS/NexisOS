use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "nexpm", version, about = "NexisOS package manager")]
pub struct Cli {
    /// Enable verbose (debug) logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Apply the declarative config (installs all packages in config)
    Apply {
        /// Path to config TOML (defaults to /etc/package_manager/config.toml)
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Imperatively install a single package by name from config
    Install {
        /// Name of the package to install
        name: String,
    },

    /// Show current state / generations
    Status,

    /// Roll back N steps (default 1)
    Rollback {
        /// Number of generations to roll back
        #[arg(long, default_value_t = 1)]
        steps: u32,
    },

    /// Delete specific generations to free space
    DeleteGenerations {
        /// List of generation IDs to delete
        #[arg(name = "id")]
        ids: Vec<u64>,
    },

    /// List all available generations and their metadata
    ListGenerations,
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
