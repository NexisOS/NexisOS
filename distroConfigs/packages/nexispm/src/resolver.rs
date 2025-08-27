//! # Package Resolution Module
//!
//! Handles version resolution for packages, particularly resolving "latest" 
//! version specifications to concrete git tags using `git ls-remote`.
//! Also handles dependency resolution and build ordering.

use anyhow::{Context, Result};
use log::{debug, info, warn, error};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::config::{NexisConfig, PackageConfig, VersionSpec};

/// Package resolver error types
#[derive(thiserror::Error, Debug)]
pub enum ResolverError {
    #[error("Failed to resolve version for package '{package}': {msg}")]
    VersionResolution { package: String, msg: String },
    
    #[error("Git command failed for package '{package}': {msg}")]
    GitError { package: String, msg: String },
    
    #[error("No valid tags found for package '{package}' in repository '{repo}'")]
    NoValidTags { package: String, repo: String },
    
    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("Package '{package}' not found")]
    PackageNotFound { package: String },
    
    #[error("Invalid semantic version '{version}' for package '{package}'")]
    InvalidVersion { package: String, version: String },
    
    #[error("Network error resolving '{package}': {msg}")]
    NetworkError { package: String, msg: String },
    
    #[error("Dependency '{dependency}' of package '{package}' not found")]
    DependencyNotFound { package: String, dependency: String },
}

/// A resolved package configuration with concrete version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPackageConfig {
    /// Base package configuration
    #[serde(flatten)]
    pub config: PackageConfig,
    
    /// Resolved concrete version (replaces "latest" with actual tag)
    pub resolved_version: String,
    
    /// Resolved source URL with version interpolation
    pub resolved_source: Option<String>,
    
    /// Resolved prebuilt URL with template variables filled
    pub resolved_prebuilt: Option<String>,
    
    /// Build order index (for dependency ordering)
    pub build_order: usize,
}

/// Semantic version for comparison
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

impl SemanticVersion {
    /// Parse a semantic version from a string
    pub fn parse(version: &str) -> Result<Self> {
        // Remove 'v' prefix if present
        let version = version.strip_prefix('v').unwrap_or(version);
        
        // Split on '-' for prerelease
        let (version_part, prerelease) = if let Some(dash_pos) = version.find('-') {
            let (v, pre) = version.split_at(dash_pos);
            (v, Some(version[dash_pos + 1..].to_string()))
        } else {
            (version, None)
        };
        
        // Parse major.minor.patch
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid semantic version format: {}", version);
        }
        
        let major = parts[0].parse().context("Invalid major version")?;
        let minor = parts[1].parse().context("Invalid minor version")?;
        let patch = parts.get(2).unwrap_or(&"0").parse().context("Invalid patch version")?;
        
        Ok(SemanticVersion {
            major,
            minor,
            patch,
            prerelease,
        })
    }
    
    /// Check if this is a stable release (no prerelease)
    pub fn is_stable(&self) -> bool {
        self.prerelease.is_none()
    }
}

/// Cache entry for resolved git tags
#[derive(Debug, Clone)]
struct GitTagCache {
    tags: Vec<String>,
    timestamp: std::time::SystemTime,
}

/// Package resolver with caching and dependency resolution
pub struct PackageResolver {
    config: Arc<NexisConfig>,
    git_tag_cache: Arc<RwLock<HashMap<String, GitTagCache>>>,
    cache_ttl: std::time::Duration,
}

impl PackageResolver {
    /// Create a new package resolver
    pub fn new(config: Arc<NexisConfig>) -> Self {
        Self {
            config,
            git_tag_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: std::time::Duration::from_secs(300), // 5 minutes
        }
    }
    
    /// Resolve all packages in the configuration
    pub async fn resolve_all_packages(&self, packages: &[PackageConfig]) -> Result<Vec<ResolvedPackageConfig>> {
        info!("Resolving {} packages with dependency ordering", packages.len());
        
        // First, resolve all versions (especially "latest" ones)
        let mut resolved_packages = Vec::new();
        for package in packages {
            let resolved = self.resolve_package_version(package).await
                .with_context(|| format!("Failed to resolve package '{}'", package.name))?;
            resolved_packages.push(resolved);
        }
        
        // Then, resolve dependencies and determine build order
        let ordered_packages = self.resolve_dependencies(resolved_packages)
            .context("Failed to resolve package dependencies")?;
        
        info!("Successfully resolved {} packages in dependency order", ordered_packages.len());
        Ok(ordered_packages)
    }
    
