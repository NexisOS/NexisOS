use blake3::Hasher;
use std::fs;
use anyhow::Result;

pub fn hash_content(content: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(content.as_bytes());
    hasher.finalize().to_hex().to_string()
}

pub fn hash_file(path: &Path) -> Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Hasher::new();
    hasher.update(&content);
    Ok(hasher.finalize().to_hex().to_string())
}
