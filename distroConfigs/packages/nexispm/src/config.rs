//! # Configuration Management
//!
//! Handles parsing and validation of NexisOS system configuration files.
//! Supports declarative package definitions with "latest" version resolution,
//! system settings, user management, and service definitions.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

/// Main configuration error types
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("Invalid TOML syntax: {0}")]
    TomlParse(#[from] toml::de::Error),
    
    #[error("I/O error reading config: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid configuration: {msg}")]
    Validation { msg: String },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid include path: {path}")]
    InvalidInclude { path: PathBuf },
}

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexisConfig {
    pub system: SystemConfig,
    pub users: HashMap<String, UserConfig>,
    pub network: NetworkConfig,
    pub packages: Vec<PackageConfig>,
    pub config_files: HashMap<String, ConfigFileTemplate>,
    pub dinit_services: HashMap<String, DinitService>,
    pub log_rotation: Vec<LogRotationConfig>,
    pub includes: Option<IncludesConfig>,
    
    /// Internal field to track config file path for relative includes
    #[serde(skip)]
    pub config_dir: PathBuf,
}

/// System-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub timezone: String,
    pub version: String,
    pub kernel: String,
    pub kernel_source: String,
    pub kernel_config: PathBuf,
    
    /// Storage backend: "ext4" or "xfs"
    #[serde(default = "default_storage_backend")]
    pub storage_backend: String,
    
    /// Where packages are stored (content-addressable store)
    #[serde(default = "default_store_path")]
    pub store_path: PathBuf,
    
    /// GRUB configuration path for generation menu entries
    #[serde(default = "default_grub_config")]
    pub grub_config_path: PathBuf,
    
    /// SELinux configuration
    pub selinux: Option<SelinuxConfig>,
    
    /// Firewall configuration
    pub firewall: Option<FirewallConfig>,
    
    /// Locale settings
    pub locale: Option<LocaleConfig>,
}

/// SELinux configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelinuxConfig {
    pub enabled: bool,
    /// "enforcing", "permissive", or "disabled"
    pub mode: String,
}

/// Firewall configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallConfig {
    /// "nftables", "iptables", or "firewalld"
    pub backend: String,
}

/// System locale configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfig {
    pub lang: String,
    pub keyboard_layout: String,
}

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub password_hash: String,
    pub authorized_keys: Vec<String>,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub interface: String,
    pub dhcp: bool,
    pub static_ip: Option<String>,
    pub gateway: Option<String>,
    pub dns: Option<Vec<String>>,
}

/// Package configuration with support for "latest" version resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    
    /// Version specification - can be "latest", specific version, or git ref
    pub version: VersionSpec,
    
    /// Optional prebuilt binary URL with template variables
    pub prebuilt: Option<String>,
    
    /// Whether to fall back to source build if prebuilt fails
    #[serde(default)]
    pub fallback_to_source: bool,
    
    /// Source repository URL (git, tarball, etc.)
    pub source: Option<String>,
    
    /// Expected hash for verification (optional)
    pub hash: Option<String>,
    
    /// Build system type
    pub build_system: Option<BuildSystem>,
    
    /// Build flags to pass to the build system
    #[serde(default)]
    pub build_flags: Vec<String>,
    
    /// Dependencies that must be built first
    #[serde(default)]
    pub dependencies: Vec<String>,
    
    /// Patches to apply before building
    #[serde(default)]
    pub patches: Vec<PathBuf>,
    
    /// Pre-build script to run
    pub pre_build_script: Option<PathBuf>,
    
    /// Post-build script to run
    pub post_build_script: Option<PathBuf>,
    
    /// SELinux context file for this package
    pub context_file: Option<PathBuf>,
    
    /// Environment variables for the build
    #[serde(default)]
    pub env: HashMap<String, String>,
    
    /// Runtime directories to create
    #[serde(default)]
    pub runtime_dirs: Vec<String>,
}

