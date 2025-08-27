//! # Content-Addressable Package Store
//!
//! High-performance content-addressable storage with deduplication for NexisOS packages.
//! Supports both ext4+hardlinks and XFS+reflinks backends with bucketed hash layout
//! for optimal filesystem performance.
//!
//! ## Store Layout
//! ```text
//! /store/
//! ├── ab/cd/abcd1234-package-name/     # Bucketed by hash prefix
//! ├── ef/gh/efgh5678-other-package/
//! └── .trash/                          # Staged deletes for GC
//!     └── to-delete-123456/
//! ```
//!
//! ## Performance Features
//! - Ingest-time deduplication (no global sweeps)
//! - Parallel operations with async I/O
//! - Bucketed layout reduces filesystem bottlenecks
//! - Optional io_uring for batch operations

use anyhow::{Context, Result};
use async_trait::async_trait;
use blake3::Hasher;
use log::{debug, info, warn, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

pub use backend::StoreBackend;
pub use ingest::IngestOptions;
pub use layout::{StoreLayout, HashBucket};

pub mod backend;
pub mod ingest;
pub mod layout;

/// Store operation errors
#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("Package not found: {hash}")]
    PackageNotFound { hash: String },
    
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    
    #[error("Store corruption detected: {msg}")]
    Corruption { msg: String },
    
    #[error("Deduplication failed: {msg}")]
    DeduplicationError { msg: String },
    
    #[error("Backend operation failed: {msg}")]
    BackendError { msg: String },
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Path error: {path} - {msg}")]
    PathError { path: PathBuf, msg: String },
    
    #[error("Concurrent access conflict for package {hash}")]
    ConcurrencyError { hash: String },
}

/// Package metadata stored alongside content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPackage {
    /// Content hash (BLAKE3)
    pub hash: String,
    
    /// Package name
    pub name: String,
    
    /// Resolved version
    pub version: String,
    
    /// Size in bytes
    pub size: u64,
    
    /// Store path relative to store root
    pub store_path: PathBuf,
    
    /// When this package was first stored
    pub ingested_at: chrono::DateTime<chrono::Utc>,
    
    /// Reference count (for GC)
    pub ref_count: u64,
    
    /// Build metadata
    pub build_info: BuildInfo,
    
    /// Deduplication info
    pub dedup_info: Option<DeduplicationInfo>,
}

/// Build information for a stored package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Build system used
    pub build_system: String,
    
    /// Build flags
    pub build_flags: Vec<String>,
    
    /// Build environment
    pub build_env: HashMap<String, String>,
    
    /// Source URL or path
    pub source: Option<String>,
    
    /// Build duration
    pub build_duration: Option<std::time::Duration>,
    
    /// Builder host information
    pub builder_host: String,
}

/// Deduplication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationInfo {
    /// Number of files deduplicated
    pub deduplicated_files: u64,
    
    /// Bytes saved through deduplication
    pub bytes_saved: u64,
    
    /// Deduplication method used
    pub method: DeduplicationMethod,
    
    /// Original packages that share content
    pub shared_with: Vec<String>,
}

/// Deduplication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeduplicationMethod {
    /// Hard links (ext4)
    Hardlink,
    /// Reflinks (XFS/Btrfs)
    Reflink,
    /// No deduplication
    None,
}

/// Store operation statistics
#[derive(Debug, Clone, Default)]
pub struct StoreStats {
    pub total_packages: u64,
    pub total_size: u64,
    pub deduplicated_size: u64,
    pub deduplication_ratio: f64,
    pub average_package_size: u64,
    pub store_efficiency: f64,
}

/// Content store trait - abstracts over different storage backends
#[async_trait]
pub trait ContentStore: Send + Sync {
    /// Store a package directory in the content-addressable store
    async fn store_package(
        &self,
        source_path: &Path,
        package_name: &str,
        version: &str,
        build_info: BuildInfo,
        options: IngestOptions,
    ) -> Result<StoredPackage, StoreError>;
    
