use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub system: SystemConfig,
    pub admin: AdminConfig,
    pub packages: Vec<Package>,
    pub files: Vec<FileDeclaration>,
    pub users: Vec<User>,
    pub includes: Option<Includes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub timezone: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub prebuilt: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileDeclaration {
    pub path: String,
    pub content: Option<String>,
    pub source: Option<String>,
    pub mode: String,
    pub owner: String,
    pub group: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub shell: String,
    pub groups: Vec<String>,
    pub profiles: Option<Vec<String>>,
    pub files: Vec<FileDeclaration>,
}
