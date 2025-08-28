use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::process::Command;
use anyhow::{Context, Result, anyhow};
use log::{debug, warn, info};

/// Trait defining storage backend operations for different filesystems
pub trait StorageBackend: Send + Sync {
    /// Deduplicate a file by creating a reflink/hardlink from existing to new location
    fn deduplicate_file(&self, source: &Path, dest: &Path) -> Result<()>;
    
    /// Check if the backend supports this filesystem
    fn supports_filesystem(&self, path: &Path) -> Result<bool>;
    
    /// Get the backend type name for logging
    fn backend_type(&self) -> &'static str;
    
    /// Initialize any backend-specific setup
    fn initialize(&self, store_root: &Path) -> Result<()> {
        let _ = store_root; // Default implementation does nothing
        Ok(())
    }
    
    /// Cleanup backend resources
    fn cleanup(&self) -> Result<()> {
        Ok(()) // Default implementation does nothing
    }
    
    /// Get estimated space savings from deduplication
    fn get_space_savings(&self, path: &Path) -> Result<SpaceSavings> {
        let _ = path; // Default implementation returns zero savings
        Ok(SpaceSavings {
            total_size: 0,
            unique_size: 0,
            deduplicated_size: 0,
        })
    }
}

/// Space savings information from deduplication
#[derive(Debug, Clone, Default)]
pub struct SpaceSavings {
    pub total_size: u64,
    pub unique_size: u64,
    pub deduplicated_size: u64,
}

impl SpaceSavings {
    pub fn savings_ratio(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            (self.deduplicated_size as f64) / (self.total_size as f64)
        }
    }
}

/// Reflink-based backend for XFS and other copy-on-write filesystems
pub struct ReflinkBackend {
    verify_support: bool,
}

impl ReflinkBackend {
    pub fn new() -> Self {
        Self {
            verify_support: true,
        }
    }

    pub fn new_unchecked() -> Self {
        Self {
            verify_support: false,
        }
    }

    /// Check if reflink is supported by attempting a test operation
    fn test_reflink_support(&self, path: &Path) -> Result<bool> {
        let test_dir = path.join(".reflink_test");
        
        // Clean up any existing test
        let _ = fs::remove_dir_all(&test_dir);
        
        fs::create_dir_all(&test_dir)?;
        
        let source = test_dir.join("source");
        let dest = test_dir.join("dest");
        
        // Create test file
        fs::write(&source, b"reflink test")?;
        
        // Try reflink
        let result = Command::new("cp")
            .arg("--reflink=always")
            .arg(&source)
            .arg(&dest)
            .status();
        
        // Cleanup
        let _ = fs::remove_dir_all(&test_dir);
        
        match result {
            Ok(status) => Ok(status.success()),
            Err(_) => Ok(false),
        }
    }

    /// Perform reflink operation using cp --reflink
    fn reflink_file(&self, source: &Path, dest: &Path) -> Result<()> {
        let status = Command::new("cp")
            .arg("--reflink=always")
            .arg(source)
            .arg(dest)
            .status()
            .context("Failed to execute cp command for reflink")?;

        if !status.success() {
            return Err(anyhow!("Reflink operation failed for {} -> {}", 
                              source.display(), dest.display()));
        }

        debug!("Reflinked file: {} -> {}", source.display(), dest.display());
        Ok(())
    }
}

impl StorageBackend for ReflinkBackend {
    fn deduplicate_file(&self, source: &Path, dest: &Path) -> Result<()> {
        if !source.exists() {
            return Err(anyhow!("Source file does not exist: {}", source.display()));
        }

        self.reflink_file(source, dest)
            .with_context(|| format!("Failed to reflink {} to {}", 
                                    source.display(), dest.display()))
    }

    fn supports_filesystem(&self, path: &Path) -> Result<bool> {
        if !self.verify_support {
            return Ok(true);
        }

        self.test_reflink_support(path)
            .context("Failed to test reflink support")
    }

