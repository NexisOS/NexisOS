use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Read, BufReader};
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};
use sha2::{Sha256, Digest};
use walkdir::WalkDir;
use log::{info, debug, warn};

use crate::meta::MetaStore;
use crate::store::backend::StorageBackend;
use crate::store::layout::StoreLayout;

/// Manages package ingestion with deduplication
pub struct PackageIngestor {
    layout: StoreLayout,
    meta_store: Box<dyn MetaStore>,
    backend: Box<dyn StorageBackend>,
}

/// Result of a package ingestion operation
#[derive(Debug, Clone)]
pub struct IngestResult {
    pub package_hash: String,
    pub store_path: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
    pub deduplicated_files: usize,
    pub new_files: usize,
}

/// File metadata for deduplication tracking
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub relative_path: PathBuf,
    pub hash: String,
    pub size: u64,
    pub is_executable: bool,
}

impl PackageIngestor {
    /// Create a new package ingestor
    pub fn new(
        layout: StoreLayout,
        meta_store: Box<dyn MetaStore>,
        backend: Box<dyn StorageBackend>,
    ) -> Self {
        Self {
            layout,
            meta_store,
            backend,
        }
    }

    /// Ingest a package from source directory with deduplication
    pub fn ingest_package(
        &mut self,
        name: &str,
        version: &str,
        source_path: &Path,
    ) -> Result<IngestResult> {
        info!("Starting ingestion of package {}-{} from {}", name, version, source_path.display());

        // Scan source directory and compute file hashes
        let file_entries = self.scan_source_directory(source_path)?;
        
        // Compute package hash from all file hashes
        let package_hash = self.compute_package_hash(&file_entries)?;
        
        // Determine destination path in store
        let store_path = self.layout.package_path(&package_hash, name);
        
        // Check if package already exists (full deduplication)
        if store_path.exists() {
            let existing_refcount = self.meta_store.get_refcount(&package_hash)?;
            self.meta_store.increment_refcount(&package_hash)?;
            
            info!("Package {}-{} already exists (hash: {}), incremented refcount to {}", 
                  name, version, package_hash, existing_refcount + 1);
                  
            return Ok(IngestResult {
                package_hash: package_hash.clone(),
                store_path,
                size_bytes: self.calculate_total_size(&file_entries),
                file_count: file_entries.len(),
                deduplicated_files: file_entries.len(),
                new_files: 0,
            });
        }

        // Create package directory
        fs::create_dir_all(&store_path)
            .with_context(|| format!("Failed to create package directory: {}", store_path.display()))?;

        // Ingest files with per-file deduplication
        let mut deduplicated_count = 0;
        let mut new_files_count = 0;
        
        for file_entry in &file_entries {
            let source_file = source_path.join(&file_entry.relative_path);
            let dest_file = store_path.join(&file_entry.relative_path);
            
            // Create parent directory if needed
            if let Some(parent) = dest_file.parent() {
                fs::create_dir_all(parent)?;
            }
            
            // Check if we can deduplicate this file
            if let Some(existing_path) = self.find_existing_file(&file_entry.hash)? {
                // Deduplicate using backend (reflink/hardlink)
                self.backend.deduplicate_file(&existing_path, &dest_file)?;
                deduplicated_count += 1;
                debug!("Deduplicated file: {} -> {}", source_file.display(), dest_file.display());
            } else {
                // Copy new file
                fs::copy(&source_file, &dest_file)
                    .with_context(|| format!("Failed to copy file: {}", source_file.display()))?;
                
                // Preserve executable permissions
                if file_entry.is_executable {
                    self.set_executable(&dest_file)?;
                }
                
                new_files_count += 1;
                debug!("Copied new file: {} -> {}", source_file.display(), dest_file.display());
            }
            
            // Track file in metadata store
            self.meta_store.track_file(&file_entry.hash, &dest_file)?;
        }

        // Record package metadata
        self.meta_store.store_package_metadata(
            &package_hash,
            name,
            version,
            &store_path,
            &file_entries,
        )?;
        
        // Set initial refcount
        self.meta_store.set_refcount(&package_hash, 1)?;

        let result = IngestResult {
            package_hash: package_hash.clone(),
            store_path,
            size_bytes: self.calculate_total_size(&file_entries),
            file_count: file_entries.len(),
            deduplicated_files: deduplicated_count,
            new_files: new_files_count,
        };

        info!("Package ingestion complete: {} files ({} deduplicated, {} new), hash: {}", 
              result.file_count, result.deduplicated_files, result.new_files, package_hash);

        Ok(result)
    }