/// Version specification that supports "latest" git tag resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VersionSpec {
    /// Literal version string (e.g., "1.2.3")
    Exact(String),
    
    /// Special "latest" keyword for git tag resolution
    Latest,
    
    /// Git-specific version specification
    Git {
        /// Git reference (branch, tag, commit)
        #[serde(rename = "ref")]
        git_ref: String,
    },
}

impl VersionSpec {
    pub fn is_latest(&self) -> bool {
        matches!(self, VersionSpec::Latest) || 
        (matches!(self, VersionSpec::Exact(v)) if v == "latest")
    }
    
    pub fn as_string(&self) -> String {
        match self {
            VersionSpec::Exact(v) => v.clone(),
            VersionSpec::Latest => "latest".to_string(),
            VersionSpec::Git { git_ref } => git_ref.clone(),
        }
    }
}

/// Supported build systems
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildSystem {
    Make,
    Configure,  // autotools
    Cmake,
    Meson,
    Cargo,      // Rust
    Npm,        // Node.js
    Python,     // setup.py/pip
    Custom,     // Uses custom build script
}

/// Configuration file template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileTemplate {
    pub path: PathBuf,
    pub source: PathBuf,
    pub owner: String,
    pub group: String,
    pub mode: String,  // Octal mode as string (e.g., "0644")
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
}

/// Dinit service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DinitService {
    pub name: String,
    /// "scripted" or "process"
    #[serde(rename = "type")]
    pub service_type: String,
    pub command: String,
    #[serde(default)]
    pub depends: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    #[serde(default)]
    pub restart: bool,
    pub start_timeout: Option<u32>,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    pub path: String,
    pub max_size_mb: u64,
    pub max_files: u32,
    #[serde(default)]
    pub compress: bool,
    pub rotate_interval_days: Option<u32>,
}

/// Include configuration for modular config files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludesConfig {
    pub paths: Vec<PathBuf>,
}

impl NexisConfig {
    /// Load configuration from a file with include support
    pub fn load<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config_path = config_path.as_ref();
        let config_dir = config_path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
        
        let mut config: NexisConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config: {:?}", config_path))?;
        
        config.config_dir = config_dir;
        
        // Process includes
        if let Some(includes) = &config.includes {
            config = config.process_includes(includes)?;
        }
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Process include files and merge configurations
    fn process_includes(mut self, includes: &IncludesConfig) -> Result<Self> {
        for include_path in &includes.paths {
            let full_path = if include_path.is_absolute() {
                include_path.clone()
            } else {
                self.config_dir.join(include_path)
            };
            
            if !full_path.exists() {
                return Err(ConfigError::InvalidInclude { 
                    path: full_path 
                }.into());
            }
            
            let include_content = std::fs::read_to_string(&full_path)
                .with_context(|| format!("Failed to read include file: {:?}", full_path))?;
            
            let include_config: PartialConfig = toml::from_str(&include_content)
                .with_context(|| format!("Failed to parse include file: {:?}", full_path))?;
            
            // Merge the included configuration
            self = self.merge(include_config)?;
        }
        
        Ok(self)
    }
    
