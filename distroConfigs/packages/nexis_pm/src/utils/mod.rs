//! Utility modules for NexisPM
//!
//! Provides common functionality used across the package manager:
//! - Error handling and types
//! - Filesystem operations
//! - Logging setup
//! - Progress indicators
//! - Path manipulation

pub mod errors;
pub mod fs;
pub mod logging;
pub mod paths;
pub mod progress;

// Re-export commonly used items
pub use errors::{NexisError, Result};
pub use fs::{copy_file, create_dir_all, ensure_dir_exists};
pub use paths::{normalize_path, resolve_path};
