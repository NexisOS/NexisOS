use crate::error::{Result, VaultError};
use crate::store::backend::StorageBackend;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// File metadata stored in the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: Uuid,
    pub path: String,
    pub original_name: String,
    pub size: u64,
    pub mime_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub version: u64,
    pub checksum: String, // SHA-256 hash
    pub encryption: EncryptionMetadata,
    pub tags: HashSet<String>,
    pub custom_attributes: HashMap<String, String>,
}

/// Encryption metadata for files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    pub algorithm: String,
    pub key_id: String,
    pub nonce: Vec<u8>,
    pub is_compressed: bool,
}

/// Version information for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub size: u64,
    pub checksum: String,
    pub comment: Option<String>,
}

/// Search index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    pub file_id: Uuid,
    pub content_tokens: Vec<String>,
    pub path_tokens: Vec<String>,
    pub tags: HashSet<String>,
    pub mime_type: Option<String>,
    pub last_indexed: DateTime<Utc>,
}

/// Statistics about the vault
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultStats {
    pub total_files: u64,
    pub total_size: u64,
    pub total_encrypted_size: u64,
    pub compression_ratio: f64,
    pub file_types: HashMap<String, u64>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Configuration for metadata store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub enable_versioning: bool,
    pub max_versions: Option<u32>,
    pub enable_search_index: bool,
    pub auto_backup_interval: Option<u64>, // seconds
    pub compress_metadata: bool,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            enable_versioning: true,
            max_versions: Some(10),
            enable_search_index: true,
            auto_backup_interval: Some(3600), // 1 hour
            compress_metadata: true,
        }
    }
}

/// In-memory metadata cache for fast access
#[derive(Debug, Default)]
struct MetadataCache {
    files: HashMap<Uuid, FileMetadata>,
    path_to_id: HashMap<String, Uuid>,
    versions: HashMap<Uuid, Vec<VersionInfo>>,
    search_index: HashMap<Uuid, SearchIndex>,
    stats: VaultStats,
    dirty: bool,
}

/// Main metadata store implementation
pub struct MetadataStore {
    backend: Arc<dyn StorageBackend>,
    config: MetadataConfig,
    cache: Arc<RwLock<MetadataCache>>,
    metadata_key: String,
}

impl MetadataStore {
    const METADATA_PREFIX: &'static str = "meta/";
    const FILES_KEY: &'static str = "meta/files.json";
    const VERSIONS_KEY: &'static str = "meta/versions.json";
    const SEARCH_INDEX_KEY: &'static str = "meta/search_index.json";
    const STATS_KEY: &'static str = "meta/stats.json";
    const CONFIG_KEY: &'static str = "meta/config.json";

    pub async fn new(
        backend: Arc<dyn StorageBackend>,
        config: MetadataConfig,
    ) -> Result<Self> {
        let store = Self {
            backend,
            config,
            cache: Arc::new(RwLock::new(MetadataCache::default())),
            metadata_key: Self::METADATA_PREFIX.to_string(),
        };

        // Load existing metadata
        store.load_metadata().await?;
        
        Ok(store)
    }

    /// Add a new file to the metadata store
    pub async fn add_file(&self, metadata: FileMetadata) -> Result<()> {
        let mut cache = self.cache.write().await;
        
        // Check for path conflicts
        if let Some(existing_id) = cache.path_to_id.get(&metadata.path) {
            if *existing_id != metadata.id {
                return Err(VaultError::ConflictError(
                    format!("File already exists at path: {}", metadata.path)
                ));
            }
        }

        // Add file metadata
        cache.path_to_id.insert(metadata.path.clone(), metadata.id);
        cache.files.insert(metadata.id, metadata.clone());

        // Initialize version history if versioning is enabled
        if self.config.enable_versioning {
            let version_info = VersionInfo {
                version: metadata.version,
                created_at: metadata.created_at,
                size: metadata.size,
                checksum: metadata.checksum.clone(),
                comment: None,
            };
            cache.versions.insert(metadata.id, vec![version_info]);
        }

        // Update statistics
        self.update_stats(&mut cache, &metadata, true).await;
        cache.dirty = true;

        Ok(())
    }