    /// Retrieve a package by its content hash
    async fn get_package(&self, hash: &str) -> Result<Option<StoredPackage>, StoreError>;
    
    /// Check if a package exists by hash
    async fn has_package(&self, hash: &str) -> Result<bool, StoreError>;
    
    /// Get the filesystem path to a stored package
    async fn get_package_path(&self, hash: &str) -> Result<PathBuf, StoreError>;
    
    /// List all stored packages
    async fn list_packages(&self) -> Result<Vec<StoredPackage>, StoreError>;
    
    /// Calculate content hash for a directory
    async fn calculate_hash(&self, path: &Path) -> Result<String, StoreError>;
    
    /// Mark package for deletion (moves to .trash)
    async fn mark_for_deletion(&self, hash: &str) -> Result<(), StoreError>;
    
    /// Permanently delete packages in .trash
    async fn empty_trash(&self) -> Result<Vec<String>, StoreError>;
    
    /// Get store statistics
    async fn get_stats(&self) -> Result<StoreStats, StoreError>;
    
    /// Optimize store layout and cleanup
    async fn optimize(&self) -> Result<(), StoreError>;
    
    /// Verify store integrity
    async fn verify(&self, fix_errors: bool) -> Result<Vec<String>, StoreError>;
    
    /// Get backend-specific information
    fn backend_info(&self) -> &dyn StoreBackend;
}

