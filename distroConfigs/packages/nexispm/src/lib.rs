//! # NexisOS Package Manager
//!
//! A declarative system package manager with optimized store operations, generation rollbacks,
//! and high-performance garbage collection. Designed for NexisOS with support for both ext4+sled
//! and XFS+RocksDB backends.
//!
//! ## Core Features
//! - Content-addressable storage with deduplication
//! - Atomic generation-based rollbacks  
//! - Parallel garbage collection with staged deletes
//! - Support for "latest" git tag resolution
//! - SELinux policy integration
//!
//! ## Architecture
//! ```text
//! Store Layout: /store/ab/cd/abcd1234-package-name/
//! Metadata: DB tracks hash → path + refcounts
//! GC: Mark-and-sweep with parallel deletion workers
//! ```

use anyhow::{Context, Result};
use log::{debug, info, warn, error};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export core modules for library users
pub use config::{NexisConfig, PackageConfig, SystemConfig};
pub use store::{ContentStore, StoreBackend, StoreError};
pub use meta::{MetaStore, GenerationInfo, PackageMetadata};
pub use gc::{GarbageCollector, GCConfig, GCStats};
pub use gen::{GenerationManager, Generation, GenerationError};
pub use resolver::{PackageResolver, ResolverError, VersionSpec};
pub use builder::{PackageBuilder, BuildContext, BuildError};

// Core modules
pub mod config;
pub mod store;
pub mod meta;
pub mod gc;
pub mod gen;
pub mod resolver;
pub mod builder;
pub mod util;

