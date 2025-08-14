use serde::{Deserialize, Serialize};

/// The full declarative configuration file format.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub packages: Vec<Package>,
    /// Optional files to include in this config.
    #[serde(default)]
    pub includes: Vec<String>,
}

/// Representation of a single package in config.
#[derive(Debug, Deserialize, Clone)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
    /// Optional git source repo (for `latest` resolution).
    pub source: Option<String>,
    /// Optional prebuilt URL (may include `{tag}` and `{arch}` placeholders).
    pub prebuilt: Option<String>,
}

/// On-disk state DB format.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StateDb {
    pub generation: u64,
    pub generations: Vec<Generation>,
}

/// A specific generation snapshot of installed packages.
#[derive(Debug, Serialize, Deserialize)]
pub struct Generation {
    pub id: u64,
    pub timestamp: String,
    pub packages: Vec<InstalledPackage>,
}

/// A package that has been installed in a generation.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub store_path: String,
    pub installed_at: String,
    pub files: Vec<String>,
}
