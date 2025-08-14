use anyhow::{bail, Context, Result};
use log::{debug, info, warn};
use std::{fs, path::Path};

use crate::store::{current_backend, state_db_path};
use crate::types::StateDb;
use crate::grub::update_grub;

/// Called at end of `apply()` after StateDb updated.
pub fn snapshot_after_apply(gen_id: u64) -> Result<()> {
    debug!("Starting snapshot_after_apply for generation {}", gen_id);
    let backend = current_backend()?;
    backend.ensure_prereqs()?;

    backend.snapshot_ro("/@sys", &format!("/@sys-gen-{}", gen_id))?;
    backend.snapshot_ro("/@nexstore", &format!("/@nexstore-gen-{}", gen_id))?;
    backend.snapshot_ro("/@nexpmd", &format!("/@nexpmd-gen-{}", gen_id))?;

    update_grub().context("updating GRUB after snapshot")?;
    debug!("Completed snapshot_after_apply for generation {}", gen_id);
    Ok(())
}

/// Restore only the package store and nexpm state from a previous generation.
pub fn rollback(steps: u32) -> Result<()> {
    debug!("Initiating rollback by {} steps", steps);
    let backend = current_backend()?;
    backend.ensure_prereqs()?;

    let db_path = state_db_path();
    if !db_path.exists() {
        bail!("no state DB found at {}", db_path.display());
    }
    let mut state: StateDb = serde_json::from_str(&fs::read_to_string(&db_path)?)?;
    if state.generation == 0 {
        bail!("no generations recorded yet");
    }

    let cur = state.generation as i64;
    let target = cur - (steps as i64);
    if target <= 0 {
        bail!("invalid rollback target {}", target);
    }
    let target_u = target as u64;

    let snap_store = format!("/@nexstore-gen-{}", target_u);
    let snap_state = format!("/@nexpmd-gen-{}", target_u);
    if !Path::new(&snap_store).exists() || !Path::new(&snap_state).exists() {
        bail!("missing snapshots for generation {}", target_u);
    }

    backend.delete("/@nexstore")?;
    backend.snapshot_rw(&snap_store, "/@nexstore")?;

    backend.delete("/@nexpmd")?;
    backend.snapshot_rw(&snap_state, "/@nexpmd")?;

    state.generation = target_u;
    fs::write(&db_path, serde_json::to_string_pretty(&state)?)?;

    info!("Rolled back store/state to generation {}.", target_u);
    println!(
        "For full rollback, boot 'NexisOS (generation {})' in GRUB.",
        target_u
    );
    debug!("Rollback to generation {} completed", target_u);
    Ok(())
}

/// List all available generations and their snapshot status.
pub fn list_generations() -> Result<()> {
    debug!("Listing generations from state DB");
    let db_path = state_db_path();
    if !db_path.exists() {
        bail!("no state DB found");
    }
    let state: StateDb = serde_json::from_str(&fs::read_to_string(&db_path)?)?;

    println!("Available generations:");
    for gen in 1..=state.generation {
        let sys_exists = Path::new(&format!("/@sys-gen-{}", gen)).exists();
        let store_exists = Path::new(&format!("/@nexstore-gen-{}", gen)).exists();
        let state_exists = Path::new(&format!("/@nexpmd-gen-{}", gen)).exists();
        println!(
            "  {}: sys={} store={} state={}",
            gen, sys_exists, store_exists, state_exists
        );
        debug!(
            "Gen {} -> sys: {}, store: {}, state: {}",
            gen, sys_exists, store_exists, state_exists
        );
    }
    Ok(())
}

/// Delete one or more generations.
pub fn delete_generations(gen_ids: &[u64]) -> Result<()> {
    debug!("Deleting generations: {:?}", gen_ids);
    let backend = current_backend()?;
    backend.ensure_prereqs()?;

    let db_path = state_db_path();
    if !db_path.exists() {
        bail!("no state DB found");
    }
    let state: StateDb = serde_json::from_str(&fs::read_to_string(&db_path)?)?;

    for &gen_id in gen_ids {
        if gen_id == state.generation {
            warn!("Skipping deletion of current generation {}.", gen_id);
            println!("Skipping deletion of current generation {}.", gen_id);
            continue;
        }
        for sv in [
            format!("/@sys-gen-{}", gen_id),
            format!("/@nexstore-gen-{}", gen_id),
            format!("/@nexpmd-gen-{}", gen_id),
        ] {
            if Path::new(&sv).exists() {
                backend
                    .delete(&sv)
                    .with_context(|| format!("deleting snapshot {}", sv))?;
            }
        }
        info!("Deleted snapshots for generation {}.", gen_id);
        println!("Deleted snapshots for generation {}.", gen_id);
    }

    update_grub().context("updating GRUB after deleting generations")?;
    debug!("Finished deleting generations: {:?}", gen_ids);
    Ok(())
}
