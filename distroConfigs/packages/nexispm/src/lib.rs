//! Core library for NexisOS declarative package manager.
//! Provides store, metadata, garbage collection, generation management, and utilities.

/// Configuration parser
pub mod config;

/// Store backend
pub mod store;

/// Metadata management
pub mod meta;

/// Garbage collection
pub mod gc;

/// Generation management
pub mod gen;

/// Utility functions
pub mod util;

use anyhow::Result;

/// Initialize the package store (called during rebuild or startup)
pub fn init_store() -> Result<()> {
    util::log_info("Initializing store backend");
    // TODO: choose backend (sled for ext4, rocksdb for XFS)
    Ok(())
}

/// Rebuild the system according to `/etc/nexis/config.toml`.
/// Computes a new generation and activates it.
pub fn rebuild_system() -> Result<()> {
    util::log_info("Rebuilding system based on declarative config");
    // TODO: parse config, resolve packages, create new generation, activate it
    Ok(())
}

/// Roll back to a specific generation, or the previous one if None.
pub fn rollback(generation: Option<u32>) -> Result<()> {
    util::log_info(&format!("Rolling back to generation {:?}", generation));
    // TODO: activate the requested generation
    Ok(())
}

/// Garbage collect unused generations and free storage.
pub fn gc() -> Result<()> {
    util::log_info("Garbage collecting unused generations");
    // TODO: mark + sweep generations from the database
    Ok(())
}

/// Show current system generation info.
pub fn show_info() -> Result<()> {
    util::log_info("Showing current system generations info");
    // TODO: print active generation, available generations, store usage
    Ok(())
}

/// List all available generations (stub for now)
pub fn list_generations() -> Vec<String> {
    vec![]
}

/// Parse the declarative `/etc/nexis/config.toml`
/// Can be called separately from rebuild_system if needed.
pub fn rebuild_config() -> Result<()> {
    util::log_info("Parsing declarative config");
    // TODO: parse using config module
    Ok(())
}
