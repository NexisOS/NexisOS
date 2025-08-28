use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::util::NexisError;

/// Main configuration structure matching the TOML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexisConfig {
    pub system: SystemConfig,
    #[serde(default)]
    pub users: HashMap<String, UserConfig>,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub includes: IncludesConfig,
    #[serde(default)]
    pub packages: Vec<PackageConfig>,
    #[serde(default)]
    pub config_files: HashMap<String, ConfigFile>,
    #[serde(default)]
    pub dinit_services: HashMap<String, DinitService>,
    #[serde(default)]
    pub log_rotation: Vec<LogRotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub timezone: String,
    pub version: String,
    pub kernel: String,
    pub kernel_source: String,
    pub kernel_config: String,
    #[serde(default)]
    pub selinux: SelinuxConfig,
    #[serde(default)]
    pub firewall: FirewallConfig,
    #[serde(default)]
    pub locale: LocaleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelinuxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_selinux_mode")]
    pub mode: String,
}

fn default_selinux_mode() -> String {
    "enforcing".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FirewallConfig {
    #[serde(default = "default_firewall_backend")]
    pub backend: String,
}

fn default_firewall_backend() -> String {
    "nftables".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocaleConfig {
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default = "default_keyboard")]
    pub keyboard_layout: String,
}

fn default_lang() -> String {
    "en_US.UTF-8".to_string()
}

fn default_keyboard() -> String {
    "us".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub password_hash: String,
    #[serde(default)]
    pub authorized_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    #[serde(default = "default_interface")]
    pub interface: String,
    #[serde(default = "default_dhcp")]
    pub dhcp: bool,
    pub static_ip: Option<String>,
    pub gateway: Option<String>,
    #[serde(default)]
    pub dns: Vec<String>,
}

fn default_interface() -> String {
    "eth0".to_string()
}