    /// Get file metadata by ID
    pub async fn get_file(&self, file_id: &Uuid) -> Result<FileMetadata> {
        let cache = self.cache.read().await;
        cache.files.get(file_id)
            .cloned()
            .ok_or_else(|| VaultError::NotFound(format!("File not found: {}", file_id)))
    }

    /// Get file metadata by path
    pub async fn get_file_by_path(&self, path: &str) -> Result<FileMetadata> {
        let cache = self.cache.read().await;
        let file_id = cache.path_to_id.get(path)
            .ok_or_else(|| VaultError::NotFound(format!("File not found at path: {}", path)))?;
        
        cache.files.get(file_id)
            .cloned()
            .ok_or_else(|| VaultError::NotFound(format!("File metadata corrupted for: {}", path)))
    }

    /// Update file metadata
    pub async fn update_file(&self, file_id: &Uuid, metadata: FileMetadata) -> Result<()> {
        let mut cache = self.cache.write().await;
        
        let old_metadata = cache.files.get(file_id)
            .ok_or_else(|| VaultError::NotFound(format!("File not found: {}", file_id)))?
            .clone();

        // Update path mapping if path changed
        if old_metadata.path != metadata.path {
            cache.path_to_id.remove(&old_metadata.path);
            cache.path_to_id.insert(metadata.path.clone(), metadata.id);
        }

        // Add new version if versioning is enabled and content changed
        if self.config.enable_versioning && old_metadata.checksum != metadata.checksum {
            self.add_version(&mut cache, file_id, &metadata).await?;
        }

        // Update file metadata
        cache.files.insert(*file_id, metadata.clone());

        // Update statistics
        self.update_stats(&mut cache, &old_metadata, false).await;
        self.update_stats(&mut cache, &metadata, true).await;
        cache.dirty = true;

        Ok(())
    }

    /// Delete file metadata
    pub async fn delete_file(&self, file_id: &Uuid) -> Result<FileMetadata> {
        let mut cache = self.cache.write().await;
        
        let metadata = cache.files.remove(file_id)
            .ok_or_else(|| VaultError::NotFound(format!("File not found: {}", file_id)))?;

        // Remove path mapping
        cache.path_to_id.remove(&metadata.path);

        // Remove versions
        cache.versions.remove(file_id);

        // Remove search index
        cache.search_index.remove(file_id);

        // Update statistics
        self.update_stats(&mut cache, &metadata, false).await;
        cache.dirty = true;

        Ok(metadata)
    }

