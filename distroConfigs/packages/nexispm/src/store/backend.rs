use crate::error::{Result, VaultError};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Configuration for different storage backends
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackendConfig {
    Filesystem { root_path: PathBuf },
    S3 { 
        bucket: String,
        region: String,
        access_key: Option<String>,
        secret_key: Option<String>,
        endpoint: Option<String>, // For S3-compatible services
    },
    Memory, // In-memory backend for testing
}

/// Trait defining the interface for storage backends
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store data at the given path
    async fn put(&self, path: &str, data: Bytes) -> Result<()>;
    
    /// Retrieve data from the given path
    async fn get(&self, path: &str) -> Result<Bytes>;
    
    /// Delete data at the given path
    async fn delete(&self, path: &str) -> Result<()>;
    
    /// Check if data exists at the given path
    async fn exists(&self, path: &str) -> Result<bool>;
    
    /// List all keys with the given prefix
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
    
    /// Get the size of data at the given path
    async fn size(&self, path: &str) -> Result<u64>;
    
    /// Create a backup of the entire backend
    async fn backup(&self, destination: &str) -> Result<()>;
    
    /// Restore from a backup
    async fn restore(&self, source: &str) -> Result<()>;
}

/// Filesystem-based storage backend
pub struct FilesystemBackend {
    root_path: PathBuf,
}

impl FilesystemBackend {
    pub fn new(root_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root_path)
            .map_err(|e| VaultError::StorageError(format!("Failed to create root directory: {}", e)))?;
        
        Ok(Self { root_path })
    }
    
    fn resolve_path(&self, path: &str) -> PathBuf {
        self.root_path.join(path)
    }
    
    async fn ensure_parent_dir(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await
                    .map_err(|e| VaultError::StorageError(format!("Failed to create directory: {}", e)))?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl StorageBackend for FilesystemBackend {
    async fn put(&self, path: &str, data: Bytes) -> Result<()> {
        let full_path = self.resolve_path(path);
        self.ensure_parent_dir(&full_path).await?;
        
        let mut file = fs::File::create(&full_path).await
            .map_err(|e| VaultError::StorageError(format!("Failed to create file: {}", e)))?;
        
        file.write_all(&data).await
            .map_err(|e| VaultError::StorageError(format!("Failed to write file: {}", e)))?;
        
        file.sync_all().await
            .map_err(|e| VaultError::StorageError(format!("Failed to sync file: {}", e)))?;
        
        Ok(())
    }
    
    async fn get(&self, path: &str) -> Result<Bytes> {
        let full_path = self.resolve_path(path);
        
        if !full_path.exists() {
            return Err(VaultError::NotFound(format!("File not found: {}", path)));
        }
        
        let mut file = fs::File::open(&full_path).await
            .map_err(|e| VaultError::StorageError(format!("Failed to open file: {}", e)))?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data).await
            .map_err(|e| VaultError::StorageError(format!("Failed to read file: {}", e)))?;
        
        Ok(Bytes::from(data))
    }
    
    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        
        if !full_path.exists() {
            return Ok(()); // Already deleted
        }
        
        if full_path.is_dir() {
            fs::remove_dir_all(&full_path).await
        } else {
            fs::remove_file(&full_path).await
        }
        .map_err(|e| VaultError::StorageError(format!("Failed to delete: {}", e)))?;
        
        Ok(())
    }
    
    async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.resolve_path(path);
        Ok(full_path.exists())
    }
    
    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let prefix_path = self.resolve_path(prefix);
        let mut results = Vec::new();
        
        if !prefix_path.exists() {
            return Ok(results);
        }
        
        self.collect_files(&prefix_path, prefix, &mut results).await?;
        results.sort();
        Ok(results)
    }
    
    async fn size(&self, path: &str) -> Result<u64> {
        let full_path = self.resolve_path(path);
        
        let metadata = fs::metadata(&full_path).await
            .map_err(|e| VaultError::StorageError(format!("Failed to get metadata: {}", e)))?;
        
        Ok(metadata.len())
    }
    
    async fn backup(&self, destination: &str) -> Result<()> {
        let dest_path = Path::new(destination);
        
        // Create destination directory
        fs::create_dir_all(dest_path).await
            .map_err(|e| VaultError::StorageError(format!("Failed to create backup directory: {}", e)))?;
        
        // Copy all files recursively
        self.copy_recursive(&self.root_path, dest_path).await
    }
    
    async fn restore(&self, source: &str) -> Result<()> {
        let source_path = Path::new(source);
        
        if !source_path.exists() {
            return Err(VaultError::NotFound(format!("Backup source not found: {}", source)));
        }
        
        // Clear existing data
        if self.root_path.exists() {
            fs::remove_dir_all(&self.root_path).await
                .map_err(|e| VaultError::StorageError(format!("Failed to clear existing data: {}", e)))?;
        }
        
        // Recreate root directory
        fs::create_dir_all(&self.root_path).await
            .map_err(|e| VaultError::StorageError(format!("Failed to create root directory: {}", e)))?;
        
        // Copy backup data
        self.copy_recursive(source_path, &self.root_path).await
    }
}