    fn backend_type(&self) -> &'static str {
        "reflink"
    }

    fn initialize(&self, store_root: &Path) -> Result<()> {
        if self.verify_support && !self.supports_filesystem(store_root)? {
            return Err(anyhow!("Reflink not supported on filesystem at {}", 
                              store_root.display()));
        }
        
        info!("Reflink backend initialized for {}", store_root.display());
        Ok(())
    }

    fn get_space_savings(&self, path: &Path) -> Result<SpaceSavings> {
        // For reflink, we'd need to analyze extent sharing
        // This is a simplified implementation
        let total_size = get_directory_size(path)?;
        
        Ok(SpaceSavings {
            total_size,
            unique_size: total_size, // Conservative estimate
            deduplicated_size: 0,
        })
    }
}

/// Hardlink-based backend for traditional filesystems like ext4
pub struct HardlinkBackend {
    max_links_per_file: u32,
}

impl HardlinkBackend {
    pub fn new() -> Self {
        Self {
            max_links_per_file: 1000, // Conservative limit
        }
    }

    pub fn with_max_links(max_links: u32) -> Self {
        Self {
            max_links_per_file: max_links,
        }
    }

    /// Create a hardlink from source to dest
    fn hardlink_file(&self, source: &Path, dest: &Path) -> Result<()> {
        // Check if we're approaching link limits
        let link_count = self.get_link_count(source)?;
        if link_count >= self.max_links_per_file {
            warn!("File {} has {} hardlinks, approaching limit. Copying instead.", 
                  source.display(), link_count);
            return self.copy_file(source, dest);
        }

        fs::hard_link(source, dest)
            .with_context(|| format!("Failed to create hardlink {} -> {}", 
                                    source.display(), dest.display()))?;

        debug!("Hardlinked file: {} -> {} (links: {})", 
               source.display(), dest.display(), link_count + 1);
        Ok(())
    }

    /// Get the current hardlink count for a file
    fn get_link_count(&self, path: &Path) -> Result<u32> {
        let metadata = fs::metadata(path)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(metadata.nlink() as u32)
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix, assume single link
            Ok(1)
        }
    }

    /// Fallback to regular copy when hardlinking fails
    fn copy_file(&self, source: &Path, dest: &Path) -> Result<()> {
        fs::copy(source, dest)
            .with_context(|| format!("Failed to copy file {} -> {}", 
                                    source.display(), dest.display()))?;
        Ok(())
    }
}

impl StorageBackend for HardlinkBackend {
    fn deduplicate_file(&self, source: &Path, dest: &Path) -> Result<()> {
        if !source.exists() {
            return Err(anyhow!("Source file does not exist: {}", source.display()));
        }

        // Try hardlink first, fall back to copy if needed
        self.hardlink_file(source, dest)
            .or_else(|_| {
                warn!("Hardlink failed, falling back to copy for {} -> {}", 
                      source.display(), dest.display());
                self.copy_file(source, dest)
            })
    }

    fn supports_filesystem(&self, _path: &Path) -> Result<bool> {
        // Hardlinks are supported on most filesystems
        Ok(true)
    }

    fn backend_type(&self) -> &'static str {
        "hardlink"
    }

    fn initialize(&self, store_root: &Path) -> Result<()> {
        info!("Hardlink backend initialized for {} (max links: {})", 
              store_root.display(), self.max_links_per_file);
        Ok(())
    }

    fn get_space_savings(&self, path: &Path) -> Result<SpaceSavings> {
        let mut total_size = 0u64;
        let mut unique_size = 0u64;
        let mut seen_inodes = std::collections::HashSet::new();

        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            let metadata = entry.metadata()?;
            
            if metadata.is_file() {
                total_size += metadata.len();
                
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    let inode = metadata.ino();
                    if seen_inodes.insert(inode) {
                        unique_size += metadata.len();
                    }
                }
                #[cfg(not(unix))]
                {
                    unique_size += metadata.len();
                }
            }
        }

        Ok(SpaceSavings {
            total_size,
            unique_size,
            deduplicated_size: total_size - unique_size,
        })
    }
}

/// Auto-detecting backend that chooses the best option for the filesystem
pub struct AutoBackend {
    reflink: ReflinkBackend,
    hardlink: HardlinkBackend,
    selected: Option<BackendType>,
}

#[derive(Debug, Clone, Copy)]
enum BackendType {
    Reflink,
    Hardlink,
}