    /// List all files with optional filtering
    pub async fn list_files(&self, filter: Option<FileFilter>) -> Result<Vec<FileMetadata>> {
        let cache = self.cache.read().await;
        let mut files: Vec<FileMetadata> = cache.files.values().cloned().collect();

        if let Some(filter) = filter {
            files = self.apply_filter(files, filter);
        }

        // Sort by path for consistent ordering
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    /// Search files by content, path, or tags
    pub async fn search_files(&self, query: &SearchQuery) -> Result<Vec<FileMetadata>> {
        if !self.config.enable_search_index {
            return Err(VaultError::ConfigurationError(
                "Search index is disabled".to_string()
            ));
        }

        let cache = self.cache.read().await;
        let mut matching_ids = HashSet::new();

        // Search in index
        for (file_id, index) in &cache.search_index {
            if self.matches_query(index, query) {
                matching_ids.insert(*file_id);
            }
        }

        // Get matching file metadata
        let mut results = Vec::new();
        for file_id in matching_ids {
            if let Some(metadata) = cache.files.get(&file_id) {
                results.push(metadata.clone());
            }
        }

        results.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(results)
    }

    /// Get version history for a file
    pub async fn get_versions(&self, file_id: &Uuid) -> Result<Vec<VersionInfo>> {
        let cache = self.cache.read().await;
        cache.versions.get(file_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| VaultError::NotFound(format!("No versions found for file: {}", file_id)))
    }

    /// Update search index for a file
    pub async fn update_search_index(&self, file_id: &Uuid, index: SearchIndex) -> Result<()> {
        if !self.config.enable_search_index {
            return Ok(());
        }

        let mut cache = self.cache.write().await;
        cache.search_index.insert(*file_id, index);
        cache.dirty = true;
        Ok(())
    }

    /// Get vault statistics
    pub async fn get_stats(&self) -> Result<VaultStats> {
        let cache = self.cache.read().await;
        Ok(cache.stats.clone())
    }

    /// Persist metadata to storage backend
    pub async fn persist(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        
        if !cache.dirty {
            return Ok(); // Nothing to persist
        }

        // Serialize and store each metadata component
        let files_data = self.serialize_data(&cache.files).await?;
        self.backend.put(Self::FILES_KEY, files_data).await?;

        if self.config.enable_versioning {
            let versions_data = self.serialize_data(&cache.versions).await?;
            self.backend.put(Self::VERSIONS_KEY, versions_data).await?;
        }

        if self.config.enable_search_index {
            let search_data = self.serialize_data(&cache.search_index).await?;
            self.backend.put(Self::SEARCH_INDEX_KEY, search_data).await?;
        }

        let stats_data = self.serialize_data(&cache.stats).await?;
        self.backend.put(Self::STATS_KEY, stats_data).await?;

        let config_data = self.serialize_data(&self.config).await?;
        self.backend.put(Self::CONFIG_KEY, config_data).await?;

        cache.dirty = false;
        Ok(())
    }

    /// Load metadata from storage backend
    async fn load_metadata(&self) -> Result<()> {
        let mut cache = self.cache.write().await;

        // Load files
        if self.backend.exists(Self::FILES_KEY).await? {
            let data = self.backend.get(Self::FILES_KEY).await?;
            cache.files = self.deserialize_data(&data).await?;
            
            // Rebuild path index
            for (id, metadata) in &cache.files {
                cache.path_to_id.insert(metadata.path.clone(), *id);
            }
        }

        // Load versions
        if self.config.enable_versioning && self.backend.exists(Self::VERSIONS_KEY).await? {
            let data = self.backend.get(Self::VERSIONS_KEY).await?;
            cache.versions = self.deserialize_data(&data).await?;
        }

        // Load search index
        if self.config.enable_search_index && self.backend.exists(Self::SEARCH_INDEX_KEY).await? {
            let data = self.backend.get(Self::SEARCH_INDEX_KEY).await?;
            cache.search_index = self.deserialize_data(&data).await?;
        }

        // Load stats
        if self.backend.exists(Self::STATS_KEY).await? {
            let data = self.backend.get(Self::STATS_KEY).await?;
            cache.stats = self.deserialize_data(&data).await?;
        } else {
            cache.stats.created_at = Utc::now();
        }

        cache.dirty = false;
        Ok(())
    }

    /// Add a new version to version history
    async fn add_version(&self, cache: &mut MetadataCache, file_id: &Uuid, metadata: &FileMetadata) -> Result<()> {
        let version_info = VersionInfo {
            version: metadata.version,
            created_at: metadata.created_at,
            size: metadata.size,
            checksum: metadata.checksum.clone(),
            comment: None,
        };

        let versions = cache.versions.entry(*file_id).or_insert_with(Vec::new);
        versions.push(version_info);

        // Cleanup old versions if limit is set
        if let Some(max_versions) = self.config.max_versions {
            if versions.len() > max_versions as usize {
                versions.drain(0..versions.len() - max_versions as usize);
            }
        }

        Ok(())
    }

    /// Update vault statistics
    async fn update_stats(&self, cache: &mut MetadataCache, metadata: &FileMetadata, add: bool) {
        let multiplier = if add { 1i64 } else { -1i64 };
        
        cache.stats.total_files = (cache.stats.total_files as i64 + multiplier) as u64;
        cache.stats.total_size = (cache.stats.total_size as i64 + (metadata.size as i64 * multiplier)) as u64;
        
        if let Some(mime_type) = &metadata.mime_type {
            let count = cache.stats.file_types.entry(mime_type.clone()).or_insert(0);
            *count = (*count as i64 + multiplier) as u64;
            if *count == 0 {
                cache.stats.file_types.remove(mime_type);
            }
        }

        cache.stats.last_updated = Utc::now();
    }

    /// Apply filters to file list
    fn apply_filter(&self, files: Vec<FileMetadata>, filter: FileFilter) -> Vec<FileMetadata> {
        files.into_iter().filter(|file| {
            // Path filter
            if let Some(path_pattern) = &filter.path_pattern {
                if !file.path.contains(path_pattern) {
                    return false;
                }
            }

            // Size filter
            if let Some(min_size) = filter.min_size {
                if file.size < min_size {
                    return false;
                }
            }
            if let Some(max_size) = filter.max_size {
                if file.size > max_size {
                    return false;
                }
            }

            // Date filter
            if let Some(after) = filter.created_after {
                if file.created_at < after {
                    return false;
                }
            }
            if let Some(before) = filter.created_before {
                if file.created_at > before {
                    return false;
                }
            }

            // MIME type filter
            if let Some(mime_types) = &filter.mime_types {
                if let Some(file_mime) = &file.mime_type {
                    if !mime_types.contains(file_mime) {
                        return false;
                    }
                } else if !mime_types.is_empty() {
                    return false;
                }
            }

            // Tags filter
            if let Some(required_tags) = &filter.tags {
                if !required_tags.iter().all(|tag| file.tags.contains(tag)) {
                    return false;
                }
            }

            true
        }).collect()
    }

    /// Check if search index matches query
    fn matches_query(&self, index: &SearchIndex, query: &SearchQuery) -> bool {
        // Content search
        if let Some(content_query) = &query.content {
            let content_matches = index.content_tokens.iter()
                .any(|token| token.to_lowercase().contains(&content_query.to_lowercase()));
            if !content_matches {
                return false;
            }
        }

        // Path search
        if let Some(path_query) = &query.path {
            let path_matches = index.path_tokens.iter()
                .any(|token| token.to_lowercase().contains(&path_query.to_lowercase()));
            if !path_matches {
                return false;
            }
        }

        // Tag search
        if let Some(tag_query) = &query.tags {
            if !tag_query.iter().all(|tag| index.tags.contains(tag)) {
                return false;
            }
        }

        // MIME type search
        if let Some(mime_query) = &query.mime_type {
            if let Some(index_mime) = &index.mime_type {
                if !index_mime.contains(mime_query) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Serialize data with optional compression
    async fn serialize_data<T: Serialize>(&self, data: &T) -> Result<Bytes> {
        let serialized = bincode::serialize(data)
            .map_err(|e| VaultError::SerializationError(format!("Failed to serialize: {}", e)))?;

        if self.config.compress_metadata {
            let compressed = self.compress_data(&serialized).await?;
            Ok(Bytes::from(compressed))
        } else {
            Ok(Bytes::from(serialized))
        }
    }

    /// Deserialize data with optional decompression
    async fn deserialize_data<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T> {
        let decompressed = if self.config.compress_metadata {
            self.decompress_data(data).await?
        } else {
            data.to_vec()
        };

        bincode::deserialize(&decompressed)
            .map_err(|e| VaultError::SerializationError(format!("Failed to deserialize: {}", e)))
    }

    /// Compress data using a simple compression algorithm
    async fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Using flate2 for compression
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)
            .map_err(|e| VaultError::CompressionError(format!("Compression failed: {}", e)))?;
        encoder.finish()
            .map_err(|e| VaultError::CompressionError(format!("Compression finalization failed: {}", e)))
    }

    /// Decompress data
    async fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| VaultError::CompressionError(format!("Decompression failed: {}", e)))?;
        Ok(decompressed)
    }
}