impl FilesystemBackend {
    async fn collect_files(&self, dir: &Path, prefix: &str, results: &mut Vec<String>) -> Result<()> {
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| VaultError::StorageError(format!("Failed to read directory: {}", e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| VaultError::StorageError(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            let relative_path = path.strip_prefix(&self.root_path)
                .map_err(|e| VaultError::StorageError(format!("Failed to compute relative path: {}", e)))?;
            
            let path_str = relative_path.to_string_lossy().to_string();
            
            if path.is_dir() {
                self.collect_files(&path, prefix, results).await?;
            } else if path_str.starts_with(prefix) {
                results.push(path_str);
            }
        }
        
        Ok(())
    }
    
    async fn copy_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        if src.is_dir() {
            fs::create_dir_all(dst).await
                .map_err(|e| VaultError::StorageError(format!("Failed to create directory: {}", e)))?;
            
            let mut entries = fs::read_dir(src).await
                .map_err(|e| VaultError::StorageError(format!("Failed to read source directory: {}", e)))?;
            
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| VaultError::StorageError(format!("Failed to read directory entry: {}", e)))? {
                
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());
                self.copy_recursive(&src_path, &dst_path).await?;
            }
        } else {
            fs::copy(src, dst).await
                .map_err(|e| VaultError::StorageError(format!("Failed to copy file: {}", e)))?;
        }
        
        Ok(())
    }
}

/// In-memory storage backend (primarily for testing)
pub struct MemoryBackend {
    data: tokio::sync::RwLock<HashMap<String, Bytes>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            data: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MemoryBackend {
    async fn put(&self, path: &str, data: Bytes) -> Result<()> {
        let mut store = self.data.write().await;
        store.insert(path.to_string(), data);
        Ok(())
    }
    
    async fn get(&self, path: &str) -> Result<Bytes> {
        let store = self.data.read().await;
        store.get(path)
            .cloned()
            .ok_or_else(|| VaultError::NotFound(format!("Key not found: {}", path)))
    }
    
    async fn delete(&self, path: &str) -> Result<()> {
        let mut store = self.data.write().await;
        store.remove(path);
        Ok(())
    }
    
    async fn exists(&self, path: &str) -> Result<bool> {
        let store = self.data.read().await;
        Ok(store.contains_key(path))
    }
    
    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let store = self.data.read().await;
        let mut results: Vec<String> = store.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        results.sort();
        Ok(results)
    }
    
    async fn size(&self, path: &str) -> Result<u64> {
        let store = self.data.read().await;
        store.get(path)
            .map(|data| data.len() as u64)
            .ok_or_else(|| VaultError::NotFound(format!("Key not found: {}", path)))
    }
    
