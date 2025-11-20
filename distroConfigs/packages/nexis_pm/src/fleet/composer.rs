use anyhow::Result;

pub struct FleetComposer;

impl FleetComposer {
    pub fn compose_for_machine(
        base: Config,
        profiles: &[String],
        machine_id: &str,
    ) -> Result<Config> {
        // Load profile templates
        let profile_configs: Vec<Config> = profiles
            .iter()
            .map(|name| load_profile(name))
            .collect::<Result<_>>()?;
        
        // Load machine-specific config
        let machine_config = load_machine_config(machine_id)?;
        
        // Compose: base → profiles → machine
        ConfigComposer::compose(base, profile_configs, Some(machine_config))
    }
}

fn load_profile(name: &str) -> Result<Config> {
    let path = PathBuf::from("/etc/nexis/profiles")
        .join(format!("{}.toml", name));
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn load_machine_config(id: &str) -> Result<Config> {
    let path = PathBuf::from("/etc/nexis/machines")
        .join(format!("{}.toml", id));
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}
