use anyhow::{Context, Result};
use std::process::Command;

pub mod archive_util; // shared tar/zstd logic
pub mod btrfs;
pub mod xfs;

/// Trait that snapshot backends (Btrfs, XFS, etc.) must implement.
pub trait SnapshotBackend {
    /// Ensure that any required tools or filesystem features are available.
    fn ensure_prereqs(&self) -> Result<()>;

    /// Create a snapshot after applying a new generation.
    fn snapshot_after_apply(&self, gen_id: u64) -> Result<()>;

    /// Roll back to a previous generation by number of steps.
    fn rollback(&self, steps: u32) -> Result<()>;

    /// List all stored snapshot generations.
    fn list_generations(&self) -> Result<()>;

    /// Delete a specific snapshot generation.
    fn delete_generation(&self, gen_id: u64) -> Result<()>;
}

/// Return the correct backend based on filesystem type.
/// Only Btrfs and XFS are supported for snapshots.
pub fn get_backend() -> Result<Box<dyn SnapshotBackend>> {
    let fs_type = detect_fs_type("/")?;
    match fs_type.as_str() {
        "btrfs" => Ok(Box::new(btrfs::BtrfsBackend::new())),
        "xfs" => Ok(Box::new(xfs::XfsBackend::new())),
        "ext4" | "ext3" | "ext2" => anyhow::bail!(
            "Filesystem '{}' detected — snapshots are not supported on ext filesystems. \
             Please use Btrfs or XFS if you want rollback support.",
            fs_type
        ),
        other => anyhow::bail!("Unsupported filesystem type: '{}'", other),
    }
}

/// Detect the filesystem type for a given path.
/// Uses `stat -f -c %T` which returns e.g. "btrfs", "xfs", "ext2/ext3".
fn detect_fs_type(path: &str) -> Result<String> {
    let output = Command::new("stat")
        .args(["-f", "-c", "%T", path])
        .output()
        .context("running stat to detect filesystem type")?;
    if !output.status.success() {
        anyhow::bail!("failed to detect filesystem type for {}", path);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