impl AutoBackend {
    pub fn new() -> Self {
        Self {
            reflink: ReflinkBackend::new(),
            hardlink: HardlinkBackend::new(),
            selected: None,
        }
    }

    fn detect_best_backend(&mut self, path: &Path) -> Result<BackendType> {
        if let Some(backend) = self.selected {
            return Ok(backend);
        }

        // Try reflink first (more efficient)
        if self.reflink.supports_filesystem(path)? {
            info!("Auto-detected reflink support for {}", path.display());
            self.selected = Some(BackendType::Reflink);
            Ok(BackendType::Reflink)
        } else {
            info!("Reflink not supported, using hardlink backend for {}", path.display());
            self.selected = Some(BackendType::Hardlink);
            Ok(BackendType::Hardlink)
        }
    }
}

impl StorageBackend for AutoBackend {
    fn deduplicate_file(&self, source: &Path, dest: &Path) -> Result<()> {
        let backend = self.selected.ok_or_else(|| {
            anyhow!("Backend not initialized - call initialize() first")
        })?;

        match backend {
            BackendType::Reflink => self.reflink.deduplicate_file(source, dest),
            BackendType::Hardlink => self.hardlink.deduplicate_file(source, dest),
        }
    }

    fn supports_filesystem(&self, path: &Path) -> Result<bool> {
        // Auto backend supports any filesystem by falling back appropriately
        Ok(self.reflink.supports_filesystem(path)? || 
           self.hardlink.supports_filesystem(path)?)
    }

    fn backend_type(&self) -> &'static str {
        match self.selected {
            Some(BackendType::Reflink) => "auto-reflink",
            Some(BackendType::Hardlink) => "auto-hardlink", 
            None => "auto-uninitialized",
        }
    }

    fn initialize(&mut self, store_root: &Path) -> Result<()> {
        let backend = self.detect_best_backend(store_root)?;
        
        match backend {
            BackendType::Reflink => self.reflink.initialize(store_root),
            BackendType::Hardlink => self.hardlink.initialize(store_root),
        }
    }

    fn get_space_savings(&self, path: &Path) -> Result<SpaceSavings> {
        match self.selected {
            Some(BackendType::Reflink) => self.reflink.get_space_savings(path),
            Some(BackendType::Hardlink) => self.hardlink.get_space_savings(path),
            None => Ok(SpaceSavings::default()),
        }
    }
}

/// Utility function to get directory size recursively
fn get_directory_size(path: &Path) -> Result<u64> {
    let mut total_size = 0u64;
    
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total_size += entry.metadata()?.len();
        }
    }
    
    Ok(total_size)
}

/// Factory function to create appropriate backend based on configuration
pub fn create_backend(backend_type: &str, store_root: &Path) -> Result<Box<dyn StorageBackend>> {
    match backend_type.to_lowercase().as_str() {
        "reflink" => {
            let mut backend = Box::new(ReflinkBackend::new());
            backend.initialize(store_root)?;
            Ok(backend)
        },
        "hardlink" => {
            let mut backend = Box::new(HardlinkBackend::new());
            backend.initialize(store_root)?;
            Ok(backend)
        },
        "auto" | "" => {
            let mut backend = Box::new(AutoBackend::new());
            backend.initialize(store_root)?;
            Ok(backend)
        },
        _ => Err(anyhow!("Unsupported backend type: {}", backend_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hardlink_backend() {
        let temp_dir = TempDir::new().unwrap();
        let backend = HardlinkBackend::new();
        
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");
        
        fs::write(&source, b"test content").unwrap();
        
        backend.deduplicate_file(&source, &dest).unwrap();
        
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "test content");
    }

    #[test]
    fn test_backend_factory() {
        let temp_dir = TempDir::new().unwrap();
        
        let backend = create_backend("hardlink", temp_dir.path()).unwrap();
        assert_eq!(backend.backend_type(), "hardlink");
        
        let backend = create_backend("auto", temp_dir.path()).unwrap();
        assert!(backend.backend_type().starts_with("auto"));
    }

    #[test]
    fn test_space_savings() {
        let savings = SpaceSavings {
            total_size: 1000,
            unique_size: 600,
            deduplicated_size: 400,
        };
        
        assert_eq!(savings.savings_ratio(), 0.4);
    }
}
