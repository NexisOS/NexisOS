use std::path::PathBuf;
use anyhow::Result;

pub struct GenerationManager {
    generations_dir: PathBuf,
}

impl GenerationManager {
    pub fn new(generations_dir: PathBuf) -> Self {
        Self { generations_dir }
    }
    
    pub fn create_generation(&self, config: &Config) -> Result<u64> {
        let gen_id = self.next_generation_id()?;
        let gen_path = self.generations_dir.join(gen_id.to_string());
        
        fs::create_dir_all(&gen_path)?;
        
        // Save config snapshot
        let config_path = gen_path.join("config.toml");
        let config_str = toml::to_string(config)?;
        fs::write(config_path, config_str)?;
        
        Ok(gen_id)
    }
    
    pub fn switch_generation(&self, gen_id: u64) -> Result<()> {
        let gen_path = self.generations_dir.join(gen_id.to_string());
        let current_link = self.generations_dir.join("current");
        
        // Atomic symlink switch
        let temp_link = self.generations_dir.join(".current.tmp");
        std::os::unix::fs::symlink(&gen_path, &temp_link)?;
        fs::rename(temp_link, current_link)?;
        
        Ok(())
    }
    
    pub fn list_generations(&self) -> Result<Vec<u64>> {
        let mut generations = Vec::new();
        for entry in fs::read_dir(&self.generations_dir)? {
            let entry = entry?;
            if let Ok(gen_id) = entry.file_name().to_string_lossy().parse::<u64>() {
                generations.push(gen_id);
            }
        }
        generations.sort();
        Ok(generations)
    }
}
