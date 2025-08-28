use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use crate::util::{NexisError, hash_file, hash_string};

pub mod ingest;
pub mod backend;
pub mod layout;

pub use ingest::*;
pub use backend::*;
pub use layout::*;

/// Represents a package in the store
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePackage {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub path: PathBuf,
    pub size: u64,
    pub files: Vec<StoreFile>,
    pub dependencies: Vec<String>,
}

/// Represents a file within a package
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreFile {
    pub relative_path: String,
    pub hash: String,
    pub size: u64,
    pub mode: u32,
    pub symlink_target: Option<String>,
}

/// Store statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct StoreStats {
    pub total_packages: u64,
    pub total_files: u64,
    pub total_size: u64,
    pub deduplicated_size: u64,
    pub dedup_ratio: f64,
}

/// Configuration for store behavior
#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub store_path: PathBuf,
    pub shard_depth: u8,
    pub backend: StorageBackend,
    pub compression: bool,
    pub parallel_workers: usize,
}

/// Storage backend selection
#[derive(Debug, Clone, PartialEq)]
pub enum StorageBackend {
    /// ext4 filesystem with hard links for deduplication
    Ext4,
    /// XFS filesystem with reflinks for deduplication  
    Xfs,
    /// Simple copy-based storage (for testing)
    Simple,
}

/// Main store interface
pub trait Store: Send + Sync {
    /// Add a package to the store from a directory
    fn add_package(&mut self, source_path: &Path, name: &str, version: &str) -> Result<StorePackage, NexisError>;
    
    /// Get package by name and version
    fn get_package(&self, name: &str, version: &str) -> Result<Option<StorePackage>, NexisError>;
    
    /// List all packages in the store
    fn list_packages(&self) -> Result<Vec<StorePackage>, NexisError>;
    
    /// Remove a package from the store (decrements refcount)
    fn remove_package(&mut self, name: &str, version: &str) -> Result<(), NexisError>;
    
    /// Get store statistics
    fn get_stats(&self) -> Result<StoreStats, NexisError>;
    
    /// Run garbage collection
    fn gc(&mut self, dry_run: bool) -> Result<u64, NexisError>;
    
    /// Verify store integrity
    fn verify(&self) -> Result<Vec<String>, NexisError>;
}

/// File-system based store implementation
pub struct FileStore {
    config: StoreConfig,
    layout: StoreLayout,
    backend: Box<dyn StorageBackendTrait>,
}

impl FileStore {
    /// Create a new file store
    pub fn new(config: StoreConfig) -> Result<Self, NexisError> {
        let layout = StoreLayout::new(&config.store_path, config.shard_depth)?;
        
        let backend: Box<dyn StorageBackendTrait> = match config.backend {
            StorageBackend::Ext4 => Box::new(Ext4Backend::new(config.parallel_workers)),
            StorageBackend::Xfs => Box::new(XfsBackend::new(config.parallel_workers)),
            StorageBackend::Simple => Box::new(SimpleBackend::new()),
        };

        // Ensure store directory exists
        fs::create_dir_all(&config.store_path)
            .map_err(|e| NexisError::Io {
                path: config.store_path.clone(),
                source: e,
            })?;

        Ok(Self {
            config,
            layout,
            backend,
        })
    }

    /// Create store with auto-detected backend based on filesystem
    pub fn auto_detect<P: AsRef<Path>>(store_path: P) -> Result<Self, NexisError> {
        let store_path = store_path.as_ref().to_path_buf();
        let backend = detect_filesystem(&store_path)?;
        
        let shard_depth = match backend {
            StorageBackend::Ext4 => 2,  // More sharding for ext4
            StorageBackend::Xfs => 1,   // Less sharding for XFS
            StorageBackend::Simple => 1,
        };

        let config = StoreConfig {
            store_path,
            shard_depth,
            backend,
            compression: false, // TODO: Make configurable
            parallel_workers: num_cpus::get(),
        };

        Self::new(config)
    }

