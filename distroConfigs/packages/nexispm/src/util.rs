//! Core utilities for NexisOS package manager
//! 
//! This module provides fundamental utilities used throughout the package manager:
//! - Content-addressable hashing
//! - Error types and handling
//! - I/O utilities
//! - Path manipulation helpers

use anyhow::{Context, Result};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

/// Content hash for store objects (256-bit BLAKE3)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Create a new content hash from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes of the hash
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string (lowercase)
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self, hex::FromHexError> {
        let bytes: Vec<u8> = hex::decode(hex)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(Self(hash))
    }

    /// Get the first 8 characters of the hex representation (for display)
    pub fn short_hex(&self) -> String {
        self.to_hex()[..8].to_string()
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Hash a file's contents using BLAKE3
pub fn hash_file<P: AsRef<Path>>(path: P) -> Result<ContentHash> {
    let mut file = File::open(path.as_ref())
        .with_context(|| format!("Failed to open file: {}", path.as_ref().display()))?;
    
    let mut hasher = Hasher::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .with_context(|| format!("Failed to read from file: {}", path.as_ref().display()))?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(ContentHash::from_bytes(*hasher.finalize().as_bytes()))
}

/// Hash arbitrary data using BLAKE3
pub fn hash_data<D: AsRef<[u8]>>(data: D) -> ContentHash {
    let hash = blake3::hash(data.as_ref());
    ContentHash::from_bytes(*hash.as_bytes())
}

/// Hash a directory recursively (content-addressable directory hash)
pub fn hash_directory<P: AsRef<Path>>(path: P) -> Result<ContentHash> {
    let mut hasher = Hasher::new();
    let mut entries = Vec::new();

    // Collect all entries for deterministic ordering
    for entry in WalkDir::new(path.as_ref())
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.with_context(|| {
            format!("Failed to traverse directory: {}", path.as_ref().display())
        })?;
        entries.push(entry);
    }

    // Hash each entry in sorted order
    for entry in entries {
        let relative_path = entry.path().strip_prefix(path.as_ref())
            .unwrap_or(entry.path());
        
        // Hash the relative path
        hasher.update(relative_path.to_string_lossy().as_bytes());
        hasher.update(&[0]); // separator
        
        let metadata = entry.metadata()
            .with_context(|| format!("Failed to get metadata for: {}", entry.path().display()))?;
        
        if metadata.is_file() {
            // Hash file contents
            let file_hash = hash_file(entry.path())?;
            hasher.update(file_hash.as_bytes());
        } else if metadata.is_dir() {
            // Just hash the directory marker
            hasher.update(b"dir");
        } else if metadata.is_symlink() {
            // Hash the symlink target
            let target = std::fs::read_link(entry.path())
                .with_context(|| format!("Failed to read symlink: {}", entry.path().display()))?;
            hasher.update(b"symlink:");
            hasher.update(target.to_string_lossy().as_bytes());
        }
        
        hasher.update(&[0]); // entry separator
    }

    Ok(ContentHash::from_bytes(*hasher.finalize().as_bytes()))
}

/// Package manager specific errors
#[derive(Error, Debug)]
pub enum NexisError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Store operation failed: {0}")]
    Store(String),
    
    #[error("Package not found: {0}")]
    PackageNotFound(String),
    
    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(String),
    
    #[error("Build failed for package {package}: {reason}")]
    BuildFailed { package: String, reason: String },
    
    #[error("Generation operation failed: {0}")]
    Generation(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias for convenience
pub type NexisResult<T> = std::result::Result<T, NexisError>;

/// Atomic file write utility - write to temporary file then rename
pub fn atomic_write<P: AsRef<Path>, D: AsRef<[u8]>>(path: P, data: D) -> Result<()> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");
    
    // Write to temporary file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)
        .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;
    
    file.write_all(data.as_ref())
        .with_context(|| format!("Failed to write to temp file: {}", temp_path.display()))?;
    
    file.sync_all()
        .with_context(|| format!("Failed to sync temp file: {}", temp_path.display()))?;
    
    // Atomic rename
    std::fs::rename(&temp_path, path)
        .with_context(|| format!("Failed to rename {} to {}", temp_path.display(), path.display()))?;
    
    Ok(())
}

