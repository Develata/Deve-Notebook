use super::{IdentityKeyPair, RepoKey};
use anyhow::{Result, anyhow};
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
    if let Some(kp) = read_identity_key(&path)? {
        return Ok(Arc::new(kp));
    }
    let kp = IdentityKeyPair::generate();
    write_key_file(&path, &kp.to_bytes())?;
    Ok(Arc::new(kp))
}

pub fn load_or_generate_repo_key_at(dir: &Path) -> Result<RepoKey> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("repo.key");
    if let Some(key) = read_repo_key(&path)? {
        return Ok(key);
    }
    let key = RepoKey::generate();
    write_key_file(&path, &key.to_bytes())?;
    Ok(key)
}

fn read_identity_key(path: &Path) -> Result<Option<IdentityKeyPair>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    IdentityKeyPair::from_bytes(&bytes)
        .map(Some)
        .ok_or_else(|| anyhow!("Corrupt identity key at {:?}", path))
}

fn read_repo_key(path: &Path) -> Result<Option<RepoKey>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    RepoKey::from_bytes(&bytes)
        .map(Some)
        .ok_or_else(|| anyhow!("Corrupt repo key at {:?}", path))
}

#[cfg(test)]
#[path = "storage_test.rs"]
mod tests;
