use selinux::{SELinux, FileClass};
use anyhow::Result;

pub struct SELinuxManager;

impl SELinuxManager {
    pub fn enforce_immutability(&self, path: &Path) -> Result<()> {
        if !SELinux::is_enabled() {
            return Ok(());
        }
        
        // Set immutable_dir_t context
        SELinux::setfilecon(path, "system_u:object_r:immutable_dir_t:s0")?;
        Ok(())
    }
    
    pub fn allow_nexispm_write(&self, path: &Path) -> Result<()> {
        // Only nexispm_t domain can write to immutable paths
        // This is enforced by SELinux policy, not runtime code
        Ok(())
    }
}
