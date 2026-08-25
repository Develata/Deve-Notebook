//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout

use super::{IdentityKeyPair, RepoKey};
use anyhow::{Result, anyhow};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

const MAX_KEY_BYTES: u64 = 64;

struct KeyInitLock {
    _file: std::fs::File,
}

impl KeyInitLock {
    fn acquire(dir: &Path, key_name: &str) -> Result<Self> {
        let path = dir.join(format!("{key_name}.lock"));
        let file = crate::utils::fs::open_regular_file_lock(&path, "key initialization lock")?;
        crate::utils::fs::lock_file_exclusive(&file)?;
        crate::utils::fs::ensure_open_file_matches_path(&file, &path, "key initialization lock")?;
        Ok(Self { _file: file })
    }
}

fn publish_new_key(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Key path has no containing directory: {:?}", path))?;
    let mut file = crate::utils::fs::create_owner_only_regular_file_new(path, "key file")?;
    file.write_all(data)?;
    file.sync_all()?;
    crate::utils::fs::ensure_open_file_matches_path(&file, path, "key file")?;
    crate::utils::fs::sync_directory(parent)?;
    Ok(())
}

pub fn load_or_generate_identity_key_at(dir: &Path) -> Result<Arc<IdentityKeyPair>> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("identity.key");
    let _lock = KeyInitLock::acquire(dir, "identity.key")?;
    if let Some(kp) = read_identity_key(&path)? {
        return Ok(Arc::new(kp));
    }
    let kp = IdentityKeyPair::generate();
    publish_new_key(&path, &kp.to_bytes())?;
    Ok(Arc::new(kp))
}

pub fn load_or_generate_repo_key_at(dir: &Path) -> Result<RepoKey> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("repo.key");
    let _lock = KeyInitLock::acquire(dir, "repo.key")?;
    if let Some(key) = read_repo_key(&path)? {
        return Ok(key);
    }
    let key = RepoKey::generate();
    publish_new_key(&path, &key.to_bytes())?;
    Ok(key)
}

fn read_identity_key(path: &Path) -> Result<Option<IdentityKeyPair>> {
    let bytes = match read_key_file(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    IdentityKeyPair::from_bytes(&bytes)
        .map(Some)
        .ok_or_else(|| anyhow!("Corrupt identity key at {:?}", path))
}

fn read_repo_key(path: &Path) -> Result<Option<RepoKey>> {
    let bytes = match read_key_file(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    RepoKey::from_bytes(&bytes)
        .map(Some)
        .ok_or_else(|| anyhow!("Corrupt repo key at {:?}", path))
}

fn read_key_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = crate::utils::fs::open_owner_only_regular_file_read(path, "key file")?;
    let mut bytes = Vec::with_capacity(
        usize::try_from(file.metadata()?.len().min(MAX_KEY_BYTES)).unwrap_or_default(),
    );
    Read::by_ref(&mut file)
        .take(MAX_KEY_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    crate::utils::fs::ensure_open_file_matches_path(&file, path, "key file")?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
