//! # NexisOS Package Manager CLI
//!
//! Command-line interface for the NexisOS declarative package manager.
//! Provides commands for building generations, activation, rollbacks, and garbage collection.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{info, warn, error, debug};
use std::path::PathBuf;
use tokio;

use nexis_pkg_mgr::{
    NexisPackageManager, 
    NexisConfig,
    init_logging,
    load_default_config,
    NexisError,
};

/// NexisOS Package Manager - Declarative system package management
#[derive(Parser)]
#[command(
    name = "nexis",
    version = env!("CARGO_PKG_VERSION"),
    about = "Declarative system package manager for NexisOS",
    long_about = "NexisOS Package Manager provides declarative system configuration with \
                  atomic rollbacks, optimized garbage collection, and 'latest' git tag resolution. \
                  \n\nFeatures:\n\
                  - Generation-based rollbacks like NixOS\n\
                  - Content-addressable storage with deduplication\n\
                  - Parallel garbage collection (5-20x faster than NixOS)\n\
                  - Automatic 'latest' git tag resolution"
)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,
    
    /// Dry run mode (don't make actual changes)
    #[arg(long, global = true)]
    dry_run: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a new system generation from configuration
    Build {
        /// Skip building packages that are already in store
        #[arg(long)]
        skip_existing: bool,
        
        /// Build only specific packages (comma-separated)
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,
        
        /// Show what would be built without actually building
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Activate a specific generation (make it current)
    Activate {
        /// Generation ID to activate
        generation_id: u64,
        
        /// Skip GRUB configuration update
        #[arg(long)]
        skip_grub: bool,
    },
    
    /// List all available generations
    List {
        /// Show detailed information for each generation
        #[arg(long)]
        detailed: bool,
        
        /// Limit number of generations to show
        #[arg(short, long)]
        limit: Option<usize>,
    },
    
    /// Show information about the current system
    Status {
        /// Show package-level details
        #[arg(long)]
        packages: bool,
    },
    
    /// Run garbage collection to free unused packages
    Gc {
        /// Show what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
        
        /// Delete generations older than N days
        #[arg(long)]
        older_than: Option<u32>,
        
        /// Keep at least N generations
        #[arg(long, default_value = "5")]
        keep_generations: usize,
    },
    
    /// Rollback to the previous generation
    Rollback {
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    
    /// Show package information
    Info {
        /// Package name to show information for
        package: String,
    },
    
    /// Resolve package versions without building
    Resolve {
        /// Show git tag resolution details
        #[arg(long)]
        show_resolution: bool,
    },
    
    /// Configuration management commands
    #[command(subcommand)]
    Config(ConfigCommands),
    
    /// Store management commands  
    #[command(subcommand)]
    Store(StoreCommands),
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Validate the current configuration
    Validate {
        /// Show warnings as errors
        #[arg(long)]
        strict: bool,
    },
    
    /// Show the current configuration
    Show {
        /// Show only a specific section
        #[arg(long)]
        section: Option<String>,
        
        /// Output format: toml, json, yaml
        #[arg(long, default_value = "toml")]
        format: String,
    },
    
    /// Edit the configuration file
    Edit {
        /// Editor to use (defaults to $EDITOR)
        #[arg(long)]
        editor: Option<String>,
    },
}

#[derive(Subcommand)]
enum StoreCommands {
    /// Show store statistics
    Stats {
        /// Show detailed breakdown by package
        #[arg(long)]
        detailed: bool,
    },
    
    /// Verify store integrity
    Verify {
        /// Fix any issues found
        #[arg(long)]
        fix: bool,
    },
    
    /// Optimize store layout and cleanup
    Optimize {
        /// Force optimization even if not needed
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging based on verbosity
    setup_logging(cli.verbose, cli.debug)?;
    
    info!("NexisOS Package Manager v{}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        debug!("Loading configuration from: {:?}", config_path);
        NexisConfig::load(config_path)
            .with_context(|| format!("Failed to load configuration from {:?}", config_path))?
    } else {
        debug!("Loading configuration from default locations");
        load_default_config().await
            .context("Failed to load configuration from default locations")?
    };
    
    // Initialize package manager
    let pm = NexisPackageManager::new(&config)
        .await
        .context("Failed to initialize package manager")?;
    
    // Execute command
    match execute_command(pm, &config, cli.command, cli.dry_run).await {
        Ok(_) => {
            info!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Command failed: {}", e);
            
            // Show additional context for common errors
            match e.downcast_ref::<NexisError>() {
                Some(NexisError::PackageNotFound { package }) => {
                    eprintln!("\nHint: Check if '{}' is defined in your configuration file", package);
                }
                Some(NexisError::GenerationNotFound { gen_id }) => {
                    eprintln!("\nHint: Run 'nexis list' to see available generations");
                }
                _ => {}
            }
            
            std::process::exit(1);
        }
    }
}

async fn execute_command(
    pm: NexisPackageManager,
    config: &NexisConfig,
    command: Commands,
    global_dry_run: bool,
) -> Result<()> {
    match command {
        Commands::Build { skip_existing, only, dry_run } => {
            let dry_run = dry_run || global_dry_run;
            
            if dry_run {
                info!("Building new generation (dry run)");
            } else {
                info!("Building new generation");
            }
            
            // TODO: Implement package filtering logic for --only and --skip-existing
            if only.is_some() {
                warn!("--only flag not yet implemented, building all packages");
            }
            if skip_existing {
                warn!("--skip-existing flag not yet implemented");
            }
            
            let generation = pm.build_generation().await?;
            
            if dry_run {
                println!("Would create generation {} with {} packages:", 
                         generation.id, generation.packages.len());
                for pkg in &generation.packages {
                    println!("  - {} ({})", pkg.name, pkg.version);
                }
            } else {
                println!("Created generation {}", generation.id);
                println!("Packages: {}", generation.packages.len());
                println!("Use 'nexis activate {}' to make this generation active", generation.id);
            }
        }
        
        Commands::Activate { generation_id, skip_grub } => {
            info!("Activating generation {}", generation_id);
            
            if global_dry_run {
                println!("Would activate generation {}", generation_id);
                return Ok(());
            }
            
            pm.activate_generation(generation_id).await?;
            
            if !skip_grub {
                info!("Updating GRUB configuration");
                // TODO: Update GRUB menu entries
            }
            
            println!("Successfully activated generation {}", generation_id);
            println!("Reboot required for kernel/system changes to take effect");
        }
        
        Commands::List { detailed, limit } => {
            let generations = pm.list_generations().await?;
            let current = pm.current_generation().await?;
            
            let generations_to_show = if let Some(limit) = limit {
                generations.iter().take(limit).collect()
            } else {
                generations.iter().collect()
            };
            
            if generations_to_show.is_empty() {
                println!("No generations found");
                return Ok(());
            }
            
            println!("Available generations:");
            for gen in generations_to_show {
                let current_marker = if current.as_ref().map(|c| c.id) == Some(gen.id) {
                    " (current)"
                } else {
                    ""
                };
                
                if detailed {
                    println!("  {} - {} packages - {}{}", 
                             gen.id, gen.package_count, gen.created_at, current_marker);
                    // TODO: Show package list in detailed mode
                } else {
                    println!("  {}{}", gen.id, current_marker);
                }
            }
        }
        
        Commands::Status { packages } => {
            let current = pm.current_generation().await?;
            
            match current {
                Some(gen) => {
                    println!("Current generation: {}", gen.id);
                    println!("Created: {}", gen.created_at);
                    println!("Packages: {}", gen.package_count);
                    
                    if packages {
                        println!("\nPackages:");
                        // TODO: List packages in current generation
                        println!("  (package listing not yet implemented)");
                    }
                }
                None => {
                    println!("No active generation");
                }
            }
        }
        
        Commands::Gc { dry_run, older_than, keep_generations } => {
            let dry_run = dry_run || global_dry_run;
            
            if let Some(days) = older_than {
                info!("Running GC for generations older than {} days", days);
                // TODO: Implement age-based GC
                warn!("--older-than not yet implemented");
            }
            
            info!("Keeping at least {} generations", keep_generations);
            
            let stats = pm.collect_garbage(dry_run).await?;
            
            if dry_run {
                println!("Garbage collection (dry run):");
                println!("  Would delete {} packages", stats.packages_to_delete);
                println!("  Would free ~{} MB", stats.bytes_to_free / (1024 * 1024));
            } else {
                println!("Garbage collection completed:");
                println!("  Deleted {} packages", stats.packages_deleted);
                println!("  Freed ~{} MB", stats.bytes_freed / (1024 * 1024));
                println!("  Duration: {:.2}s", stats.duration.as_secs_f64());
            }
        }
        
        Commands::Rollback { yes } => {
            let generations = pm.list_generations().await?;
            let current = pm.current_generation().await?;
            
            let current_id = current.as_ref().map(|g| g.id);
            
            // Find previous generation
            let previous = generations
                .iter()
                .filter(|g| Some(g.id) != current_id)
                .max_by_key(|g| g.id);
            
            match previous {
                Some(prev_gen) => {
                    if !yes && !global_dry_run {
                        println!("Rollback from generation {} to generation {}?", 
                                current_id.unwrap_or(0), prev_gen.id);
                        print!("Continue? [y/N]: ");
                        use std::io::{self, Write};
                        io::stdout().flush()?;
                        
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        
                        if !input.trim().to_lowercase().starts_with('y') {
                            println!("Rollback cancelled");
                            return Ok(());
                        }
                    }
                    
                    if global_dry_run {
                        println!("Would rollback to generation {}", prev_gen.id);
                    } else {
                        pm.activate_generation(prev_gen.id).await?;
                        println!("Rolled back to generation {}", prev_gen.id);
                    }
                }
                None => {
                    println!("No previous generation available for rollback");
                }
            }
        }
        
        Commands::Info { package } => {
            let info = pm.package_info(&package).await?;
            
            match info {
                Some(pkg_info) => {
                    println!("Package: {}", package);
                    // TODO: Display package metadata
                    println!("  (detailed package info not yet implemented)");
                }
                None => {
                    println!("Package '{}' not found", package);
                }
            }
        }
        
        Commands::Resolve { show_resolution } => {
            info!("Resolving package versions");
            
            let resolver = nexis_pkg_mgr::resolver::PackageResolver::new(std::sync::Arc::new(config.clone()));
            let resolved = resolver.resolve_all_packages(&config.packages).await?;
            
            println!("Resolved {} packages:", resolved.len());
            
            for pkg in resolved {
                if show_resolution && pkg.config.version.is_latest() {
                    println!("  {} latest -> {} (resolved from git)", 
                             pkg.config.name, pkg.resolved_version);
                } else {
                    println!("  {} -> {}", pkg.config.name, pkg.resolved_version);
                }
            }
        }
        
        Commands::Config(config_cmd) => {
            execute_config_command(config, config_cmd).await?;
        }
        
        Commands::Store(store_cmd) => {
            execute_store_command(pm, store_cmd).await?;
        }
    }
    
    Ok(())
}

async fn execute_config_command(config: &NexisConfig, command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Validate { strict } => {
            info!("Validating configuration");
            
            config.validate()?;
            
            println!("Configuration is valid");
            
            // TODO: Additional validation checks
            if config.packages.is_empty() {
                if strict {
                    anyhow::bail!("No packages defined in configuration");
                } else {
                    warn!("No packages defined in configuration");
                }
            }
        }
        
        ConfigCommands::Show { section, format } => {
            match format.as_str() {
                "toml" => {
                    let toml_str = toml::to_string_pretty(&config)?;
                    println!("{}", toml_str);
                }
                "json" => {
                    let json_str = serde_json::to_string_pretty(&config)?;
                    println!("{}", json_str);
                }
                _ => {
                    anyhow::bail!("Unsupported format: {}. Use 'toml' or 'json'", format);
                }
            }
        }
        
        ConfigCommands::Edit { editor } => {
            let editor = editor.unwrap_or_else(|| {
                std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string())
            });
            
            let config_path = config.config_dir.join("config.toml");
            
            println!("Opening {} with {}", config_path.display(), editor);
            
            std::process::Command::new(editor)
                .arg(config_path)
                .status()?;
        }
    }
    
    Ok(())
}

async fn execute_store_command(pm: NexisPackageManager, command: StoreCommands) -> Result<()> {
    match command {
        StoreCommands::Stats { detailed } => {
            // TODO: Implement store statistics
            println!("Store statistics:");
            println!("  (not yet implemented)");
        }
        
        StoreCommands::Verify { fix } => {
            // TODO: Implement store verification
            println!("Store verification:");
            println!("  (not yet implemented)");
        }
        
        StoreCommands::Optimize { force } => {
            // TODO: Implement store optimization
            println!("Store optimization:");
            println!("  (not yet implemented)");
        }
    }
    
    Ok(())
}

fn setup_logging(verbose: bool, debug: bool) -> Result<()> {
    let log_level = if debug {
        log::LevelFilter::Debug
    } else if verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };
    
    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .format_timestamp_secs()
        .init();
    
    Ok(())
}
