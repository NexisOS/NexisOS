use anyhow::{bail, Context, Result};
use std::path::Path;

use super::SnapshotBackend;
use crate::snapshots::archive_util::{
    archive_dir, extract_archive, list_generations_vec, SNAPSHOT_ROOT,
};

const REQUIRED_DIRS: &[&str] = &["/sysroot", "/nexstore", "/nexpmd"];

pub struct XfsBackend;

impl XfsBackend {
    pub fn new() -> Self {
        Self
    }
}

impl SnapshotBackend for XfsBackend {
    fn ensure_prereqs(&self) -> Result<()> {
        for dir in REQUIRED_DIRS {
            if !Path::new(dir).exists() {
                bail!("{dir} does not exist or is inaccessible.");
            }
        }
        Ok(())
    }

    fn snapshot_after_apply(&self, gen_id: u64) -> Result<()> {
        let gen_path = Path::new(SNAPSHOT_ROOT).join(gen_id.to_string());
        std::fs::create_dir_all(&gen_path)
            .with_context(|| format!("creating snapshot dir {}", gen_path.display()))?;
        for dir in REQUIRED_DIRS {
            let src = Path::new(dir);
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

        for dir in REQUIRED_DIRS {
            let dest = Path::new(dir);
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