    /// Calculate hash for a package directory
    fn calculate_package_hash(&self, source_path: &Path) -> Result<String, NexisError> {
        use std::collections::BTreeMap;
        
        let mut file_hashes = BTreeMap::new();
        
        for entry in walkdir::WalkDir::new(source_path)
            .follow_links(false)
            .sort_by_file_name() 
        {
            let entry = entry.map_err(|e| NexisError::Io {
                path: source_path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

            if entry.file_type().is_file() {
                let rel_path = entry.path().strip_prefix(source_path)
                    .map_err(|_| NexisError::Store("Invalid file path".to_string()))?;
                
                let file_hash = hash_file(entry.path())?;
                file_hashes.insert(rel_path.to_string_lossy().to_string(), file_hash);
            }
        }

        // Create deterministic hash of all file hashes
        let combined = format!("{:?}", file_hashes);
        Ok(hash_string(&combined))
    }

    /// Get the store path for a package
    fn get_package_store_path(&self, name: &str, hash: &str) -> PathBuf {
        self.layout.get_package_path(&format!("{}-{}", hash, name))
    }
}

impl Store for FileStore {
    fn add_package(&mut self, source_path: &Path, name: &str, version: &str) -> Result<StorePackage, NexisError> {
        if !source_path.exists() {
            return Err(NexisError::Store(format!("Source path does not exist: {}", source_path.display())));
        }

        // Calculate package hash
        let hash = self.calculate_package_hash(source_path)?;
        let store_path = self.get_package_store_path(name, &hash);

        // Check if package already exists
        if store_path.exists() {
            return self.load_existing_package(&store_path, name, version, &hash);
        }

        // Create store directory structure
        let store_parent = store_path.parent()
            .ok_or_else(|| NexisError::Store("Invalid store path".to_string()))?;
        fs::create_dir_all(store_parent)
            .map_err(|e| NexisError::Io {
                path: store_parent.to_path_buf(),
                source: e,
            })?;

        // Ingest files with deduplication
        let ingest_result = self.backend.ingest_directory(source_path, &store_path)?;
        
        // Build package metadata
        let mut files = Vec::new();
        let mut total_size = 0;

        for entry in walkdir::WalkDir::new(&store_path).follow_links(false) {
            let entry = entry.map_err(|e| NexisError::Io {
                path: store_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

            if entry.file_type().is_file() {
                let rel_path = entry.path().strip_prefix(&store_path)
                    .map_err(|_| NexisError::Store("Invalid file path".to_string()))?;
                
                let metadata = entry.metadata().map_err(|e| NexisError::Io {
                    path: entry.path().to_path_buf(),
                    source: e,
                })?;

                let file_hash = hash_file(entry.path())?;
                total_size += metadata.len();

                let symlink_target = if metadata.file_type().is_symlink() {
                    Some(fs::read_link(entry.path())
                        .map_err(|e| NexisError::Io {
                            path: entry.path().to_path_buf(),
                            source: e,
                        })?
                        .to_string_lossy()
                        .to_string())
                } else {
                    None
                };

                files.push(StoreFile {
                    relative_path: rel_path.to_string_lossy().to_string(),
                    hash: file_hash,
                    size: metadata.len(),
                    mode: get_file_mode(&metadata),
                    symlink_target,
                });
            }
        }

        let package = StorePackage {
            name: name.to_string(),
            version: version.to_string(),
            hash: hash.clone(),
            path: store_path,
            size: total_size,
            files,
            dependencies: Vec::new(), // TODO: Extract from package metadata
        };

        println!("Added package {} v{} to store (hash: {})", name, version, &hash[..8]);
        
        Ok(package)
    }

    fn get_package(&self, name: &str, version: &str) -> Result<Option<StorePackage>, NexisError> {
        // For now, we need to scan the store to find packages by name/version
        // In a real implementation, this would use the metadata store
        for entry in fs::read_dir(&self.config.store_path)
            .map_err(|e| NexisError::Io {
                path: self.config.store_path.clone(),
                source: e,
            })? 
        {
            let entry = entry.map_err(|e| NexisError::Io {
                path: self.config.store_path.clone(),
                source: e,
            })?;

            if let Some(dir_name) = entry.file_name().to_str() {
                if dir_name.ends_with(&format!("-{}", name)) {
                    let package_path = entry.path();
                    if let Ok(package) = self.load_package_from_path(&package_path, name, version) {
                        if package.name == name && package.version == version {
                            return Ok(Some(package));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn list_packages(&self) -> Result<Vec<StorePackage>, NexisError> {
        let mut packages = Vec::new();

        for shard_dir in self.layout.list_shards()? {
            for entry in fs::read_dir(&shard_dir)
                .map_err(|e| NexisError::Io {
                    path: shard_dir.clone(),
                    source: e,
                })? 
            {
                let entry = entry.map_err(|e| NexisError::Io {
                    path: shard_dir.clone(),
                    source: e,
                })?;

                if entry.file_type()
                    .map_err(|e| NexisError::Io {
                        path: entry.path(),
                        source: e,
                    })?
                    .is_dir() 
                {
                    // Parse package name from directory name (format: hash-name)
                    if let Some(dir_name) = entry.file_name().to_str() {
                        if let Some(dash_pos) = dir_name.find('-') {
                            let name = &dir_name[dash_pos + 1..];
                            // We don't have version info in the path, so use "unknown"
                            if let Ok(package) = self.load_package_from_path(&entry.path(), name, "unknown") {
                                packages.push(package);
                            }
                        }
                    }
                }
            }
        }

        Ok(packages)
    }

    fn remove_package(&mut self, name: &str, version: &str) -> Result<(), NexisError> {
        if let Some(package) = self.get_package(name, version)? {
            // In a full implementation, this would decrement refcounts in the metadata store
            // and only remove files when refcount reaches 0
            println!("Removing package {} v{}", name, version);
            
            // For now, just remove the directory
            fs::remove_dir_all(&package.path)
                .map_err(|e| NexisError::Io {
                    path: package.path,
                    source: e,
                })?;
        }

        Ok(())
    }

    fn get_stats(&self) -> Result<StoreStats, NexisError> {
        let packages = self.list_packages()?;
        let total_packages = packages.len() as u64;
        let total_files = packages.iter().map(|p| p.files.len() as u64).sum();
        let total_size = packages.iter().map(|p| p.size).sum();

        // Calculate deduplication ratio (simplified)
        let unique_hashes: std::collections::HashSet<_> = packages
            .iter()
            .flat_map(|p| p.files.iter().map(|f| &f.hash))
            .collect();

        let deduplicated_size = total_size; // TODO: Calculate actual deduplicated size
        let dedup_ratio = if total_size > 0 {
            (total_size - deduplicated_size) as f64 / total_size as f64
        } else {
            0.0
        };

        Ok(StoreStats {
            total_packages,
            total_files,
            total_size,
            deduplicated_size,
            dedup_ratio,
        })
    }

    fn gc(&mut self, dry_run: bool) -> Result<u64, NexisError> {
        println!("Running garbage collection (dry_run: {})", dry_run);
        
        // In a full implementation, this would:
        // 1. Mark all live packages based on current generations
        // 2. Find unreferenced packages/files
        // 3. Move them to trash directory
        // 4. Background workers delete trash contents
        
        // For now, return 0 bytes collected
        Ok(0)
    }

    fn verify(&self) -> Result<Vec<String>, NexisError> {
        let mut errors = Vec::new();
        let packages = self.list_packages()?;

        for package in packages {
            // Verify each file exists and matches its hash
            for file in &package.files {
                let file_path = package.path.join(&file.relative_path);
                
                if !file_path.exists() {
                    errors.push(format!("Missing file: {} in package {}", 
                                      file.relative_path, package.name));
                    continue;
                }

                match hash_file(&file_path) {
                    Ok(actual_hash) => {
                        if actual_hash != file.hash {
                            errors.push(format!("Hash mismatch: {} in package {} (expected: {}, actual: {})", 
                                              file.relative_path, package.name, file.hash, actual_hash));
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Cannot hash file: {} in package {}: {}", 
                                          file.relative_path, package.name, e));
                    }
                }
            }
        }

        Ok(errors)
    }
}

impl FileStore {
    /// Load an existing package from the store
    fn load_existing_package(&self, store_path: &Path, name: &str, version: &str, hash: &str) -> Result<StorePackage, NexisError> {
        println!("Package {} v{} already exists in store", name, version);
        self.load_package_from_path(store_path, name, version)
    }

    /// Load package metadata from a store path
    fn load_package_from_path(&self, store_path: &Path, name: &str, version: &str) -> Result<StorePackage, NexisError> {
        let mut files = Vec::new();
        let mut total_size = 0;

        for entry in walkdir::WalkDir::new(store_path).follow_links(false) {
            let entry = entry.map_err(|e| NexisError::Io {
                path: store_path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

            if entry.file_type().is_file() {
                let rel_path = entry.path().strip_prefix(store_path)
                    .map_err(|_| NexisError::Store("Invalid file path".to_string()))?;
                
                let metadata = entry.metadata().map_err(|e| NexisError::Io {
                    path: entry.path().to_path_buf(),
                    source: e,
                })?;

                let file_hash = hash_file(entry.path())?;
                total_size += metadata.len();

                files.push(StoreFile {
                    relative_path: rel_path.to_string_lossy().to_string(),
                    hash: file_hash,
                    size: metadata.len(),
                    mode: get_file_mode(&metadata),
                    symlink_target: None, // TODO: Handle symlinks
                });
            }
        }

        // Extract hash from directory name if available
        let hash = store_path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.find('-').map(|pos| &n[..pos]))
            .unwrap_or("unknown")
            .to_string();

        Ok(StorePackage {
            name: name.to_string(),
            version: version.to_string(),
            hash,
            path: store_path.to_path_buf(),
            size: total_size,
            files,
            dependencies: Vec::new(),
        })
    }
}

/// Detect filesystem type for a path
fn detect_filesystem(path: &Path) -> Result<StorageBackend, NexisError> {
    // Try to detect filesystem type
    // This is a simplified implementation - in practice you'd check /proc/mounts
    if let Ok(output) = std::process::Command::new("stat")
        .args(&["-f", "-c", "%T", path.to_str().unwrap_or("/")])
        .output() 
    {
        let fs_type = String::from_utf8_lossy(&output.stdout);
        match fs_type.trim() {
            "xfs" => Ok(StorageBackend::Xfs),
            "ext2/ext3" | "ext4" => Ok(StorageBackend::Ext4),
            _ => Ok(StorageBackend::Simple),
        }
    } else {
        // Fallback to simple backend
        Ok(StorageBackend::Simple)
    }
}

/// Get file mode from metadata (cross-platform helper)
#[cfg(unix)]
fn get_file_mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode()
}

#[cfg(not(unix))]
fn get_file_mode(_metadata: &std::fs::Metadata) -> u32 {
    0o644 // Default file permissions on non-Unix systems
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            store_path: PathBuf::from("/store"),
            shard_depth: 2,
            backend: StorageBackend::Simple,
            compression: false,
            parallel_workers: num_cpus::get(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = StoreConfig {
            store_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let store = FileStore::new(config).unwrap();
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_package_hash_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("test-package");
        fs::create_dir_all(&package_dir).unwrap();
        
        // Create a test file
        fs::write(package_dir.join("test.txt"), "hello world").unwrap();

        let config = StoreConfig {
            store_path: temp_dir.path().join("store"),
            ..Default::default()
        };
        let store = FileStore::new(config).unwrap();
        
        let hash1 = store.calculate_package_hash(&package_dir).unwrap();
        let hash2 = store.calculate_package_hash(&package_dir).unwrap();
        
        assert_eq!(hash1, hash2); // Hash should be deterministic
        assert!(hash1.len() > 0);
    }
}
