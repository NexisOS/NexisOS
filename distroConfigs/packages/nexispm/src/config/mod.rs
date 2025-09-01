use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// Main configuration structure for NexisOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub system: SystemConfig,
    pub network: Option<NetworkConfig>,
    pub packages: Vec<Package>,
    pub dinit_services: Option<HashMap<String, DinitService>>,
    pub config_files: Option<HashMap<String, ConfigFile>>,
    pub log_rotation: Option<Vec<LogRotation>>,
    pub includes: Option<IncludeConfig>,
}

/// System-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub version: String,
    pub timezone: Option<String>,
    pub kernel: Option<String>,
    pub kernel_source: Option<String>,
    pub kernel_config: Option<String>,
    pub locale: Option<LocaleConfig>,
    pub selinux: Option<SeLinuxConfig>,
    pub firewall: Option<FirewallConfig>,
}

/// Locale configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfig {
    pub lang: String,
    pub keyboard_layout: Option<String>,
}

/// SELinux configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeLinuxConfig {
    pub enabled: bool,
    pub mode: String, // "enforcing", "permissive", "disabled"
}

/// Firewall configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallConfig {
    pub backend: String, // "nftables", "iptables", "firewalld"
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub interface: String,
    pub dhcp: Option<bool>,
    pub static_ip: Option<String>,
    pub gateway: Option<String>,
    pub dns: Option<Vec<String>>,
}

/// Package definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub prebuilt: Option<String>,
    pub fallback_to_source: Option<bool>,
    pub hash: Option<String>,
    pub patches: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    pub build_system: Option<String>,
    pub build_flags: Option<Vec<String>>,
    pub pre_build_script: Option<String>,
    pub post_build_script: Option<String>,
    pub context_file: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub runtime_dirs: Option<Vec<String>>,
}

/// Dinit service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DinitService {
    pub name: String,
    #[serde(rename = "type")]
    pub service_type: String, // "process", "scripted", "internal"
    pub command: String,
    pub depends: Option<Vec<String>>,
    pub working_directory: Option<String>,
    pub log_file: Option<String>,
    pub restart: Option<String>,
    pub start_timeout: Option<u32>,
}

/// Configuration file template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub path: String,
    pub source: String,
    pub owner: String,
    pub group: String,
    pub mode: String,
    pub variables: Option<HashMap<String, serde_json::Value>>,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotation {
    pub path: String,
    pub max_size_mb: u64,
    pub max_files: u32,
    pub compress: bool,
    pub rotate_interval_days: Option<u32>,
}

/// Include configuration for modular configs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeConfig {
    pub paths: Vec<String>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        Self::from_str(&content)
    }

    /// Parse configuration from a TOML string
    pub fn from_str(content: &str) -> Result<Self> {
        let config: Config = toml::from_str(content)
            .context("Failed to parse TOML configuration")?;
        
        config.validate()?;
        Ok(config)
    }

    /// Load configuration with includes resolved
    pub fn from_file_with_includes<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = Self::from_file(&path)?;
        
        if let Some(includes) = &config.includes {
            let base_dir = path.as_ref().parent()
                .unwrap_or_else(|| Path::new("."));
            
            for include_path in &includes.paths {
                let full_path = base_dir.join(include_path);
                let include_config = Self::from_file(&full_path)
                    .with_context(|| format!("Failed to load included config: {}", full_path.display()))?;
                
                config.merge(include_config)?;
            }
        }
        
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate hostname
        if self.system.hostname.is_empty() {
            anyhow::bail!("System hostname cannot be empty");
        }

        // Validate version
        if self.system.version.is_empty() {
            anyhow::bail!("System version cannot be empty");
        }

        // Validate packages
        for package in &self.packages {
            if package.name.is_empty() {
                anyhow::bail!("Package name cannot be empty");
            }
            
            if package.version.is_empty() {
                anyhow::bail!("Package version cannot be empty for package '{}'", package.name);
            }

            // Must have either source or prebuilt
            if package.source.is_none() && package.prebuilt.is_none() {
                anyhow::bail!("Package '{}' must have either 'source' or 'prebuilt' specified", package.name);
            }

            // Validate build system if building from source
            if package.source.is_some() && package.build_system.is_none() {
                anyhow::bail!("Package '{}' building from source must specify 'build_system'", package.name);
            }
        }

        // Validate SELinux mode if enabled
        if let Some(selinux) = &self.system.selinux {
            if selinux.enabled && !["enforcing", "permissive", "disabled"].contains(&selinux.mode.as_str()) {
                anyhow::bail!("Invalid SELinux mode '{}'. Must be one of: enforcing, permissive, disabled", selinux.mode);
            }
        }

        // Validate firewall backend
        if let Some(firewall) = &self.system.firewall {
            if !["nftables", "iptables", "firewalld"].contains(&firewall.backend.as_str()) {
                anyhow::bail!("Invalid firewall backend '{}'. Must be one of: nftables, iptables, firewalld", firewall.backend);
            }
        }

        Ok(())
    }

    /// Merge another configuration into this one
    pub fn merge(&mut self, other: Config) -> Result<()> {
        // Extend packages
        self.packages.extend(other.packages);

        // Merge services
        if let Some(other_services) = other.dinit_services {
            match &mut self.dinit_services {
                Some(services) => services.extend(other_services),
                None => self.dinit_services = Some(other_services),
            }
        }

        // Merge config files
        if let Some(other_configs) = other.config_files {
            match &mut self.config_files {
                Some(configs) => configs.extend(other_configs),
                None => self.config_files = Some(other_configs),
            }
        }

        // Extend log rotation
        if let Some(other_logs) = other.log_rotation {
            match &mut self.log_rotation {
                Some(logs) => logs.extend(other_logs),
                None => self.log_rotation = Some(other_logs),
            }
        }

        Ok(())
    }

    /// Get all package names
    pub fn package_names(&self) -> Vec<&str> {
        self.packages.iter().map(|p| p.name.as_str()).collect()
    }

    /// Find a package by name
    pub fn find_package(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Get packages that need to be built from source
    pub fn source_packages(&self) -> Vec<&Package> {
        self.packages.iter()
            .filter(|p| p.source.is_some() && (p.prebuilt.is_none() || p.fallback_to_source.unwrap_or(false)))
            .collect()
    }

    /// Get packages that can use prebuilt binaries
    pub fn prebuilt_packages(&self) -> Vec<&Package> {
        self.packages.iter()
            .filter(|p| p.prebuilt.is_some() && !p.fallback_to_source.unwrap_or(false))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config_parsing() {
        let toml_content = r#"
[system]
hostname = "test-host"
version = "0.1.0"

[system.locale]
lang = "en_US.UTF-8"

[[packages]]
name = "busybox"
version = "1.36.1"
source = "https://busybox.net/downloads/busybox-1.36.1.tar.bz2"
build_system = "make"
        "#;

        let config = Config::from_str(toml_content).expect("Failed to parse config");
        assert_eq!(config.system.hostname, "test-host");
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].name, "busybox");
    }

    #[test]
    fn test_config_validation() {
        let toml_content = r#"
[system]
hostname = ""
version = "0.1.0"

[[packages]]
name = "test"
version = "1.0.0"
        "#;

        let result = Config::from_str(toml_content);
        assert!(result.is_err());
    }
}