    async fn backup(&self, destination: &str) -> Result<()> {
        // For memory backend, we'll serialize to a file
        let store = self.data.read().await;
        let serialized = bincode::serialize(&*store)
            .map_err(|e| VaultError::SerializationError(format!("Failed to serialize data: {}", e)))?;
        
        fs::write(destination, serialized).await
            .map_err(|e| VaultError::StorageError(format!("Failed to write backup: {}", e)))?;
        
        Ok(())
    }
    
    async fn restore(&self, source: &str) -> Result<()> {
        let data = fs::read(source).await
            .map_err(|e| VaultError::StorageError(format!("Failed to read backup: {}", e)))?;
        
        let restored: HashMap<String, Bytes> = bincode::deserialize(&data)
            .map_err(|e| VaultError::SerializationError(format!("Failed to deserialize backup: {}", e)))?;
        
        let mut store = self.data.write().await;
        *store = restored;
        
        Ok(())
    }
}

/// S3-compatible storage backend
#[cfg(feature = "s3")]
pub mod s3 {
    use super::*;
    use aws_sdk_s3::{Client, Config, Credentials, Region};
    use aws_sdk_s3::primitives::ByteStream;
    use aws_types::SdkConfig;
    
    pub struct S3Backend {
        client: Client,
        bucket: String,
    }
    
    impl S3Backend {
        pub async fn new(config: S3Config) -> Result<Self> {
            let aws_config = if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
                let credentials = Credentials::new(
                    access_key,
                    secret_key,
                    None,
                    None,
                    "vault-s3-backend",
                );
                
                let mut sdk_config = SdkConfig::builder()
                    .credentials_provider(credentials)
                    .region(Region::new(config.region.clone()));
                
                if let Some(endpoint) = &config.endpoint {
                    sdk_config = sdk_config.endpoint_url(endpoint);
                }
                
                sdk_config.build()
            } else {
                // Use default credential chain
                aws_config::load_from_env().await
            };
            
            let s3_config = Config::from(&aws_config);
            let client = Client::from_conf(s3_config);
            
            Ok(Self {
                client,
                bucket: config.bucket,
            })
        }
    }
    
    #[derive(Debug, Clone)]
    pub struct S3Config {
        pub bucket: String,
        pub region: String,
        pub access_key: Option<String>,
        pub secret_key: Option<String>,
        pub endpoint: Option<String>,
    }
    
    #[async_trait]
    impl StorageBackend for S3Backend {
        async fn put(&self, path: &str, data: Bytes) -> Result<()> {
            let body = ByteStream::from(data);
            
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(path)
                .body(body)
                .send()
                .await
                .map_err(|e| VaultError::StorageError(format!("S3 put failed: {}", e)))?;
            
            Ok(())
        }
        
        async fn get(&self, path: &str) -> Result<Bytes> {
            let response = self.client
                .get_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await
                .map_err(|e| VaultError::StorageError(format!("S3 get failed: {}", e)))?;
            
            let data = response.body.collect().await
                .map_err(|e| VaultError::StorageError(format!("Failed to read S3 response: {}", e)))?;
            
            Ok(data.into_bytes())
        }
        
        async fn delete(&self, path: &str) -> Result<()> {
            self.client
                .delete_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await
                .map_err(|e| VaultError::StorageError(format!("S3 delete failed: {}", e)))?;
            
            Ok(())
        }
        
        async fn exists(&self, path: &str) -> Result<bool> {
            match self.client
                .head_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
        
        async fn list(&self, prefix: &str) -> Result<Vec<String>> {
            let mut results = Vec::new();
            let mut continuation_token = None;
            
            loop {
                let mut request = self.client
                    .list_objects_v2()
                    .bucket(&self.bucket)
                    .prefix(prefix);
                
                if let Some(token) = continuation_token {
                    request = request.continuation_token(token);
                }
                
                let response = request.send().await
                    .map_err(|e| VaultError::StorageError(format!("S3 list failed: {}", e)))?;
                
                if let Some(contents) = response.contents {
                    for object in contents {
                        if let Some(key) = object.key {
                            results.push(key);
                        }
                    }
                }
                
                if response.is_truncated == Some(true) {
                    continuation_token = response.next_continuation_token;
                } else {
                    break;
                }
            }
            
            results.sort();
            Ok(results)
        }
        
        async fn size(&self, path: &str) -> Result<u64> {
            let response = self.client
                .head_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await
                .map_err(|e| VaultError::StorageError(format!("S3 head failed: {}", e)))?;
            
            Ok(response.content_length.unwrap_or(0) as u64)
        }
        
        async fn backup(&self, destination: &str) -> Result<()> {
            // S3 backup would involve copying to another bucket or exporting
            // This is a simplified implementation
            Err(VaultError::NotImplemented("S3 backup not implemented".to_string()))
        }
        
        async fn restore(&self, source: &str) -> Result<()> {
            // S3 restore would involve importing from backup
            // This is a simplified implementation
            Err(VaultError::NotImplemented("S3 restore not implemented".to_string()))
        }
    }
}

