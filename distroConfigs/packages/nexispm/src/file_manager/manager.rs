use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tera::{Tera, Context};
use fs_err as fs;
use camino::Utf8PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct NexisConfig {
    pub nexis: NexisSection,
    pub system: SystemConfig,
    pub users: HashMap<String, UserConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NexisSection {
    pub files: HashMap<String, FileSpec>,
    pub directories: HashMap<String, DirectorySpec>,
    pub environment: HashMap<String, String>,
    pub dinit: HashMap<String, DinitService>,
    pub system_files: HashMap<String, FileSpec>,
    pub packages: HashMap<String, PackageConfig>,
    pub hooks: HooksConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum FileSpec {
    Simple { source: String },
    Content { content: String },
    Template { 
        template: String, 
        variables: HashMap<String, serde_json::Value> 
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
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DirectorySpec {
    pub mode: Option<String>,
    pub owner: Option<String>, 
    pub group: Option<String>,
    pub recursive: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Condition {
    pub package: Option<String>,
    pub file_exists: Option<String>,
    pub env_var: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DinitService {
    pub r#type: String, // "process", "scripted", etc.
    pub command: String,
    pub depends: Option<Vec<String>>,
    pub user: Option<String>,
    pub working_directory: Option<String>,
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageConfig {
    pub files: HashMap<String, FileSpec>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HooksConfig {
    pub on_file_change: Vec<Hook>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Hook {
    pub pattern: String,
    pub command: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub timezone: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserConfig {
    pub shell: Option<String>,
    pub home: Option<String>,
}

pub struct FileManager {
    tera: Tera,
    config: NexisConfig,
    base_path: Utf8PathBuf,
}

impl FileManager {
    pub fn new(config: NexisConfig, base_path: Utf8PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let mut tera = Tera::new(&format!("{}/**/*", base_path.join("templates")))?;
        
        // Add global variables
        let mut context = Context::new();
        context.insert("hostname", &config.system.hostname);
        context.insert("timezone", &config.system.timezone);
        
        Ok(Self {
            tera,
            config,
            base_path,
        })
    }

    pub fn apply_files(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        let user_config = self.config.users.get(user)
            .ok_or("User not found")?;
        
        let home_dir = user_config.home.as_ref()
            .unwrap_or(&format!("/home/{}", user));
        
        // Apply regular files
        for (path, spec) in &self.config.nexis.files {
            self.apply_file_spec(path, spec, home_dir, user)?;
        }

        // Apply directories
        for (path, spec) in &self.config.nexis.directories {
            self.create_directory(path, spec, home_dir)?;
        }

        // Apply package-specific files
        for (package, pkg_config) in &self.config.nexis.packages {
            if self.is_package_installed(package)? {
                for (path, spec) in &pkg_config.files {
                    self.apply_file_spec(path, spec, home_dir, user)?;
                }
            }
        }

        Ok(())
    }

    fn apply_file_spec(
        &self, 
        target_path: &str, 
        spec: &FileSpec, 
        home_dir: &str,
        user: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let full_path = if target_path.starts_with('/') {
            Utf8PathBuf::from(target_path)
        } else {
            Utf8PathBuf::from(home_dir).join(target_path)
        };

        match spec {
            FileSpec::Simple { source } => {
                let source_path = self.base_path.join("files").join(source);
                self.copy_file(&source_path, &full_path)?;
            }
            FileSpec::Content { content } => {
                fs::write(&full_path, content)?;
            }
            FileSpec::Template { template, variables } => {
                let mut context = Context::new();
                for (k, v) in variables {
                    context.insert(k, v);
                }
                context.insert("user", user);
                context.insert("home", home_dir);
                
                let rendered = self.tera.render(template, &context)?;
                fs::write(&full_path, rendered)?;
            }
            FileSpec::Advanced { 
                source, content, template, variables, 
                mode, owner, group, symlink, directory, 
                condition, backup, force 
            } => {
                // Check condition
                if let Some(cond) = condition {
                    if !self.check_condition(cond)? {
                        return Ok(());
                    }
                }

                // Backup if requested
                if backup.unwrap_or(false) && full_path.exists() {
                    let backup_path = format!("{}.backup", full_path);
                    fs::copy(&full_path, backup_path)?;
                }

                // Handle different file types
                if directory.unwrap_or(false) {
                    fs::create_dir_all(&full_path)?;
                } else if let Some(link_target) = symlink {
                    if full_path.exists() && !force.unwrap_or(false) {
                        return Err("Target exists and force=false".into());
                    }
                    if full_path.exists() {
                        fs::remove_file(&full_path)?;
                    }
                    std::os::unix::fs::symlink(link_target, &full_path)?;
                } else if let Some(src) = source {
                    let source_path = self.base_path.join("files").join(src);
                    self.copy_file(&source_path, &full_path)?;
                } else if let Some(cnt) = content {
                    fs::write(&full_path, cnt)?;
                } else if let Some(tmpl) = template {
                    let mut context = Context::new();
                    if let Some(vars) = variables {
                        for (k, v) in vars {
                            context.insert(k, v);
                        }
                    }
                    context.insert("user", user);
                    context.insert("home", home_dir);
                    
                    let rendered = self.tera.render(tmpl, &context)?;
                    fs::write(&full_path, rendered)?;
                }

                // Set permissions
                if let Some(mode_str) = mode {
                    let mode = u32::from_str_radix(mode_str.trim_start_matches("0o"), 8)?;
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(mode);
                    fs::set_permissions(&full_path, perms)?;
                }

                // TODO: Set owner/group using users crate
            }
        }

        Ok(())
    }

    fn create_directory(
        &self, 
        path: &str, 
        spec: &DirectorySpec,
        home_dir: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let full_path = if path.starts_with('/') {
            Utf8PathBuf::from(path)
        } else {
            Utf8PathBuf::from(home_dir).join(path)
        };

        if spec.recursive.unwrap_or(false) {
            fs::create_dir_all(&full_path)?;
        } else {
            fs::create_dir(&full_path)?;
        }

        if let Some(mode_str) = &spec.mode {
            let mode = u32::from_str_radix(mode_str.trim_start_matches("0o"), 8)?;
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(mode);
            fs::set_permissions(&full_path, perms)?;
        }

        Ok(())
    }

    fn copy_file(&self, source: &Utf8PathBuf, target: &Utf8PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
        Ok(())
    }

    fn check_condition(&self, condition: &Condition) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(pkg) = &condition.package {
            return self.is_package_installed(pkg);
        }
        
        if let Some(file_path) = &condition.file_exists {
            return Ok(std::path::Path::new(file_path).exists());
        }
        
        if let Some(env_var) = &condition.env_var {
            return Ok(std::env::var(env_var).is_ok());
        }

        Ok(true)
    }

    fn is_package_installed(&self, _package: &str) -> Result<bool, Box<dyn std::error::Error>> {
        // TODO: Check against your package database
        Ok(true)
    }
}

// Usage example
pub fn apply_user_config(config_path: &str, user: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(config_path)?;
    let config: NexisConfig = toml::from_str(&config_content)?;
    
    let base_path = Utf8PathBuf::from(config_path)
        .parent()
        .unwrap_or(&Utf8PathBuf::from("."))
        .to_path_buf();
    
    let file_manager = FileManager::new(config, base_path)?;
    file_manager.apply_files(user)?;
    
    println!("Applied configuration for user: {}", user);
    Ok(())
}
