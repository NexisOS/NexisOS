use std::path::PathBuf;

pub struct StoreLayout {
    root: PathBuf,
}

impl StoreLayout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    
    /// Get path for object: /nexis-store/ab/cd/abcd1234-name/
    pub fn object_path(&self, hash: &str, name: &str) -> PathBuf {
        let prefix1 = &hash[0..2];
        let prefix2 = &hash[2..4];
        self.root
            .join("packages")
            .join(prefix1)
            .join(prefix2)
            .join(format!("{}-{}", hash, name))
    }
    
    /// Get path for file: /nexis-store/files/ab/cd/abcd1234.txt
    pub fn file_path(&self, hash: &str) -> PathBuf {
        let prefix1 = &hash[0..2];
        let prefix2 = &hash[2..4];
        self.root
            .join("files")
            .join(prefix1)
            .join(prefix2)
            .join(hash)
    }
}