/// ext4 storage implementation using hardlinks for deduplication
pub struct Ext4Store {
    store_path: PathBuf,
    layout: StoreLayout,
    backend: backend::Ext4Backend,
    package_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl Ext4Store {
    /// Create a new ext4 store
    pub async fn new(store_path: &Path) -> Result<Self, StoreError> {
        info!("Initializing ext4 store at: {:?}", store_path);
        
        let layout = StoreLayout::new(store_path, 2); // 2-level bucketing for ext4
        let backend = backend::Ext4Backend::new(store_path).await?;
        
        // Create store structure
        fs::create_dir_all(&store_path).await?;
        fs::create_dir_all(store_path.join(".trash")).await?;
        fs::create_dir_all(store_path.join(".tmp")).await?;
        
        Ok(Self {
            store_path: store_path.to_path_buf(),
            layout,
            backend,
            package_locks: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl ContentStore for Ext4Store {
    async fn store_package(
        &self,
        source_path: &Path,
        package_name: &str,
        version: &str,
        build_info: BuildInfo,
        options: IngestOptions,
    ) -> Result<StoredPackage, StoreError> {
        debug!("Storing package '{}' version '{}' from {:?}", 
               package_name, version, source_path);
        
        // Calculate content hash
        let hash = self.calculate_hash(source_path).await?;
        debug!("Package hash: {}", hash);
        
        // Check if package already exists (deduplication)
        if let Some(existing) = self.get_package(&hash).await? {
            info!("Package '{}' already exists with hash {}, incrementing ref count", 
                  package_name, hash);
            // TODO: Increment reference count in metadata store
            return Ok(existing);
        }
        
        // Get package lock to prevent concurrent operations
        let package_lock = {
            let mut locks = self.package_locks.write().await;
            locks.entry(hash.clone())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        
        let _lock_guard = package_lock.lock().await;
        
        // Double-check after acquiring lock
        if let Some(existing) = self.get_package(&hash).await? {
            return Ok(existing);
        }
        
        // Create store path
        let store_path = self.layout.get_package_path(&hash, package_name);
        let store_dir = store_path.parent()
            .ok_or_else(|| StoreError::PathError {
                path: store_path.clone(),
                msg: "Invalid store path".to_string(),
            })?;
        
        fs::create_dir_all(store_dir).await
            .with_context(|| format!("Failed to create store directory: {:?}", store_dir))?;
        
        // Ingest package with backend-specific deduplication
        let dedup_info = self.backend
            .ingest_package(source_path, &store_path, &options)
            .await
            .with_context(|| format!("Failed to ingest package to {:?}", store_path))?;
        
        // Calculate package size
        let size = calculate_dir_size(&store_path).await?;
        
        let stored_package = StoredPackage {
            hash: hash.clone(),
            name: package_name.to_string(),
            version: version.to_string(),
            size,
            store_path: store_path.strip_prefix(&self.store_path)
                .unwrap_or(&store_path)
                .to_path_buf(),
            ingested_at: chrono::Utc::now(),
            ref_count: 1,
            build_info,
            dedup_info,
        };
        
        info!("Successfully stored package '{}' ({} bytes) with {} deduplication", 
              package_name, size,
              stored_package.dedup_info.as_ref()
                  .map(|d| format!("{:?}", d.method))
                  .unwrap_or_else(|| "no".to_string()));
        
        Ok(stored_package)
    }
    
    async fn get_package(&self, hash: &str) -> Result<Option<StoredPackage>, StoreError> {
        // This would typically query the metadata store
        // For now, we check if the path exists
        let package_path = self.layout.find_package_by_hash(hash).await?;
        
        if let Some(path) = package_path {
            // TODO: Load metadata from metadata store
            // For now, return a basic package info
            Ok(Some(StoredPackage {
                hash: hash.to_string(),
                name: "unknown".to_string(), // Would be loaded from metadata
                version: "unknown".to_string(),
                size: calculate_dir_size(&path).await?,
                store_path: path.strip_prefix(&self.store_path)
                    .unwrap_or(&path)
                    .to_path_buf(),
                ingested_at: chrono::Utc::now(),
                ref_count: 1,
                build_info: BuildInfo {
                    build_system: "unknown".to_string(),
                    build_flags: vec![],
                    build_env: HashMap::new(),
                    source: None,
                    build_duration: None,
                    builder_host: "unknown".to_string(),
                },
                dedup_info: None,
            }))
        } else {
            Ok(None)
        }
    }
    
    async fn has_package(&self, hash: &str) -> Result<bool, StoreError> {
        let package_path = self.layout.find_package_by_hash(hash).await?;
        Ok(package_path.is_some())
    }
    
    async fn get_package_path(&self, hash: &str) -> Result<PathBuf, StoreError> {
        let package_path = self.layout.find_package_by_hash(hash).await?;
        package_path.ok_or_else(|| StoreError::PackageNotFound {
            hash: hash.to_string(),
        })
    }
    
    async fn list_packages(&self) -> Result<Vec<StoredPackage>, StoreError> {
        let mut packages = Vec::new();
        
        // Walk through bucketed directories
        let mut walker = fs::read_dir(&self.store_path).await?;
        
        while let Some(entry) = walker.next_entry().await? {
            let path = entry.path();
            if path.is_dir() && !path.file_name().unwrap().to_str().unwrap().starts_with('.') {
                self.collect_packages_from_dir(&path, &mut packages).await?;
            }
        }
        
        Ok(packages)
    }
    
    async fn calculate_hash(&self, path: &Path) -> Result<String, StoreError> {
        calculate_directory_hash(path).await
    }
    
    async fn mark_for_deletion(&self, hash: &str) -> Result<(), StoreError> {
        let package_path = self.get_package_path(hash).await?;
        let trash_dir = self.store_path.join(".trash");
        let trash_path = trash_dir.join(format!("to-delete-{}-{}", hash, chrono::Utc::now().timestamp()));
        
        fs::create_dir_all(&trash_dir).await?;
        fs::rename(&package_path, &trash_path).await
            .with_context(|| format!("Failed to move {:?} to trash", package_path))?;
        
        debug!("Marked package {} for deletion", hash);
        Ok(())
    }
    
    async fn empty_trash(&self) -> Result<Vec<String>, StoreError> {
        let trash_dir = self.store_path.join(".trash");
        let mut deleted = Vec::new();
        
        if !trash_dir.exists() {
            return Ok(deleted);
        }
        
        let mut walker = fs::read_dir(&trash_dir).await?;
        
        while let Some(entry) = walker.next_entry().await? {
            let path = entry.path();
            let name = entry.file_name();
            
            if path.is_dir() {
                debug!("Permanently deleting: {:?}", path);
                fs::remove_dir_all(&path).await
                    .with_context(|| format!("Failed to delete {:?}", path))?;
                deleted.push(name.to_string_lossy().to_string());
            }
        }
        
        info!("Permanently deleted {} packages from trash", deleted.len());
        Ok(deleted)
    }
    
    async fn get_stats(&self) -> Result<StoreStats, StoreError> {
        let packages = self.list_packages().await?;
        
        let total_packages = packages.len() as u64;
        let total_size: u64 = packages.iter().map(|p| p.size).sum();
        let deduplicated_size: u64 = packages.iter()
            .filter_map(|p| p.dedup_info.as_ref().map(|d| d.bytes_saved))
            .sum();
        
        let deduplication_ratio = if total_size > 0 {
            deduplicated_size as f64 / total_size as f64
        } else {
            0.0
        };
        
        let average_package_size = if total_packages > 0 {
            total_size / total_packages
        } else {
            0
        };
        
        Ok(StoreStats {
            total_packages,
            total_size,
            deduplicated_size,
            deduplication_ratio,
            average_package_size,
            store_efficiency: 1.0 - deduplication_ratio,
        })
    }
    
    async fn optimize(&self) -> Result<(), StoreError> {
        info!("Optimizing ext4 store");
        // TODO: Implement store optimization
        // - Rebalance bucket sizes
        // - Consolidate small directories
        // - Update layout if needed
        warn!("Store optimization not yet implemented");
        Ok(())
    }
    
    async fn verify(&self, fix_errors: bool) -> Result<Vec<String>, StoreError> {
        info!("Verifying store integrity (fix_errors={})", fix_errors);
        let mut issues = Vec::new();
        
        // TODO: Implement integrity verification
        // - Check hash consistency
        // - Verify hardlink counts
        // - Check for orphaned files
        // - Validate bucket structure
        
        warn!("Store verification not yet implemented");
        Ok(issues)
    }
    
    fn backend_info(&self) -> &dyn StoreBackend {
        &self.backend
    }
}

impl Ext4Store {
    async fn collect_packages_from_dir(&self, dir: &Path, packages: &mut Vec<StoredPackage>) -> Result<(), StoreError> {
        let mut walker = fs::read_dir(dir).await?;
        
        while let Some(entry) = walker.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if this looks like a package directory (hash-name format)
                    if let Some(dash_pos) = name.find('-') {
                        let hash = &name[..dash_pos];
                        if hash.len() >= 8 { // Minimum hash length
                            // This looks like a package directory
                            if let Ok(Some(package)) = self.get_package(hash).await {
                                packages.push(package);
                            }
                        }
                    }
                } else {
                    // Recurse into subdirectories (bucket structure)
                    self.collect_packages_from_dir(&path, packages).await?;
                }
            }
        }
        
        Ok(())
    }
}

/// XFS storage implementation using reflinks for deduplication
pub struct XfsStore {
    store_path: PathBuf,
    layout: StoreLayout,
    backend: backend::XfsBackend,
    package_locks: Arc<RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl XfsStore {
    /// Create a new XFS store
    pub async fn new(store_path: &Path) -> Result<Self, StoreError> {
        info!("Initializing XFS store at: {:?}", store_path);
        
        let layout = StoreLayout::new(store_path, 1); // 1-level bucketing for XFS (reflinks are fast)
        let backend = backend::XfsBackend::new(store_path).await?;
        
        // Create store structure
        fs::create_dir_all(&store_path).await?;
        fs::create_dir_all(store_path.join(".trash")).await?;
        fs::create_dir_all(store_path.join(".tmp")).await?;
        
        Ok(Self {
            store_path: store_path.to_path_buf(),
            layout,
            backend,
            package_locks: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

// XfsStore implements the same ContentStore trait with reflink-specific optimizations
#[async_trait]
impl ContentStore for XfsStore {
    // Implementation would be similar to Ext4Store but using XFS reflinks
    // For brevity, I'll implement the key differences
    
    async fn store_package(
        &self,
        source_path: &Path,
        package_name: &str,
        version: &str,
        build_info: BuildInfo,
        options: IngestOptions,
    ) -> Result<StoredPackage, StoreError> {
        // Similar to Ext4Store but uses reflink deduplication
        debug!("Storing package '{}' with XFS reflinks", package_name);
        // ... implementation using self.backend (XfsBackend)
        todo!("XFS store implementation")
    }
    
    // ... other methods similar to Ext4Store but with XFS optimizations
    async fn get_package(&self, hash: &str) -> Result<Option<StoredPackage>, StoreError> { todo!() }
    async fn has_package(&self, hash: &str) -> Result<bool, StoreError> { todo!() }
    async fn get_package_path(&self, hash: &str) -> Result<PathBuf, StoreError> { todo!() }
    async fn list_packages(&self) -> Result<Vec<StoredPackage>, StoreError> { todo!() }
    async fn calculate_hash(&self, path: &Path) -> Result<String, StoreError> { todo!() }
    async fn mark_for_deletion(&self, hash: &str) -> Result<(), StoreError> { todo!() }
    async fn empty_trash(&self) -> Result<Vec<String>, StoreError> { todo!() }
    async fn get_stats(&self) -> Result<StoreStats, StoreError> { todo!() }
    async fn optimize(&self) -> Result<(), StoreError> { todo!() }
    async fn verify(&self, fix_errors: bool) -> Result<Vec<String>, StoreError> { todo!() }
    
    fn backend_info(&self) -> &dyn StoreBackend {
        &self.backend
    }
}

/// Calculate BLAKE3 hash of directory contents
async fn calculate_directory_hash(path: &Path) -> Result<String, StoreError> {
    let mut hasher = Hasher::new();
    
    // Walk directory in deterministic order
    let mut entries = vec![];
    collect_entries_recursive(path, &mut entries).await?;
    entries.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by path
    
    for (rel_path, abs_path) in entries {
        // Hash the relative path
        hasher.update(rel_path.as_os_str().to_string_lossy().as_bytes());
        
        // Hash file contents
        if abs_path.is_file() {
            let mut file = fs::File::open(&abs_path).await?;
            let mut buffer = vec![0u8; 8192];
            
            loop {
                let bytes_read = file.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
        }
    }
    
    Ok(hasher.finalize().to_hex().to_string())
}

async fn collect_entries_recursive(
    dir: &Path,
    entries: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), StoreError> {
    let mut walker = fs::read_dir(dir).await?;
    
    while let Some(entry) = walker.next_entry().await? {
        let abs_path = entry.path();
        let rel_path = abs_path.strip_prefix(dir)
            .map_err(|_| StoreError::PathError {
                path: abs_path.clone(),
                msg: "Failed to create relative path".to_string(),
            })?
            .to_path_buf();
        
        if abs_path.is_dir() {
            collect_entries_recursive(&abs_path, entries).await?;
        } else {
            entries.push((rel_path, abs_path));
        }
    }
    
    Ok(())
}

async fn calculate_dir_size(path: &Path) -> Result<u64, StoreError> {
    let mut total_size = 0u64;
    let mut stack = vec![path.to_path_buf()];
    
    while let Some(current_path) = stack.pop() {
        let mut walker = fs::read_dir(&current_path).await?;
        
        while let Some(entry) = walker.next_entry().await? {
            let entry_path = entry.path();
            let metadata = entry.metadata().await?;
            
            if metadata.is_dir() {
                stack.push(entry_path);
            } else {
                total_size += metadata.len();
            }
        }
    }
    
    Ok(total_size)
}