fn default_dhcp() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncludesConfig {
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    pub prebuilt: Option<String>,
    #[serde(default)]
    pub fallback_to_source: bool,
    pub source: Option<String>,
    #[serde(default)]
    pub patches: Vec<String>,
    pub pre_build_script: Option<String>,
    pub post_build_script: Option<String>,
    pub build_system: Option<String>,
    #[serde(default)]
    pub build_flags: Vec<String>,
    pub context_file: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub runtime_dirs: Vec<String>,
    pub hash: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub path: String,
    pub source: String,
    pub owner: String,
    pub group: String,
    pub mode: String,
    #[serde(default)]
    pub variables: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DinitService {
    pub name: String,
    #[serde(rename = "type")]
    pub service_type: String,
    pub command: String,
    #[serde(default)]
    pub depends: Vec<String>,
    pub start_timeout: Option<u32>,
    pub working_directory: Option<String>,
    pub log_file: Option<String>,
    pub restart: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotation {
    pub path: String,
    pub max_size_mb: u64,
    pub max_files: u32,
    #[serde(default)]
    pub compress: bool,
    pub rotate_interval_days: Option<u32>,
}

impl NexisConfig {
    /// Load configuration from the default path
    pub fn load() -> Result<Self, NexisError> {
        Self::load_from("/etc/nexis/config.toml")
    }

    /// Load configuration from a specific path
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self, NexisError> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| NexisError::Io {
                path: path.as_ref().to_path_buf(),
                source: e,
            })?;

        let mut config: NexisConfig = toml::from_str(&content)
            .map_err(|e| NexisError::Config(format!("Failed to parse TOML: {}", e)))?;

        // Process includes
        config.process_includes(path.as_ref().parent())?;

        Ok(config)
    }

    /// Process included configuration files
    fn process_includes(&mut self, base_dir: Option<&Path>) -> Result<(), NexisError> {
        let base_dir = base_dir.unwrap_or_else(|| Path::new("/etc/nexis"));

        for include_path in &self.includes.paths {
            let full_path = if Path::new(include_path).is_absolute() {
                PathBuf::from(include_path)
            } else {
                base_dir.join(include_path)
            };

            if !full_path.exists() {
                eprintln!("Warning: Include file not found: {}", full_path.display());
                continue;
            }

            let included = Self::load_from(&full_path)?;
            self.merge_config(included)?;
        }

        Ok(())
    }

    /// Merge another configuration into this one
    fn merge_config(&mut self, other: NexisConfig) -> Result<(), NexisError> {
        // Merge packages
        self.packages.extend(other.packages);

        // Merge config files
        for (name, config_file) in other.config_files {
            if self.config_files.contains_key(&name) {
                eprintln!("Warning: Overriding config file definition: {}", name);
            }
            self.config_files.insert(name, config_file);
        }

        // Merge dinit services
        for (name, service) in other.dinit_services {
            if self.dinit_services.contains_key(&name) {
                eprintln!("Warning: Overriding dinit service definition: {}", name);
            }
            self.dinit_services.insert(name, service);
        }

        // Merge log rotation configs
        self.log_rotation.extend(other.log_rotation);

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), NexisError> {
        // Validate hostname
        if self.system.hostname.is_empty() {
            return Err(NexisError::Config("Hostname cannot be empty".to_string()));
        }

        // Validate kernel version format
        if !self.system.kernel.starts_with("linux-") {
            return Err(NexisError::Config(
                "Kernel version must start with 'linux-'".to_string()
            ));
        }

        // Validate SELinux mode
        match self.system.selinux.mode.as_str() {
            "enforcing" | "permissive" | "disabled" => {},
            _ => return Err(NexisError::Config(
                "SELinux mode must be 'enforcing', 'permissive', or 'disabled'".to_string()
            )),
        }

        // Validate firewall backend
        match self.system.firewall.backend.as_str() {
            "nftables" | "iptables" | "firewalld" => {},
            _ => return Err(NexisError::Config(
                "Firewall backend must be 'nftables', 'iptables', or 'firewalld'".to_string()
            )),
        }

        // Validate package configurations
        for pkg in &self.packages {
            if pkg.name.is_empty() {
                return Err(NexisError::Config("Package name cannot be empty".to_string()));
            }

            if pkg.prebuilt.is_none() && pkg.source.is_none() {
                return Err(NexisError::Config(
                    format!("Package '{}' must have either prebuilt or source URL", pkg.name)
                ));
            }
        }

        // Validate dinit services
        for (name, service) in &self.dinit_services {
            match service.service_type.as_str() {
                "process" | "scripted" | "internal" => {},
                _ => return Err(NexisError::Config(
                    format!("Invalid dinit service type '{}' for service '{}'", 
                           service.service_type, name)
                )),
            }
        }

        // Validate config file permissions
        for (name, config_file) in &self.config_files {
            if !config_file.mode.starts_with('0') || config_file.mode.len() != 4 {
                return Err(NexisError::Config(
                    format!("Invalid file mode '{}' for config file '{}'", 
                           config_file.mode, name)
                ));
            }
        }

        Ok(())
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Option<&PackageConfig> {
        self.packages.iter().find(|pkg| pkg.name == name)
    }

    /// Get all packages with dependencies resolved in topological order
    pub fn get_packages_ordered(&self) -> Result<Vec<&PackageConfig>, NexisError> {
        use std::collections::{HashSet, VecDeque};

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit<'a>(
            pkg_name: &str,
            packages: &'a [PackageConfig],
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
            result: &mut Vec<&'a PackageConfig>,
        ) -> Result<(), NexisError> {
            if visited.contains(pkg_name) {
                return Ok(());
            }

            if visiting.contains(pkg_name) {
                return Err(NexisError::Config(
                    format!("Circular dependency detected involving package '{}'", pkg_name)
                ));
            }

            let pkg = packages.iter().find(|p| p.name == pkg_name)
                .ok_or_else(|| NexisError::Config(
                    format!("Package '{}' not found", pkg_name)
                ))?;

            visiting.insert(pkg_name.to_string());

            for dep in &pkg.dependencies {
                visit(dep, packages, visited, visiting, result)?;
            }

            visiting.remove(pkg_name);
            visited.insert(pkg_name.to_string());
            result.push(pkg);

            Ok(())
        }

        for pkg in &self.packages {
            visit(&pkg.name, &self.packages, &mut visited, &mut visiting, &mut result)?;
        }

        Ok(result)
    }

    /// Expand environment variables in a string
    pub fn expand_vars(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        // Replace system variables
        result = result.replace("{hostname}", &self.system.hostname);
        result = result.replace("{version}", &self.system.version);
        result = result.replace("{kernel}", &self.system.kernel);
        
        // Replace XDG variables (basic implementation)
        if let Ok(user) = std::env::var("USER") {
            result = result.replace("$XDG_RUNTIME_DIR", &format!("/run/user/{}", 
                get_user_id(&user).unwrap_or(1000)));
        }
        
        result
    }
}

