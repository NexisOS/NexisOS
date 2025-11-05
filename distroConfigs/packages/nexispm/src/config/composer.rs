use merge::Merge;
use anyhow::Result;

pub struct ConfigComposer;

impl ConfigComposer {
    /// Compose configuration: base → profiles → machine → user overrides
    pub fn compose(
        base: Config,
        profiles: Vec<Config>,
        machine: Option<Config>,
    ) -> Result<Config> {
        let mut composed = base;
        
        // Merge profiles
        for profile in profiles {
            composed.merge(profile)?;
        }
        
        // Merge machine-specific config
        if let Some(machine_config) = machine {
            composed.merge(machine_config)?;
        }
        
        Ok(composed)
    }
}