    /// Resolve a single package's version specification
    async fn resolve_package_version(&self, package: &PackageConfig) -> Result<ResolvedPackageConfig> {
        let resolved_version = match &package.version {
            VersionSpec::Latest => {
                if let Some(source) = &package.source {
                    self.resolve_latest_git_tag(&package.name, source).await?
                } else {
                    return Err(ResolverError::VersionResolution {
                        package: package.name.clone(),
                        msg: "Cannot resolve 'latest' version without source repository".to_string(),
                    }.into());
                }
            }
            VersionSpec::Exact(version) => {
                if version == "latest" {
                    // Handle string "latest" same as enum Latest
                    if let Some(source) = &package.source {
                        self.resolve_latest_git_tag(&package.name, source).await?
                    } else {
                        return Err(ResolverError::VersionResolution {
                            package: package.name.clone(),
                            msg: "Cannot resolve 'latest' version without source repository".to_string(),
                        }.into());
                    }
                } else {
                    version.clone()
                }
            }
            VersionSpec::Git { git_ref } => git_ref.clone(),
        };
        
        debug!("Resolved package '{}' to version '{}'", package.name, resolved_version);
        
        // Resolve template variables in URLs
        let resolved_source = package.source.as_ref().map(|s| {
            self.interpolate_template_vars(s, &package.name, &resolved_version)
        });
        
        let resolved_prebuilt = package.prebuilt.as_ref().map(|s| {
            self.interpolate_template_vars(s, &package.name, &resolved_version)
        });
        
        Ok(ResolvedPackageConfig {
            config: package.clone(),
            resolved_version,
            resolved_source,
            resolved_prebuilt,
            build_order: 0, // Will be set during dependency resolution
        })
    }
    
    /// Resolve "latest" git tag using git ls-remote
    async fn resolve_latest_git_tag(&self, package_name: &str, repo_url: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.git_tag_cache.read().await;
            if let Some(cached) = cache.get(repo_url) {
                if cached.timestamp.elapsed().unwrap_or(self.cache_ttl) < self.cache_ttl {
                    debug!("Using cached git tags for {}", repo_url);
                    return self.find_latest_semver_tag(package_name, &cached.tags);
                }
            }
        }
        
        debug!("Fetching git tags for package '{}' from '{}'", package_name, repo_url);
        
