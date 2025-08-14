use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Central location for archive-based snapshots.
pub const SNAPSHOT_ROOT: &str = "/.nexis_snapshots";

/// Compress a directory into a .tar.zst archive
pub fn archive_dir(src: &Path, dest_file: &Path) -> Result<()> {
    fs::create_dir_all(dest_file.parent().unwrap())
        .with_context(|| format!("creating archive parent dir {}", dest_file.display()))?;
    run_ok(
        Command::new("tar")
            .args(["-I", "zstd", "-cf"])
            .arg(dest_file)
            .arg("-C")
            .arg(
                src.parent()
                    .context("source directory has no parent")?,
            )
            .arg(src.file_name().context("source path has no filename")?),
    )
    .with_context(|| format!("archiving {} to {}", src.display(), dest_file.display()))
}

/// Extract a .tar.zst archive into a directory
pub fn extract_archive(archive: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)
            .with_context(|| format!("removing existing {}", dest.display()))?;
    }
    fs::create_dir_all(dest.parent().unwrap())
        .with_context(|| format!("creating extraction parent dir {}", dest.display()))?;
    run_ok(
        Command::new("tar")
            .args(["-I", "zstd", "-xf"])
            .arg(archive)
            .arg("-C")
            .arg(
                dest.parent()
                    .context("destination directory has no parent")?,
            ),
    )
    .with_context(|| format!("extracting {} to {}", archive.display(), dest.display()))
}

/// List numeric snapshot generations in a given root directory
pub fn list_generations_vec(snapshot_root: &Path) -> Result<Vec<u64>> {
    let mut gens = Vec::new();
    if let Ok(entries) = fs::read_dir(snapshot_root) {
        for entry in entries.flatten() {
            if let Ok(fname) = entry.file_name().into_string() {
                if let Ok(num) = fname.parse::<u64>() {
                    gens.push(num);
                }
            }
        }
    }
    gens.sort_unstable();
    Ok(gens)
}

/// Helper to run commands and bail on error
pub fn run_ok(cmd: &mut Command) -> Result<()> {
    let cmd_str = format!("{:?}", cmd);
    let output = cmd.output().with_context(|| format!("running command: {}", cmd_str))?;
    if output.status.success() {
        return Ok(());
    }
    bail!(
        "command failed (code {}): {}\nCommand: {}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).trim(),
        cmd_str
    )
}