    /// Merge a partial configuration into this one
    fn merge(mut self, partial: PartialConfig) -> Result<Self> {
        // Merge packages
        if let Some(packages) = partial.packages {
            self.packages.extend(packages);
        }
        
        // Merge config files
        if let Some(config_files) = partial.config_files {
            self.config_files.extend(config_files);
        }
        
        // Merge dinit services
        if let Some(dinit_services) = partial.dinit_services {
            self.dinit_services.extend(dinit_services);
        }
        
        // Merge log rotation configs
        if let Some(log_rotation) = partial.log_rotation {
            self.log_rotation.extend(log_rotation);
        }
        
        Ok(self)
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate storage backend
        match self.system.storage_backend.as_str() {
            "ext4" | "xfs" => {},
            backend => {
                return Err(ConfigError::Validation {
                    msg: format!("Invalid storage backend: {}. Must be 'ext4' or 'xfs'", backend)
                }.into());
            }
        }
        
        // Validate SELinux mode if enabled
        if let Some(selinux) = &self.system.selinux {
            if selinux.enabled {
                match selinux.mode.as_str() {
                    "enforcing" | "permissive" | "disabled" => {},
                    mode => {
                        return Err(ConfigError::Validation {
                            msg: format!("Invalid SELinux mode: {}. Must be 'enforcing', 'permissive', or 'disabled'", mode)
                        }.into());
                    }
                }
            }
        }
        
        // Validate firewall backend
        if let Some(firewall) = &self.system.firewall {
            match firewall.backend.as_str() {
                "nftables" | "iptables" | "firewalld" => {},
                backend => {
                    return Err(ConfigError::Validation {
                        msg: format!("Invalid firewall backend: {}. Must be 'nftables', 'iptables', or 'firewalld'", backend)
                    }.into());
                }
            }
        }
        
        // Validate package configurations
        for package in &self.packages {
            if package.name.is_empty() {
                return Err(ConfigError::Validation {
                    msg: "Package name cannot be empty".to_string()
                }.into());
            }
            
            // If fallback_to_source is true, source must be provided
            if package.fallback_to_source && package.source.is_none() {
                return Err(ConfigError::Validation {
                    msg: format!("Package '{}' has fallback_to_source enabled but no source specified", package.name)
                }.into());
            }
        }
        
        // Validate dinit services
        for (name, service) in &self.dinit_services {
            if service.name != *name {
                return Err(ConfigError::Validation {
                    msg: format!("Service name mismatch: key '{}' vs service.name '{}'", name, service.name)
                }.into());
            }
            
            match service.service_type.as_str() {
                "scripted" | "process" => {},
                stype => {
                    return Err(ConfigError::Validation {
                        msg: format!("Invalid service type '{}' for service '{}'. Must be 'scripted' or 'process'", stype, name)
                    }.into());
                }
            }
        }
        
        Ok(())
    }
    
    /// Get packages that need "latest" version resolution
    pub fn packages_needing_resolution(&self) -> Vec<&PackageConfig> {
        self.packages
            .iter()
            .filter(|pkg| pkg.version.is_latest())
            .collect()
    }
    
    /// Find a package configuration by name
    pub fn find_package(&self, name: &str) -> Option<&PackageConfig> {
        self.packages.iter().find(|pkg| pkg.name == name)
    }
}

/// Partial configuration structure for includes
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PartialConfig {
    pub packages: Option<Vec<PackageConfig>>,
    pub config_files: Option<HashMap<String, ConfigFileTemplate>>,
    pub dinit_services: Option<HashMap<String, DinitService>>,
    pub log_rotation: Option<Vec<LogRotationConfig>>,
}

// Default value functions
fn default_storage_backend() -> String {
    "ext4".to_string()
}

fn default_store_path() -> PathBuf {
    PathBuf::from("/store")
}

fn default_grub_config() -> PathBuf {
    PathBuf::from("/boot/grub/grub.cfg")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_spec_latest() {
        let latest_str = VersionSpec::Exact("latest".to_string());
        let latest_enum = VersionSpec::Latest;
        let specific = VersionSpec::Exact("1.2.3".to_string());
        
        assert!(latest_str.is_latest());
        assert!(latest_enum.is_latest());
        assert!(!specific.is_latest());
    }
    
    #[test]
    fn test_basic_config_parsing() {
        let toml_content = r#"
[system]
hostname = "test-host"
timezone = "UTC"
version = "0.1.0"
kernel = "linux-6.9.2"
kernel_source = "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.9.2.tar.xz"
kernel_config = "configs/kernel-default.config"

[users.root]
password_hash = "$argon2id$v=19$m=65536,t=3,p=4$SOME_BASE64_SALT$SOME_BASE64_HASH"
authorized_keys = []

[network]
interface = "eth0"
dhcp = true

[[packages]]
name = "vim"
version = "latest"
source = "https://github.com/vim/vim.git"
build_system = "make"
        "#;
        
        let config: NexisConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.system.hostname, "test-host");
        assert_eq!(config.packages.len(), 1);
        assert!(config.packages[0].version.is_latest());
    }
}
