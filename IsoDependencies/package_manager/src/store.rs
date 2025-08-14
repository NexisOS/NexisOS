use anyhow::{Context, Result};
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::snapshot::{self, SnapshotBackend};

/// Root directory for the immutable package store.
pub fn store_root() -> PathBuf {
    PathBuf::from("/nex/store")
}

/// Path to the state DB JSON file.
pub fn state_db_path() -> PathBuf {
    PathBuf::from("/var/lib/nexpm/state.json")
}

/// Where the backend selection is stored (chosen during install).
fn backend_config_path() -> PathBuf {
    PathBuf::from("/etc/nexpm/backend")
}

/// Determine which snapshot backend is active (default: Btrfs).
pub fn current_backend() -> Result<Box<dyn SnapshotBackend>> {
    let cfg_path = backend_config_path();
    if cfg_path.exists() {
        let choice = fs::read_to_string(&cfg_path)?.trim().to_lowercase();
        debug!("Backend config found: '{}'", choice);
        match choice.as_str() {
            "btrfs" => {
                info!("Using Btrfs snapshot backend");
                return Ok(Box::new(snapshot::BtrfsBackend));
            }
            "xfs" => {
                info!("Using XFS snapshot backend");
                return Ok(Box::new(snapshot::XfsBackend));
            }
            _ => {
                warn!("Unknown backend '{}', falling back to Btrfs", choice);
            }
        }
    } else {
        debug!("No backend config found, using default Btrfs backend");
    }
    Ok(Box::new(snapshot::BtrfsBackend))
}

/// Ensure store and state DB directories exist.
pub fn ensure_dirs() -> Result<()> {
    debug!("Ensuring store directory exists at {:?}", store_root());
    fs::create_dir_all(store_root()).context("creating /nex/store")?;
    if let Some(p) = state_db_path().parent() {
        debug!("Ensuring state DB directory exists at {:?}", p);
        fs::create_dir_all(p)?;
    }
    Ok(())
}

/// Compute SHA-256 of a file.
pub fn sha256_file(path: &Path) -> Result<String> {
    debug!("Computing SHA-256 for file: {:?}", path);
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let digest = hex::encode(hasher.finalize());
    debug!("SHA-256 for {:?} = {}", path, digest);
    Ok(digest)
}

/// Compute the destination path for a stored package.
pub fn path_for(name: &str, version: &str, hash: &str) -> PathBuf {
    let dest = store_root().join(format!("{}-{}-{}", name, version, &hash[..12]));
    debug!(
        "Computed path for package '{}', version '{}', hash prefix '{}': {:?}",
        name,
        version,
        &hash[..12],
        dest
    );
    dest
}
