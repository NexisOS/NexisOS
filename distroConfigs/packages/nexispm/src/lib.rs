//! NexisOS Package Manager
//! 
//! A declarative system package manager with optimized store operations,
//! refcounting, generation rollbacks, and home-manager-like file management.

#![warn(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod config;
pub mod resolver;
pub mod util;

// File and configuration management
pub mod file_manager;

// Storage and metadata
pub mod store;
pub mod meta;

// Generation management
pub mod gen;
pub mod gc;

// Build system integration
pub mod build;

// Networking
pub mod network;

// Cryptography
pub mod crypto;

// System integration
pub mod dinit;

// SELinux support (conditional compilation for Linux)
#[cfg(target_os = "linux")]
pub mod selinux;

// Re-exports for convenience
pub use file_manager::{FileManager, NexisConfig};
pub use store::Store;
pub use resolver::DependencyResolver;

/// Package manager version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration paths
pub mod paths {
    use camino::Utf8PathBuf;
    
    /// System configuration directory
    pub const SYSTEM_CONFIG_DIR: &str = "/etc/nexispm";
    
    /// User configuration directory
    pub const USER_CONFIG_DIR: &str = ".config/nexispm";
    
    /// Package store path
    pub const STORE_PATH: &str = "/store";
    
    /// Generation profiles path
    pub const PROFILES_PATH: &str = "/store/profiles";
    
    /// Get user config path
    pub fn user_config_dir() -> Utf8PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            Utf8PathBuf::from(home).join(USER_CONFIG_DIR)
        } else {
            Utf8PathBuf::from(USER_CONFIG_DIR)
        }
    }
    
    /// Get XDG config home
    pub fn xdg_config_home() -> Utf8PathBuf {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            Utf8PathBuf::from(xdg)
        } else if let Ok(home) = std::env::var("HOME") {
            Utf8PathBuf::from(home).join(".config")
        } else {
            Utf8PathBuf::from(".config")
        }
    }
}
