//! Global constants for NexisPM
//!
//! Defines paths, limits, and configuration values used throughout the package manager.

use std::path::PathBuf;

// ============================================================================
// Store Paths
// ============================================================================

/// Root directory of the package store
pub const NEXIS_STORE_ROOT: &str = "/nexis-store";

/// Package storage directory
pub const NEXIS_STORE_PACKAGES: &str = "/nexis-store/packages";

/// File storage directory (content-addressed)
pub const NEXIS_STORE_FILES: &str = "/nexis-store/files";

/// Store metadata database
pub const NEXIS_STORE_METADATA: &str = "/nexis-store/metadata.redb";

/// Generations directory
pub const NEXIS_GENERATIONS: &str = "/nexis-store/generations";

/// Temporary build directory
pub const NEXIS_BUILD_TMP: &str = "/nexis-store/.tmp";

/// Garbage collection trash directory
pub const NEXIS_GC_TRASH: &str = "/nexis-store/.trash";

// ============================================================================
// Configuration Paths
// ============================================================================

/// Main configuration directory
pub const NEXIS_CONFIG_DIR: &str = "/etc/nexis";

/// System configuration file
pub const NEXIS_SYSTEM_CONFIG: &str = "/etc/nexis/system.toml";

/// Profile templates directory
pub const NEXIS_PROFILES_DIR: &str = "/etc/nexis/profiles";

/// Machine-specific configurations directory
pub const NEXIS_MACHINES_DIR: &str = "/etc/nexis/machines";

/// Lock file path
pub const NEXIS_LOCK_FILE: &str = "/etc/nexis/nexis.lock";

// ============================================================================
// System Paths
// ============================================================================

/// System binary directory
pub const SYSTEM_BIN_DIR: &str = "/usr/bin";

/// System library directory
pub const SYSTEM_LIB_DIR: &str = "/usr/lib";

/// System configuration directory
pub const SYSTEM_ETC_DIR: &str = "/etc";

/// Boot directory
pub const SYSTEM_BOOT_DIR: &str = "/boot";

// ============================================================================
// Performance & Limits
// ============================================================================

/// Default number of parallel build workers
pub const DEFAULT_PARALLEL_BUILDS: usize = 4;

/// Maximum number of parallel downloads
pub const MAX_PARALLEL_DOWNLOADS: usize = 8;

/// Maximum number of generations to keep (0 = unlimited)
pub const MAX_GENERATIONS_KEEP: usize = 10;

/// Store object hash prefix length (for bucketing: ab/cd/abcd...)
pub const HASH_PREFIX_LENGTH: usize = 2;

/// Download timeout in seconds
pub const DOWNLOAD_TIMEOUT_SECS: u64 = 300;

/// Build timeout in seconds (0 = no timeout)
pub const BUILD_TIMEOUT_SECS: u64 = 3600;

// ============================================================================
// Hashing & Compression
// ============================================================================

/// Hash algorithm identifier
pub const HASH_ALGORITHM: &str = "blake3";

/// Compression algorithm for store objects
pub const COMPRESSION_ALGORITHM: &str = "zstd";

/// Compression level (1-22 for zstd)
pub const COMPRESSION_LEVEL: i32 = 3;

// ============================================================================
// SELinux Contexts
// ============================================================================

/// SELinux context for store objects
pub const SELINUX_STORE_CONTEXT: &str = "system_u:object_r:immutable_dir_t:s0";

/// SELinux context for nexispm binary
pub const SELINUX_NEXISPM_CONTEXT: &str = "system_u:system_r:nexispm_t:s0";

// ============================================================================
// Logging & Diagnostics
// ============================================================================

/// Log directory
pub const NEXIS_LOG_DIR: &str = "/var/log/nexis";

/// Main log file
pub const NEXIS_LOG_FILE: &str = "/var/log/nexis/nexispm.log";

/// Build log directory
pub const NEXIS_BUILD_LOG_DIR: &str = "/var/log/nexis/builds";

/// User operation log directory
pub const NEXIS_USER_LOG_DIR: &str = "/var/log/nexis/users";

// ============================================================================
// Version Resolution
// ============================================================================

/// Common tag patterns for version detection
pub const TAG_PATTERNS: &[&str] = &[
    "v{version}",
    "{version}",
    "release-{version}",
    "{name}-{version}",
    "version-{version}",
];

/// Default branch names to try
pub const DEFAULT_BRANCHES: &[&str] = &["main", "master", "develop"];

/// Version cache duration in hours
pub const VERSION_CACHE_DURATION_HOURS: u64 = 24;

// ============================================================================
// File Permissions
// ============================================================================

/// Default file mode (0644)
pub const DEFAULT_FILE_MODE: u32 = 0o644;

/// Default directory mode (0755)
pub const DEFAULT_DIR_MODE: u32 = 0o755;

/// Default executable mode (0755)
pub const DEFAULT_EXEC_MODE: u32 = 0o755;

// ============================================================================
// Utility Functions
// ============================================================================

/// Get store root as PathBuf
pub fn store_root() -> PathBuf {
    PathBuf::from(NEXIS_STORE_ROOT)
}

/// Get packages directory as PathBuf
pub fn packages_dir() -> PathBuf {
    PathBuf::from(NEXIS_STORE_PACKAGES)
}

/// Get files directory as PathBuf
pub fn files_dir() -> PathBuf {
    PathBuf::from(NEXIS_STORE_FILES)
}

/// Get generations directory as PathBuf
pub fn generations_dir() -> PathBuf {
    PathBuf::from(NEXIS_GENERATIONS)
}

/// Get config directory as PathBuf
pub fn config_dir() -> PathBuf {
    PathBuf::from(NEXIS_CONFIG_DIR)
}

/// Get profiles directory as PathBuf
pub fn profiles_dir() -> PathBuf {
    PathBuf::from(NEXIS_PROFILES_DIR)
}

/// Get machines directory as PathBuf
pub fn machines_dir() -> PathBuf {
    PathBuf::from(NEXIS_MACHINES_DIR)
}

/// Calculate number of parallel workers based on CPU count
pub fn calculate_workers() -> usize {
    num_cpus::get().max(1).min(DEFAULT_PARALLEL_BUILDS * 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        assert_eq!(store_root(), PathBuf::from("/nexis-store"));
        assert_eq!(config_dir(), PathBuf::from("/etc/nexis"));
    }

    #[test]
    fn test_workers() {
        let workers = calculate_workers();
        assert!(workers > 0);
        assert!(workers <= DEFAULT_PARALLEL_BUILDS * 2);
    }

    #[test]
    fn test_hash_prefix_length() {
        assert_eq!(HASH_PREFIX_LENGTH, 2);
    }
}