        // Execute git ls-remote to get all tags
        let output = Command::new("git")
            .args(["ls-remote", "--tags", "--refs", repo_url])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn git command")?
            .wait_with_output()
            .await
            .context("Failed to wait for git command")?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ResolverError::GitError {
                package: package_name.to_string(),
                msg: format!("git ls-remote failed: {}", error_msg),
            }.into());
        }
        
        // Parse git ls-remote output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tags = Vec::new();
        
        for line in stdout.lines() {
            if let Some(tag_ref) = line.split_whitespace().nth(1) {
                if let Some(tag_name) = tag_ref.strip_prefix("refs/tags/") {
                    tags.push(tag_name.to_string());
                }
            }
        }
        
        if tags.is_empty() {
            return Err(ResolverError::NoValidTags {
                package: package_name.to_string(),
                repo: repo_url.to_string(),
            }.into());
        }
        
        debug!("Found {} tags for package '{}'", tags.len(), package_name);
        
        // Cache the results
        {
            let mut cache = self.git_tag_cache.write().await;
            cache.insert(repo_url.to_string(), GitTagCache {
                tags: tags.clone(),
                timestamp: std::time::SystemTime::now(),
            });
        }
        
        // Find the latest semantic version tag
        self.find_latest_semver_tag(package_name, &tags)
    }
    
    /// Find the latest semantic version tag from a list of tags
    fn find_latest_semver_tag(&self, package_name: &str, tags: &[String]) -> Result<String> {
        let mut valid_versions = Vec::new();
        
        for tag in tags {
            if let Ok(semver) = SemanticVersion::parse(tag) {
                // Prefer stable releases over prereleases
                if semver.is_stable() {
                    valid_versions.push((semver, tag));
                }
            }
        }
        
        // If no stable versions found, include prereleases
        if valid_versions.is_empty() {
            for tag in tags {
                if let Ok(semver) = SemanticVersion::parse(tag) {
                    valid_versions.push((semver, tag));
                }
            }
        }
        
        if valid_versions.is_empty() {
            return Err(ResolverError::NoValidTags {
                package: package_name.to_string(),
                repo: "unknown".to_string(),
            }.into());
        }
        
        // Sort by semantic version (latest first)
        valid_versions.sort_by(|(a, _), (b, _)| b.cmp(a));
        
        let latest_tag = valid_versions[0].1.clone();
        info!("Resolved '{}' latest version to: {}", package_name, latest_tag);
        
        Ok(latest_tag)
    }
    
    /// Resolve package dependencies and determine build order
    fn resolve_dependencies(&self, packages: Vec<ResolvedPackageConfig>) -> Result<Vec<ResolvedPackageConfig>> {
        let package_map: HashMap<String, ResolvedPackageConfig> = packages
            .into_iter()
            .map(|pkg| (pkg.config.name.clone(), pkg))
            .collect();
        
        // Build dependency graph
        let mut dependency_graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        
        for (name, package) in &package_map {
            dependency_graph.insert(name.clone(), package.config.dependencies.clone());
            in_degree.insert(name.clone(), 0);
        }
        
        // Calculate in-degrees
        for dependencies in dependency_graph.values() {
            for dep in dependencies {
                if !package_map.contains_key(dep) {
                    return Err(ResolverError::DependencyNotFound {
                        package: "unknown".to_string(), // We'd need to track this better
                        dependency: dep.clone(),
                    }.into());
                }
                *in_degree.get_mut(dep).unwrap() += 1;
            }
        }
        
        // Topological sort using Kahn's algorithm
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter_map(|(name, &degree)| if degree == 0 { Some(name.clone()) } else { None })
            .collect();
        
        let mut build_order = Vec::new();
        let mut processed = HashSet::new();
        
        while let Some(package_name) = queue.pop_front() {
            if processed.contains(&package_name) {
                continue;
            }
            
            build_order.push(package_name.clone());
            processed.insert(package_name.clone());
            
            // Process dependencies
            if let Some(dependencies) = dependency_graph.get(&package_name) {
                for dep in dependencies {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }
        
        // Check for circular dependencies
        if build_order.len() != package_map.len() {
            let remaining: Vec<_> = package_map.keys()
                .filter(|name| !processed.contains(*name))
                .collect();
            return Err(ResolverError::CircularDependency {
                cycle: remaining.join(" -> "),
            }.into());
        }
        
        // Create ordered resolved packages
        let mut ordered_packages = Vec::new();
        for (index, package_name) in build_order.iter().enumerate() {
            if let Some(mut package) = package_map.get(package_name).cloned() {
                package.build_order = index;
                ordered_packages.push(package);
            }
        }
        
        debug!("Build order: {}", build_order.join(" -> "));
        Ok(ordered_packages)
    }
    
    /// Interpolate template variables in URLs
    fn interpolate_template_vars(&self, template: &str, package_name: &str, version: &str) -> String {
        template
            .replace("{name}", package_name)
            .replace("{version}", version)
            .replace("{tag}", version)
            .replace("{arch}", &self.get_system_arch())
    }
    
    /// Get system architecture for template interpolation
    fn get_system_arch(&self) -> String {
        std::env::consts::ARCH.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_semantic_version_parsing() {
        let v1 = SemanticVersion::parse("1.2.3").unwrap();
        assert_eq!(v1.major, 1);
        assert_eq!(v1.minor, 2);
        assert_eq!(v1.patch, 3);
        assert!(v1.is_stable());
        
        let v2 = SemanticVersion::parse("v2.0.0-beta.1").unwrap();
        assert_eq!(v2.major, 2);
        assert_eq!(v2.minor, 0);
        assert_eq!(v2.patch, 0);
        assert!(!v2.is_stable());
        
        let v3 = SemanticVersion::parse("1.0").unwrap();
        assert_eq!(v3.patch, 0);
    }
    
    #[test]
    fn test_version_comparison() {
        let v1 = SemanticVersion::parse("1.2.3").unwrap();
        let v2 = SemanticVersion::parse("1.3.0").unwrap();
        let v3 = SemanticVersion::parse("2.0.0").unwrap();
        
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }
    
    #[test]
    fn test_template_interpolation() {
        let config = Arc::new(NexisConfig {
            system: crate::config::SystemConfig {
                hostname: "test".to_string(),
                timezone: "UTC".to_string(),
                version: "0.1.0".to_string(),
                kernel: "linux".to_string(),
                kernel_source: "".to_string(),
                kernel_config: std::path::PathBuf::new(),
                storage_backend: "ext4".to_string(),
                store_path: std::path::PathBuf::from("/store"),
                grub_config_path: std::path::PathBuf::new(),
                selinux: None,
                firewall: None,
                locale: None,
            },
            users: std::collections::HashMap::new(),
            network: crate::config::NetworkConfig {
                interface: "eth0".to_string(),
                dhcp: true,
                static_ip: None,
                gateway: None,
                dns: None,
            },
            packages: Vec::new(),
            config_files: std::collections::HashMap::new(),
            dinit_services: std::collections::HashMap::new(),
            log_rotation: Vec::new(),
            includes: None,
            config_dir: std::path::PathBuf::new(),
        });
        
        let resolver = PackageResolver::new(config);
        let result = resolver.interpolate_template_vars(
            "https://github.com/{name}/releases/download/{tag}/{name}-{version}-linux-{arch}.tar.gz",
            "vim",
            "v9.0.0"
        );
        
        assert!(result.contains("vim"));
        assert!(result.contains("v9.0.0"));
        assert!(result.contains(&std::env::consts::ARCH));
    }
}
