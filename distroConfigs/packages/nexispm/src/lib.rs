//! # NexisPM - Declarative Package Manager for NexisOS
//!
//! A fast, declarative package manager with:
//! - Content-addressed storage with XFS reflinks
//! - Atomic generation-based rollbacks
//! - SELinux-enforced immutability
//! - Fleet management with profile templates
//! - Parallel builds and operations
//!
//! ## Example Usage
//!
//! ```no_run
//! use nexispm::{Config, Store, GenerationManager};
//! use std::path::PathBuf;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Load configuration
//! let config = Config::load("/etc/nexis/system.toml")?;
//!
//! // Initialize store
//! let store = Store::open("/nexis-store")?;
//!
//! // Create new generation
//! let gen_manager = GenerationManager::new(PathBuf::from("/nexis-store/generations"));
//! let gen_id = gen_manager.create_generation(&config)?;
//!
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod config;
pub mod store;
pub mod packages;
pub mod files;
pub mod generations;
pub mod users;
pub mod fleet;
pub mod security;
pub mod services;
pub mod build;
pub mod vcs;
pub mod utils;
pub mod constants;

// CLI module (only when building binary)
#[cfg(feature = "cli")]
pub mod cli;

// Re-export commonly used types for convenience
pub use config::{Config, Package, User, FileDeclaration};
pub use store::Store;
pub use generations::GenerationManager;
pub use utils::errors::{Result, NexisError};

/// NexisPM version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default number of parallel build workers
pub const DEFAULT_WORKERS: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