/// Factory function to create storage backends from configuration
pub async fn create_backend(config: BackendConfig) -> Result<Box<dyn StorageBackend>> {
    match config {
        BackendConfig::Filesystem { root_path } => {
            let backend = FilesystemBackend::new(root_path)?;
            Ok(Box::new(backend))
        }
        BackendConfig::Memory => {
            let backend = MemoryBackend::new();
            Ok(Box::new(backend))
        }
        #[cfg(feature = "s3")]
        BackendConfig::S3 { bucket, region, access_key, secret_key, endpoint } => {
            let s3_config = s3::S3Config {
                bucket,
                region,
                access_key,
                secret_key,
                endpoint,
            };
            let backend = s3::S3Backend::new(s3_config).await?;
            Ok(Box::new(backend))
        }
        #[cfg(not(feature = "s3"))]
        BackendConfig::S3 { .. } => {
            Err(VaultError::ConfigurationError(
                "S3 backend not available. Enable 's3' feature.".to_string()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_filesystem_backend() {
        let temp_dir = TempDir::new().unwrap();
        let backend = FilesystemBackend::new(temp_dir.path().to_path_buf()).unwrap();
        
        let test_data = Bytes::from("hello world");
        
        // Test put/get
        backend.put("test.txt", test_data.clone()).await.unwrap();
        let retrieved = backend.get("test.txt").await.unwrap();
        assert_eq!(retrieved, test_data);
        
        // Test exists
        assert!(backend.exists("test.txt").await.unwrap());
        assert!(!backend.exists("nonexistent.txt").await.unwrap());
        
        // Test size
        let size = backend.size("test.txt").await.unwrap();
        assert_eq!(size, test_data.len() as u64);
        
        // Test list
        backend.put("dir/file1.txt", Bytes::from("data1")).await.unwrap();
        backend.put("dir/file2.txt", Bytes::from("data2")).await.unwrap();
        let files = backend.list("dir/").await.unwrap();
        assert_eq!(files.len(), 2);
        
        // Test delete
        backend.delete("test.txt").await.unwrap();
        assert!(!backend.exists("test.txt").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_memory_backend() {
        let backend = MemoryBackend::new();
        let test_data = Bytes::from("memory test");
        
        // Test put/get
        backend.put("key1", test_data.clone()).await.unwrap();
        let retrieved = backend.get("key1").await.unwrap();
        assert_eq!(retrieved, test_data);
        
        // Test exists
        assert!(backend.exists("key1").await.unwrap());
        assert!(!backend.exists("key2").await.unwrap());
        
        // Test list
        backend.put("prefix_a", Bytes::from("a")).await.unwrap();
        backend.put("prefix_b", Bytes::from("b")).await.unwrap();
        backend.put("other", Bytes::from("other")).await.unwrap();
        
        let prefixed = backend.list("prefix_").await.unwrap();
        assert_eq!(prefixed.len(), 2);
        assert!(prefixed.contains(&"prefix_a".to_string()));
        assert!(prefixed.contains(&"prefix_b".to_string()));
        
        // Test delete
        backend.delete("key1").await.unwrap();
        assert!(!backend.exists("key1").await.unwrap());
    }
}
