//! Configuration structures for file management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use camino::Utf8PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct NexisConfig {
    pub nexis: NexisSection,
    pub system: SystemConfig,
    pub users: HashMap<String, UserConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NexisSection {
    #[serde(default)]
    pub files: HashMap<String, FileSpec>,
    
    #[serde(default)]
    pub directories: HashMap<String, DirectorySpec>,
    
    #[serde(default)]
    pub environment: HashMap<String, String>,
    
    #[serde(default)]
    pub dinit: HashMap<String, DinitService>,
    
    #[serde(default)]
    pub system_files: HashMap<String, FileSpec>,
    
    #[serde(default)]
    pub packages: HashMap<String, PackageConfig>,
    
    #[serde(default)]
    pub hooks: HooksConfig,
    
    #[serde(default)]
    pub xdg: XdgConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum FileSpec {
    Simple {
        source: String,
    },
    Advanced {
        source: Option<String>,
        content: Option<String>,
        template: Option<String>,
        variables: Option<HashMap<String, serde_json::Value>>,
        mode: Option<String>,
        owner: Option<String>,
        group: Option<String>,
        symlink: Option<String>,
        directory: Option<bool>,
        condition: Option<Condition>,
        backup: Option<bool>,
        force: Option<bool>,
        selinux_context: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DirectorySpec {
    pub mode: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub recursive: Option<bool>,
    pub selinux_context: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Condition {
    pub package: Option<String>,
    pub file_exists: Option<String>,
    pub env_var: Option<String>,
    pub user: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DinitService {
    pub r#type: String, // "process", "scripted", etc.
    pub command: String,
    pub depends: Option<Vec<String>>,
    pub user: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Option<HashMap<String, String>>,
    pub log_file: Option<String>,
    pub restart: Option<bool>,
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageConfig {
    pub files: HashMap<String, FileSpec>,
    pub directories: Option<HashMap<String, DirectorySpec>>,
    pub services: Option<HashMap<String, DinitService>>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub on_file_change: Vec<Hook>,
    
    #[serde(default)]
    pub before_generation: Vec<Hook>,
    
    #[serde(default)]
    pub after_generation: Vec<Hook>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Hook {
    pub pattern: String,
    pub command: String,
    pub user: Option<String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub timezone: String,
    pub kernel: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserConfig {
    pub shell: Option<String>,
    pub home: Option<Utf8PathBuf>,
    pub groups: Option<Vec<String>>,
    pub password_hash: Option<String>,
    pub authorized_keys: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct XdgConfig {
    pub config_home: Option<String>,
    pub data_home: Option<String>,
    pub cache_home: Option<String>,
    pub runtime_dir: Option<String>,
}
