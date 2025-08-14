use crate::{
    config::load_config_with_includes,
    manifest::{apply_placeholders, resolve_latest_git_tag},
    rollback::snapshot_after_apply,
    grub::update_grub,
    store::{ensure_dirs, path_for, sha256_file, state_db_path},
    types::{Config, Generation, InstalledPackage, Package, StateDb},
    snapshots::{SnapshotBackend, btrfs::BtrfsBackend},
};
use anyhow::{Context, Result};
use chrono::Utc;
use log::{debug, info, warn};
use std::{fs, path::PathBuf};

/// Apply a declarative config and install all listed packages.
pub fn apply<B: SnapshotBackend>(config_path: Option<&str>, backend: &B) -> Result<()> {
    ensure_dirs()?;

    let cfg_path = config_path
        .map(PathBuf::from)
        .unwrap_or(crate::config::default_config_path());
    debug!("Loading config from {:?}", cfg_path);

    let cfg = load_config_with_includes(&cfg_path)?;

    let mut installed = Vec::new();
    let mut changed = false;

    for mut p in cfg.packages {
        debug!("Processing package `{}`", p.name);

        let ver = if p.version.as_deref() == Some("latest") {
            let src = p
                .source
                .as_ref()
                .context("version=latest requires `source` to a git repo")?;
            debug!("Resolving latest git tag from {}", src);
            resolve_latest_git_tag(src)?
        } else {
            p.version.clone().unwrap_or_else(|| "unknown".into())
        };

        let (store_path, files) = install_one(&p, &ver)?;

        if !files.is_empty() {
            changed = true;
        }

        installed.push(InstalledPackage {
            name: p.name.clone(),
            version: ver,
            store_path: store_path.to_string_lossy().into_owned(),
            installed_at: Utc::now().to_rfc3339(),
            files,
        });
    }

    if !changed {
        info!("No changes detected — skipping new generation.");
        return Ok(());
    }

    debug!("Reading existing state DB from {:?}", state_db_path());
    let mut state: StateDb = if state_db_path().exists() {
        serde_json::from_str(&fs::read_to_string(state_db_path())?)?
    } else {
        Default::default()
    };
    state.generation += 1;
    state.generations.push(Generation {
        id: state.generation,
        timestamp: Utc::now().to_rfc3339(),
        packages: installed,
    });
    fs::write(state_db_path(), serde_json::to_string_pretty(&state)?)?;
    info!("New generation {} created.", state.generation);

    // Create snapshot after applying packages
    snapshot_after_apply(backend, state.generation)?;
    debug!("Snapshot created for generation {}", state.generation);

    // Update GRUB entries for new generation
    update_grub()?;
    debug!("GRUB updated for generation {}", state.generation);

    Ok(())
}

pub fn install_single(name: &str) -> Result<()> {
    let cfg = load_config_with_includes(&crate::config::default_config_path())?;
    let pkg = cfg
        .packages
        .into_iter()
        .find(|p| p.name == name)
        .with_context(|| format!("package `{}` not found in config", name))?;
    info!("Installing single package `{}`", name);
    let ver = pkg.version.clone().unwrap_or_else(|| "unknown".into());
    let _ = install_one(&pkg, &ver)?;
    Ok(())
}

pub fn status() -> Result<()> {
    if !state_db_path().exists() {
        info!("No generations yet.");
        return Ok(());
    }
    let state: StateDb = serde_json::from_str(&fs::read_to_string(state_db_path())?)?;
    info!(
        "Current generation: {} ({} total)",
        state.generation,
        state.generations.len()
    );
    if let Some(g) = state.generations.last() {
        info!("\nGeneration #{} @ {}:", g.id, g.timestamp);
        for p in &g.packages {
            info!("- {} {} -> {}", p.name, p.version, p.store_path);
        }
    }
    Ok(())
}

fn install_one(pkg: &Package, resolved_version: &str) -> Result<(PathBuf, Vec<String>)> {
    ensure_dirs()?;

    if let Some(prebuilt) = &pkg.prebuilt {
        let url = apply_placeholders(prebuilt, resolved_version);
        info!("Downloading `{}` version {} from {}", pkg.name, resolved_version, url);
        let archive = download(&url)?;
        let hash = sha256_file(&archive)?;
        debug!("Downloaded archive hash: {}", hash);
        let dest = path_for(&pkg.name, resolved_version, &hash);
        if dest.exists() {
            debug!("Package already exists in store at {:?}", dest);
            return Ok((dest, vec![]));
        }
        fs::create_dir_all(&dest)?;
        extract_tarball(&archive, &dest)?;
        info!("Extracted `{}` to {:?}", pkg.name, dest);

        let file_list = collect_files(&dest)?;
        debug!("Installed files: {:?}", file_list);
        Ok((dest, file_list))
    } else {
        anyhow::bail!("source builds not implemented yet for `{}`", pkg.name)
    }
}

fn download(url: &str) -> Result<PathBuf> {
    debug!("Starting download from {}", url);
    let resp = reqwest::blocking::get(url)?.error_for_status()?;
    let tmp = std::env::temp_dir().join("nexpm-download.tar.gz");
    let mut file = fs::File::create(&tmp)?;
    let bytes = resp.bytes()?;
    std::io::copy(&mut bytes.as_ref(), &mut file)?;
    debug!("Saved download to {:?}", tmp);
    Ok(tmp)
}

fn extract_tarball(archive: &PathBuf, dest: &PathBuf) -> Result<()> {
    debug!("Extracting {:?} to {:?}", archive, dest);
    let tar_gz = fs::File::open(archive)?;
    let decompressor = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(decompressor);
    archive.unpack(dest)?;
    Ok(())
}

fn collect_files(dir: &PathBuf) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.path().strip_prefix(dir)?.to_string_lossy().into_owned());
        }
    }
    Ok(files)
}
