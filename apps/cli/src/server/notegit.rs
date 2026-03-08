use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn prepare(vault_root: &Path) -> Result<PathBuf> {
    let notegit_dir = deve_core::utils::notegit::dir(vault_root);
    std::fs::create_dir_all(deve_core::utils::notegit::keys_dir(vault_root))?;
    migrate_legacy_deve(vault_root, &notegit_dir)?;
    Ok(notegit_dir)
}

fn migrate_legacy_deve(vault_root: &Path, notegit_dir: &Path) -> Result<()> {
    let legacy_dir = vault_root.join(".deve");
    if !legacy_dir.exists() {
        return Ok(());
    }

    let conflict_dir = notegit_dir.join("legacy-deve-conflicts");
    std::fs::create_dir_all(&conflict_dir)?;
    for (name, dst) in [
        (
            "identity.key",
            notegit_dir.join("keys").join("identity.key"),
        ),
        ("repo.key", notegit_dir.join("keys").join("repo.key")),
        ("mcp.json", notegit_dir.join("mcp.json")),
        (
            "legacy-flat",
            deve_core::utils::notegit::legacy_flat_dir(vault_root),
        ),
        (
            "legacy-flat-conflicts",
            deve_core::utils::notegit::legacy_flat_conflicts_dir(vault_root),
        ),
        ("active_repo", notegit_dir.join("active_repo")),
        ("peer_id", notegit_dir.join("peer_id")),
    ] {
        let src = legacy_dir.join(name);
        if src.exists() {
            merge_entry(&src, &dst, &conflict_dir)?;
        }
    }

    if legacy_dir.exists() {
        for entry in std::fs::read_dir(&legacy_dir)? {
            let entry = entry?;
            let src = entry.path();
            let dst = notegit_dir.join(entry.file_name());
            merge_entry(&src, &dst, &conflict_dir)?;
        }
        let _ = std::fs::remove_dir(&legacy_dir);
    }
    Ok(())
}

fn merge_entry(src: &Path, dst: &Path, conflict_dir: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if !dst.exists() {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(src, dst)?;
        return Ok(());
    }
    if src.is_dir() && dst.is_dir() {
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            merge_entry(&entry.path(), &dst.join(entry.file_name()), conflict_dir)?;
        }
        let _ = std::fs::remove_dir(src);
        return Ok(());
    }
    if src.is_file() && dst.is_file() && std::fs::read(src)? == std::fs::read(dst)? {
        std::fs::remove_file(src)?;
        return Ok(());
    }

    let name = src.file_name().and_then(|v| v.to_str()).unwrap_or("legacy");
    let archived = unique_conflict_path(conflict_dir, name);
    std::fs::rename(src, &archived)?;
    tracing::warn!("Legacy .deve entry archived {:?} -> {:?}", src, archived);
    Ok(())
}

fn unique_conflict_path(root: &Path, name: &str) -> PathBuf {
    let first = root.join(name);
    if !first.exists() {
        return first;
    }
    for idx in 1.. {
        let candidate = root.join(format!("{name}-{idx}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("legacy conflict suffix search is unbounded");
}
