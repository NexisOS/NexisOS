use anyhow::{bail, Context, Result};
use std::path::Path;

use super::SnapshotBackend;
use crate::snapshots::archive_util::{
    archive_dir, extract_archive, list_generations_vec, SNAPSHOT_ROOT,
};

const REQUIRED_SUBVOLUMES: &[&str] = &["/@sys", "/@nexstore", "/@nexpmd"];

pub struct BtrfsBackend;

impl BtrfsBackend {
    pub fn new() -> Self {
        Self
    }

    fn is_btrfs_subvolume(path: &Path) -> Result<bool> {
        Ok(std::process::Command::new("btrfs")
            .args(["subvolume", "show"])
            .arg(path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false))
    }
}

impl SnapshotBackend for BtrfsBackend {
    fn ensure_prereqs(&self) -> Result<()> {
        for sv in REQUIRED_SUBVOLUMES {
            if !Self::is_btrfs_subvolume(Path::new(sv))? {
                bail!("{sv} is not a Btrfs subvolume or inaccessible.");
            }
        }
        Ok(())
    }

    fn snapshot_after_apply(&self, gen_id: u64) -> Result<()> {
        let gen_path = Path::new(SNAPSHOT_ROOT).join(gen_id.to_string());
        std::fs::create_dir_all(&gen_path)
            .with_context(|| format!("creating snapshot dir {}", gen_path.display()))?;
        for sv in REQUIRED_SUBVOLUMES {
            let src = Path::new(sv);
            let archive_path = gen_path.join(format!("{}.tar.zst", src.file_name().unwrap().to_string_lossy()));
            archive_dir(src, &archive_path)?;
        }
        Ok(())
    }

    fn rollback(&self, steps: u32) -> Result<()> {
        let generations = list_generations_vec(Path::new(SNAPSHOT_ROOT))?;
        if generations.len() <= steps as usize {
            bail!("Not enough generations to roll back {} steps", steps);
        }
        let target_gen = generations[generations.len() - 1 - steps as usize];
        let gen_path = Path::new(SNAPSHOT_ROOT).join(target_gen.to_string());

        for sv in REQUIRED_SUBVOLUMES {
            let dest = Path::new(sv);
            let archive_path = gen_path.join(format!("{}.tar.zst", dest.file_name().unwrap().to_string_lossy()));
            extract_archive(&archive_path, dest)?;
        }
        Ok(())
    }

    fn list_generations(&self) -> Result<()> {
        for g in list_generations_vec(Path::new(SNAPSHOT_ROOT))? {
            println!("{}", g);
        }
        Ok(())
    }

    fn delete_generation(&self, gen_id: u64) -> Result<()> {
        let gen_path = Path::new(SNAPSHOT_ROOT).join(gen_id.to_string());
        if gen_path.exists() {
            std::fs::remove_dir_all(&gen_path)
                .with_context(|| format!("removing generation {}", gen_id))?;
        }
        Ok(())
    }
}
