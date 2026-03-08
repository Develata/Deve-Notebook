use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn quarantine_nil_shadow_repos(remotes_dir: &Path) -> Result<usize> {
    if !remotes_dir.exists() {
        return Ok(0);
    }
    let nil_name = format!("{}.redb", uuid::Uuid::nil());
    let quarantine_root = remotes_dir.join(".invalid");
    let mut moved = 0usize;
    for entry in std::fs::read_dir(remotes_dir)? {
        let entry = entry?;
        let peer_dir = entry.path();
        if !peer_dir.is_dir() || peer_dir.file_name().and_then(|s| s.to_str()) == Some(".invalid") {
            continue;
        }
        let invalid = peer_dir.join(&nil_name);
        if !invalid.exists() {
            continue;
        }
        let peer_name = peer_dir
            .file_name()
            .and_then(|s| s.to_str())
            .context("invalid peer dir name")?;
        let dst_dir = quarantine_root.join(peer_name);
        std::fs::create_dir_all(&dst_dir)?;
        let dst = dst_dir.join(&nil_name);
        std::fs::rename(&invalid, &dst)?;
        println!(
            "repair: quarantined invalid shadow {:?} -> {:?}",
            invalid, dst
        );
        moved += 1;
    }
    Ok(moved)
}
