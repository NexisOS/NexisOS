use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::types::Config;

/// Load a config TOML file and merge in any `includes.paths` TOML files.
pub fn load_config_with_includes(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    let mut cfg: Config = toml::from_str(&content)?;

    if let Some(inc) = cfg.includes.clone() {
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        for p in inc.paths {
            let child = base.join(&p);
            let s = fs::read_to_string(&child)?;
            let sub: Config = toml::from_str(&s)?;

            // Shallow merge for now — just extend package list
            let mut pkgs = cfg.packages;
            pkgs.extend(sub.packages);
            cfg.packages = pkgs;
        }
    }

    Ok(cfg)
}

/// Default config path inside NexisOS.
pub fn default_config_path() -> PathBuf {
    PathBuf::from("/etc/package_manager/config.toml")
}