/// Core error types for the package manager
#[derive(thiserror::Error, Debug)]
pub enum NexisError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("Store operation failed: {0}")]
    Store(#[from] store::StoreError),
    
    #[error("Metadata operation failed: {0}")]
    Meta(#[from] meta::MetaError),
    
    #[error("Garbage collection failed: {0}")]
    GC(#[from] gc::GCError),
    
    #[error("Generation management failed: {0}")]
    Generation(#[from] gen::GenerationError),
    
    #[error("Package resolution failed: {0}")]
    Resolver(#[from] resolver::ResolverError),
    
    #[error("Build failed: {0}")]
    Build(#[from] builder::BuildError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Package '{package}' not found")]
    PackageNotFound { package: String },
    
    #[error("Generation {gen_id} not found")]
    GenerationNotFound { gen_id: u64 },
    
    #[error("Invalid package configuration: {msg}")]
    InvalidPackage { msg: String },
}

/// Main package manager instance
pub struct NexisPackageManager {
    config: Arc<NexisConfig>,
    store: Arc<dyn ContentStore>,
    meta: Arc<dyn MetaStore>,
    gc: Arc<GarbageCollector>,
    generation_mgr: Arc<GenerationManager>,
    resolver: Arc<PackageResolver>,
    builder: Arc<PackageBuilder>,
}

impl NexisPackageManager {
    /// Initialize the package manager with the given configuration
    pub async fn new(config_path: impl AsRef<Path>) -> Result<Self> {
        let config = Arc::new(NexisConfig::load(config_path)
            .context("Failed to load configuration")?);
        
        info!("Initializing NexisOS Package Manager v{}", env!("CARGO_PKG_VERSION"));
        debug!("Configuration loaded from: {:?}", config_path.as_ref());
        
        // Initialize storage backend based on configuration
        let store = Self::init_store(&config).await
            .context("Failed to initialize content store")?;
        
        // Initialize metadata backend
        let meta = Self::init_meta_store(&config).await
            .context("Failed to initialize metadata store")?;
        
        // Initialize garbage collector
        let gc_config = gc::GCConfig::from_config(&config);
        let gc = Arc::new(GarbageCollector::new(
            Arc::clone(&store),
            Arc::clone(&meta),
            gc_config,
        ));
        
        // Initialize generation manager
        let generation_mgr = Arc::new(GenerationManager::new(
            Arc::clone(&meta),
            Arc::clone(&store),
            &config.system.grub_config_path,
        ).await?);
        
        // Initialize package resolver (handles "latest" git tag resolution)
        let resolver = Arc::new(PackageResolver::new(Arc::clone(&config)));
        
        // Initialize package builder
        let builder = Arc::new(PackageBuilder::new(
            Arc::clone(&config),
            Arc::clone(&store),
        ));
        
        Ok(Self {
            config,
            store,
            meta,
            gc,
            generation_mgr,
            resolver,
            builder,
        })
    }
    
    /// Build a new system generation from the current configuration
    pub async fn build_generation(&self) -> Result<Generation> {
        info!("Building new system generation");
        
        // Resolve all package versions (including "latest" git tags)
        let resolved_packages = self.resolver
            .resolve_all_packages(&self.config.packages)
            .await
            .context("Failed to resolve package versions")?;
        
        info!("Resolved {} packages", resolved_packages.len());
        
        // Build all packages that need building
        let mut built_packages = Vec::new();
        for pkg_config in resolved_packages {
            let built_pkg = self.builder
                .build_package(&pkg_config)
                .await
                .with_context(|| format!("Failed to build package '{}'", pkg_config.name))?;
            built_packages.push(built_pkg);
        }
        
        // Create new generation
        let generation = self.generation_mgr
            .create_generation(built_packages)
            .await
            .context("Failed to create new generation")?;
        
        info!("Created generation {} with {} packages", 
              generation.id, generation.packages.len());
        
        Ok(generation)
    }
    
    /// Activate a specific generation (make it the current system)
    pub async fn activate_generation(&self, gen_id: u64) -> Result<()> {
        info!("Activating generation {}", gen_id);
        
        self.generation_mgr
            .activate_generation(gen_id)
            .await
            .context("Failed to activate generation")?;
        
        info!("Successfully activated generation {}", gen_id);
        Ok(())
    }
    
    /// List all available generations
    pub async fn list_generations(&self) -> Result<Vec<GenerationInfo>> {
        self.generation_mgr
            .list_generations()
            .await
            .context("Failed to list generations")
    }
    
    /// Run garbage collection to clean up unused packages
    pub async fn collect_garbage(&self, dry_run: bool) -> Result<gc::GCStats> {
        if dry_run {
            info!("Running garbage collection (dry run)");
        } else {
            info!("Running garbage collection");
        }
        
        let stats = self.gc
            .collect_garbage(dry_run)
            .await
            .context("Garbage collection failed")?;
        
        if dry_run {
            info!("GC dry run completed: would free {} packages, ~{} MB", 
                  stats.packages_to_delete, stats.bytes_to_free / (1024 * 1024));
        } else {
            info!("GC completed: freed {} packages, ~{} MB in {:.2}s", 
                  stats.packages_deleted, stats.bytes_freed / (1024 * 1024), 
                  stats.duration.as_secs_f64());
        }
        
        Ok(stats)
    }
    
    /// Get information about a specific package
    pub async fn package_info(&self, name: &str) -> Result<Option<PackageMetadata>> {
        self.meta
            .get_package_metadata(name)
            .await
            .context("Failed to retrieve package metadata")
    }
    
    /// Get current system generation
    pub async fn current_generation(&self) -> Result<Option<GenerationInfo>> {
        self.generation_mgr
            .current_generation()
            .await
            .context("Failed to get current generation")
    }
    
    /// Initialize content store based on configuration
    async fn init_store(config: &NexisConfig) -> Result<Arc<dyn ContentStore>> {
        let store_path = &config.system.store_path;
        
        // Ensure store directory exists
        tokio::fs::create_dir_all(store_path).await
            .with_context(|| format!("Failed to create store directory: {:?}", store_path))?;
        
        match config.system.storage_backend.as_str() {
            "ext4" => {
                debug!("Initializing ext4 storage backend with hardlink deduplication");
                let store = store::Ext4Store::new(store_path).await?;
                Ok(Arc::new(store))
            }
            "xfs" => {
                debug!("Initializing XFS storage backend with reflink deduplication");
                let store = store::XfsStore::new(store_path).await?;
                Ok(Arc::new(store))
            }
            backend => {
                anyhow::bail!("Unsupported storage backend: {}", backend);
            }
        }
    }
    
    /// Initialize metadata store based on configuration
    async fn init_meta_store(config: &NexisConfig) -> Result<Arc<dyn MetaStore>> {
        let meta_path = config.system.store_path.join("meta");
        
        tokio::fs::create_dir_all(&meta_path).await
            .with_context(|| format!("Failed to create metadata directory: {:?}", meta_path))?;
        
        match config.system.storage_backend.as_str() {
            "ext4" => {
                debug!("Initializing sled metadata backend");
                let meta = meta::SledStore::new(&meta_path).await?;
                Ok(Arc::new(meta))
            }
            "xfs" => {
                debug!("Initializing RocksDB metadata backend");
                let meta = meta::RocksDbStore::new(&meta_path).await?;
                Ok(Arc::new(meta))
            }
            backend => {
                anyhow::bail!("Unsupported storage backend: {}", backend);
            }
        }
    }
}

/// Convenience function to initialize logging
pub fn init_logging() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    Ok(())
}

/// Convenience function to load configuration from default paths
pub async fn load_default_config() -> Result<NexisConfig> {
    let config_paths = [
        "/etc/nexis/config.toml",
        "config.toml",
        "nexis.toml",
    ];
    
    for path in &config_paths {
        if Path::new(path).exists() {
            debug!("Loading configuration from: {}", path);
            return NexisConfig::load(path)
                .with_context(|| format!("Failed to load config from: {}", path));
        }
    }
    
    anyhow::bail!("No configuration file found. Searched: {:?}", config_paths);
}