    /// Scan source directory and compute hashes for all files
    fn scan_source_directory(&self, source_path: &Path) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();
        
        for entry in WalkDir::new(source_path) {
            let entry = entry?;
            let path = entry.path();
            
            // Skip directories
            if path.is_dir() {
                continue;
            }
            
            let relative_path = path.strip_prefix(source_path)
                .context("Failed to compute relative path")?
                .to_path_buf();
            
            let metadata = entry.metadata()?;
            let size = metadata.len();
            
            // Check if file is executable
            #[cfg(unix)]
            let is_executable = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let is_executable = false;
            
            // Compute file hash
            let hash = self.compute_file_hash(path)?;
            
            entries.push(FileEntry {
                relative_path,
                hash,
                size,
                is_executable,
            });
        }
        
        // Sort for deterministic package hashing
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        
        Ok(entries)
    }

    /// Compute hash of a single file
    fn compute_file_hash(&self, path: &Path) -> Result<String> {
        let mut hasher = Sha256::new();
        let mut file = BufReader::new(fs::File::open(path)?);
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Compute package hash from all file entries
    fn compute_package_hash(&self, file_entries: &[FileEntry]) -> Result<String> {
        let mut hasher = Sha256::new();
        
        for entry in file_entries {
            // Include path and file hash for deterministic package hash
            hasher.update(entry.relative_path.to_string_lossy().as_bytes());
            hasher.update(b"|");
            hasher.update(entry.hash.as_bytes());
            hasher.update(b"\n");
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Find existing file with the same hash for deduplication
    fn find_existing_file(&self, file_hash: &str) -> Result<Option<PathBuf>> {
        self.meta_store.find_file_by_hash(file_hash)
    }

    /// Set executable permissions on a file
    #[cfg(unix)]
    fn set_executable(&self, path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(perms.mode() | 0o755);
        fs::set_permissions(path, perms)?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn set_executable(&self, _path: &Path) -> Result<()> {
        // No-op on non-Unix systems
        Ok(())
    }

    /// Calculate total size of all files
    fn calculate_total_size(&self, file_entries: &[FileEntry]) -> u64 {
        file_entries.iter().map(|entry| entry.size).sum()
    }

    /// Remove a package and update refcounts
    pub fn remove_package(&mut self, package_hash: &str) -> Result<bool> {
        let refcount = self.meta_store.get_refcount(package_hash)?;
        
        if refcount <= 1 {
            // Last reference, actually remove package
            if let Some(store_path) = self.meta_store.get_package_path(package_hash)? {
                // Get file list for this package
                let file_hashes = self.meta_store.get_package_files(package_hash)?;
                
                // Decrement refcounts for all files
                for file_hash in file_hashes {
                    self.meta_store.decrement_file_refcount(&file_hash)?;
                }
                
                // Mark package for deletion (GC will handle actual removal)
                self.meta_store.mark_for_deletion(package_hash)?;
                
                info!("Package {} marked for deletion (was last reference)", package_hash);
                return Ok(true);
            }
        } else {
            // Just decrement refcount
            self.meta_store.decrement_refcount(package_hash)?;
            info!("Package {} refcount decremented to {}", package_hash, refcount - 1);
        }
        
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_file_hash_computation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"hello world").unwrap();
        
        let layout = StoreLayout::new(temp_dir.path());
        let meta_store = Box::new(crate::meta::sled_store::SledMetaStore::new(temp_dir.path()).unwrap());
        let backend = Box::new(crate::store::backend::HardlinkBackend::new());
        let ingestor = PackageIngestor::new(layout, meta_store, backend);
        
        let hash = ingestor.compute_file_hash(&test_file).unwrap();
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_package_hash_deterministic() {
        let entries = vec![
            FileEntry {
                relative_path: PathBuf::from("file1.txt"),
                hash: "hash1".to_string(),
                size: 100,
                is_executable: false,
            },
            FileEntry {
                relative_path: PathBuf::from("file2.txt"),
                hash: "hash2".to_string(),
                size: 200,
                is_executable: true,
            },
        ];
        
        let temp_dir = TempDir::new().unwrap();
        let layout = StoreLayout::new(temp_dir.path());
        let meta_store = Box::new(crate::meta::sled_store::SledMetaStore::new(temp_dir.path()).unwrap());
        let backend = Box::new(crate::store::backend::HardlinkBackend::new());
        let ingestor = PackageIngestor::new(layout, meta_store, backend);
        
        let hash1 = ingestor.compute_package_hash(&entries).unwrap();
        let hash2 = ingestor.compute_package_hash(&entries).unwrap();
        
        assert_eq!(hash1, hash2);
    }
}