/// File filtering options
#[derive(Debug, Clone, Default)]
pub struct FileFilter {
    pub path_pattern: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub mime_types: Option<HashSet<String>>,
    pub tags: Option<HashSet<String>>,
}

/// Search query structure
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub content: Option<String>,
    pub path: Option<String>,
    pub tags: Option<HashSet<String>>,
    pub mime_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::backend::MemoryBackend;
    use std::collections::HashSet;

    fn create_test_metadata() -> FileMetadata {
        FileMetadata {
            id: Uuid::new_v4(),
            path: "test/file.txt".to_string(),
            original_name: "file.txt".to_string(),
            size: 1024,
            mime_type: Some("text/plain".to_string()),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            accessed_at: Utc::now(),
            version: 1,
            checksum: "abc123".to_string(),
            encryption: EncryptionMetadata {
                algorithm: "AES-256-GCM".to_string(),
                key_id: "key1".to_string(),
                nonce: vec![1, 2, 3, 4],
                is_compressed: false,
            },
            tags: HashSet::new(),
            custom_attributes: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_metadata_store_basic_operations() {
        let backend = Arc::new(MemoryBackend::new());
        let config = MetadataConfig::default();
        let store = MetadataStore::new(backend, config).await.unwrap();

        let metadata = create_test_metadata();
        let file_id = metadata.id;

        // Test add file
        store.add_file(metadata.clone()).await.unwrap();

        // Test get file by ID
        let retrieved = store.get_file(&file_id).await.unwrap();
        assert_eq!(retrieved.id, metadata.id);
        assert_eq!(retrieved.path, metadata.path);

        // Test get file by path
        let retrieved_by_path = store.get_file_by_path(&metadata.path).await.unwrap();
        assert_eq!(retrieved_by_path.id, metadata.id);

        // Test update file
        let mut updated_metadata = metadata.clone();
        updated_metadata.size = 2048;
        updated_metadata.version = 2;
        store.update_file(&file_id, updated_metadata).await.unwrap();

        let retrieved_updated = store.get_file(&file_id).await.unwrap();
        assert_eq!(retrieved_updated.size, 2048);
        assert_eq!(retrieved_updated.version, 2);

        // Test delete file
        let deleted = store.delete_file(&file_id).await.unwrap();
        assert_eq!(deleted.id, file_id);

        // Verify file is deleted
        assert!(store.get_file(&file_id).await.is_err());
    }

    #[tokio::test]
    async fn test_metadata_store_persistence() {
        let backend = Arc::new(MemoryBackend::new());
        let config = MetadataConfig::default();
        
        {
            let store = MetadataStore::new(backend.clone(), config.clone()).await.unwrap();
            let metadata = create_test_metadata();
            store.add_file(metadata).await.unwrap();
            store.persist().await.unwrap();
        }

        // Create new store with same backend - should load existing data
        let store2 = MetadataStore::new(backend, config).await.unwrap();
        let files = store2.list_files(None).await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test/file.txt");
    }

    #[tokio::test]
    async fn test_file_filtering() {
        let backend = Arc::new(MemoryBackend::new());
        let config = MetadataConfig::default();
        let store = MetadataStore::new(backend, config).await.unwrap();

        // Add test files
        let mut metadata1 = create_test_metadata();
        metadata1.path = "documents/file1.txt".to_string();
        metadata1.size = 1000;
        metadata1.mime_type = Some("text/plain".to_string());

        let mut metadata2 = create_test_metadata();
        metadata2.id = Uuid::new_v4();
        metadata2.path = "images/photo.jpg".to_string();
        metadata2.size = 5000;
        metadata2.mime_type = Some("image/jpeg".to_string());

        store.add_file(metadata1).await.unwrap();
        store.add_file(metadata2).await.unwrap();

        // Test path filtering
        let filter = FileFilter {
            path_pattern: Some("documents".to_string()),
            ..Default::default()
        };
        let filtered = store.list_files(Some(filter)).await.unwrap();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].path.contains("documents"));

        // Test size filtering
        let filter = FileFilter {
            min_size: Some(2000),
            ..Default::default()
        };
        let filtered = store.list_files(Some(filter)).await.unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].size, 5000);
    }
}
