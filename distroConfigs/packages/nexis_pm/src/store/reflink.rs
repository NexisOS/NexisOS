use reflink_copy::reflink;
use anyhow::Result;

pub fn reflink_copy(src: &Path, dst: &Path) -> Result<()> {
    match reflink(src, dst) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::Unsupported => {
            // Fallback to regular copy if reflink not supported
            std::fs::copy(src, dst)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