/// Create directory recursively if it doesn't exist
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    std::fs::create_dir_all(path.as_ref())
        .with_context(|| format!("Failed to create directory: {}", path.as_ref().display()))
}

/// Remove directory and all its contents
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    if path.as_ref().exists() {
        std::fs::remove_dir_all(path.as_ref())
            .with_context(|| format!("Failed to remove directory: {}", path.as_ref().display()))?;
    }
    Ok(())
}

/// Check if a path is a subdirectory of another path
pub fn is_subdirectory<P1: AsRef<Path>, P2: AsRef<Path>>(child: P1, parent: P2) -> bool {
    let child = child.as_ref().canonicalize().unwrap_or_else(|_| child.as_ref().to_path_buf());
    let parent = parent.as_ref().canonicalize().unwrap_or_else(|_| parent.as_ref().to_path_buf());
    
    child.starts_with(parent)
}

/// Format file size in human-readable format
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Simple progress callback trait
pub trait ProgressCallback {
    fn update(&mut self, current: u64, total: u64, message: &str);
}

/// No-op progress callback
pub struct NoProgress;

impl ProgressCallback for NoProgress {
    fn update(&mut self, _current: u64, _total: u64, _message: &str) {}
}

/// Console progress callback
pub struct ConsoleProgress {
    last_update: std::time::Instant,
    update_interval: std::time::Duration,
}

impl ConsoleProgress {
    pub fn new() -> Self {
        Self {
            last_update: std::time::Instant::now(),
            update_interval: std::time::Duration::from_millis(100),
        }
    }
}

impl Default for ConsoleProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressCallback for ConsoleProgress {
    fn update(&mut self, current: u64, total: u64, message: &str) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_update) < self.update_interval && current != total {
            return;
        }
        self.last_update = now;
        
        if total > 0 {
            let percentage = (current as f64 / total as f64 * 100.0) as u32;
            eprintln!("[{:3}%] {} ({}/{})", percentage, message, format_size(current), format_size(total));
        } else {
            eprintln!("[ ? ] {} ({})", message, format_size(current));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_content_hash_hex() {
        let hash = ContentHash::from_bytes([0; 32]);
        assert_eq!(hash.to_hex(), "0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(hash.short_hex(), "00000000");
    }

    #[test]
    fn test_content_hash_from_hex() {
        let hex = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let hash = ContentHash::from_hex(hex).unwrap();
        assert_eq!(hash.to_hex(), hex);
    }

    #[test]
    fn test_hash_data() {
        let data = b"hello world";
        let hash1 = hash_data(data);
        let hash2 = hash_data(data);
        assert_eq!(hash1, hash2);
        
        let different_data = b"hello world!";
        let hash3 = hash_data(different_data);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_file() -> Result<()> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test content")?;
        
        let hash = hash_file(&file_path)?;
        assert_eq!(hash.to_hex().len(), 64); // 32 bytes = 64 hex chars
        
        Ok(())
    }

    #[test]
    fn test_atomic_write() -> Result<()> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.txt");
        
        atomic_write(&file_path, b"test content")?;
        
        let content = fs::read_to_string(&file_path)?;
        assert_eq!(content, "test content");
        
        Ok(())
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(1024 * 1024), "1.0 MiB");
    }

    #[test]
    fn test_is_subdirectory() {
        assert!(is_subdirectory("/home/user/documents", "/home/user"));
        assert!(!is_subdirectory("/home/user", "/home/user/documents"));
        assert!(is_subdirectory("/home/user", "/home/user")); // same path
    }
}
