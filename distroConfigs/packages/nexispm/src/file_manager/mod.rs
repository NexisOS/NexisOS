//! File management module - provides home-manager-like functionality
//! for declarative file and directory management

pub mod config;
pub mod manager;
pub mod templates;
pub mod permissions;
pub mod hooks;

pub use config::*;
pub use manager::FileManager;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileManagerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),
    
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("Permission error: {0}")]
    Permission(String),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Condition not met: {0}")]
    ConditionNotMet(String),
    
    #[error("File already exists and force=false: {0}")]
    FileExists(String),
}

pub type Result<T> = std::result::Result<T, FileManagerError>;