impl Default for NexisConfig {
    fn default() -> Self {
        Self {
            system: SystemConfig {
                hostname: "nexis".to_string(),
                timezone: "UTC".to_string(),
                version: "0.1.0".to_string(),
                kernel: "linux-6.9.2".to_string(),
                kernel_source: "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.9.2.tar.xz".to_string(),
                kernel_config: "configs/kernel-default.config".to_string(),
                selinux: SelinuxConfig::default(),
                firewall: FirewallConfig::default(),
                locale: LocaleConfig::default(),
            },
            users: HashMap::new(),
            network: NetworkConfig::default(),
            includes: IncludesConfig::default(),
            packages: Vec::new(),
            config_files: HashMap::new(),
            dinit_services: HashMap::new(),
            log_rotation: Vec::new(),
        }
    }
}

/// Helper function to get user ID (simplified)
fn get_user_id(username: &str) -> Option<u32> {
    // In a real implementation, this would query /etc/passwd or use libc
    // For now, return a default UID for non-root users
    match username {
        "root" => Some(0),
        _ => Some(1000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_config_parsing() {
        let toml_content = r#"
[system]
hostname = "test-host"
timezone = "America/New_York"
version = "0.2.0"
kernel = "linux-6.10.1"
kernel_source = "https://example.com/kernel.tar.xz"
kernel_config = "test-config"

[system.selinux]
enabled = true
mode = "enforcing"

[users.root]
password_hash = "$argon2id$test"
authorized_keys = ["ssh-ed25519 AAAA... test@example.com"]

[[packages]]
name = "vim"
version = "latest"
source = "https://github.com/vim/vim.git"
dependencies = ["ncurses"]

[[packages]]
name = "ncurses"
version = "6.4"
prebuilt = "https://example.com/ncurses.tar.gz"
"#;

        let config: NexisConfig = toml::from_str(toml_content).unwrap();
        
        assert_eq!(config.system.hostname, "test-host");
        assert_eq!(config.system.timezone, "America/New_York");
        assert!(config.system.selinux.enabled);
        assert_eq!(config.system.selinux.mode, "enforcing");
        
        assert_eq!(config.packages.len(), 2);
        assert_eq!(config.packages[0].name, "vim");
        assert_eq!(config.packages[1].name, "ncurses");
        
        assert!(config.users.contains_key("root"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = NexisConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Empty hostname should fail
        config.system.hostname.clear();
        assert!(config.validate().is_err());
        
        // Reset and test invalid SELinux mode
        config = NexisConfig::default();
        config.system.selinux.mode = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_dependency_ordering() {
        let mut config = NexisConfig::default();
        
        config.packages = vec![
            PackageConfig {
                name: "app".to_string(),
                version: "1.0".to_string(),
                dependencies: vec!["libB".to_string(), "libA".to_string()],
                prebuilt: Some("https://example.com/app.tar.gz".to_string()),
                ..Default::default()
            },
            PackageConfig {
                name: "libB".to_string(),
                version: "1.0".to_string(),
                dependencies: vec!["libA".to_string()],
                prebuilt: Some("https://example.com/libB.tar.gz".to_string()),
                ..Default::default()
            },
            PackageConfig {
                name: "libA".to_string(),
                version: "1.0".to_string(),
                dependencies: vec![],
                prebuilt: Some("https://example.com/libA.tar.gz".to_string()),
                ..Default::default()
            },
        ];

        let ordered = config.get_packages_ordered().unwrap();
        assert_eq!(ordered[0].name, "libA");
        assert_eq!(ordered[1].name, "libB");
        assert_eq!(ordered[2].name, "app");
    }

    #[test]
    fn test_var_expansion() {
        let config = NexisConfig::default();
        
        let input = "Welcome to {hostname} running version {version}";
        let expanded = config.expand_vars(input);
        
        assert!(expanded.contains("nexis"));
        assert!(expanded.contains("0.1.0"));
    }
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            prebuilt: None,
            fallback_to_source: false,
            source: None,
            patches: Vec::new(),
            pre_build_script: None,
            post_build_script: None,
            build_system: None,
            build_flags: Vec::new(),
            context_file: None,
            env: HashMap::new(),
            runtime_dirs: Vec::new(),
            hash: None,
            dependencies: Vec::new(),
        }
    }
}
