use super::{IdentityKeyPair, RepoKey};
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

fn write_key_file(path: &Path, data: &[u8]) -> Result<()> {
    std::fs::write(path, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn load_or_generate_identity_key_at(dir: &Path) -> Result<Arc<IdentityKeyPair>> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("identity.key");
    if path.exists() {
        let bytes = std::fs::read(&path)?;
        if let Some(kp) = IdentityKeyPair::from_bytes(&bytes) {
            return Ok(Arc::new(kp));
        }
    }
    let kp = IdentityKeyPair::generate();
    write_key_file(&path, &kp.to_bytes())?;
    Ok(Arc::new(kp))
}

pub fn load_or_generate_repo_key_at(dir: &Path) -> Result<RepoKey> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("repo.key");
    if path.exists() {
        let bytes = std::fs::read(&path)?;
        if let Some(key) = RepoKey::from_bytes(&bytes) {
            return Ok(key);
        }
    }
    let key = RepoKey::generate();
    write_key_file(&path, &key.to_bytes())?;
    Ok(key)
}
