use anyhow::Result;
use deve_core::ledger::RepoManager;
use std::path::{Path, PathBuf};

pub fn prepare(repo: &RepoManager, vault_root: &Path) -> Result<PathBuf> {
    let host_dir = deve_core::utils::notegit::host_dir(repo.ledger_dir());
    let host_keys_dir = deve_core::utils::notegit::host_keys_dir(repo.ledger_dir());
    let main_repo_root = repo.local_repo_workspace_root(repo.local_repo_name())?;
    std::fs::create_dir_all(&host_keys_dir)?;
    std::fs::create_dir_all(deve_core::utils::notegit::repo_keys_dir(&main_repo_root))?;
    std::fs::create_dir_all(deve_core::utils::notegit::repo_dir(&main_repo_root))?;
    migrate_legacy_root(&vault_root.join(".notegit"), &host_dir, &main_repo_root)?;
    migrate_legacy_root(&vault_root.join(".deve"), &host_dir, &main_repo_root)?;
    Ok(host_keys_dir)
}

fn migrate_legacy_root(src_root: &Path, host_dir: &Path, main_repo_root: &Path) -> Result<()> {
    if !src_root.exists() {
        return Ok(());
    }
    let conflict_dir = host_dir.join("legacy-root-conflicts");
    std::fs::create_dir_all(&conflict_dir)?;
    for entry in std::fs::read_dir(src_root)? {
        let entry = entry?;
        let src = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "keys" && src.is_dir() {
            migrate_keys_dir(&src, host_dir, main_repo_root, &conflict_dir)?;
            continue;
        }
        let Some(dst) = entry_target(&name, host_dir, main_repo_root) else {
            archive_unknown(&src, &conflict_dir)?;
            continue;
        };
        merge_entry(&src, &dst, &conflict_dir)?;
    }
    let _ = std::fs::remove_dir(src_root);
    Ok(())
}

fn migrate_keys_dir(
    src_dir: &Path,
    host_dir: &Path,
    main_repo_root: &Path,
    conflict_dir: &Path,
) -> Result<()> {
    for entry in std::fs::read_dir(src_dir)? {
        let entry = entry?;
        let src = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(dst) = entry_target(&name, host_dir, main_repo_root) else {
            archive_unknown(&src, conflict_dir)?;
            continue;
        };
        merge_entry(&src, &dst, conflict_dir)?;
    }
    let _ = std::fs::remove_dir(src_dir);
    Ok(())
}

fn entry_target(name: &str, host_dir: &Path, main_repo_root: &Path) -> Option<PathBuf> {
    match name {
        "identity.key" => {
            Some(deve_core::utils::notegit::host_keys_dir(host_dir.parent()?).join("identity.key"))
        }
        "repo.key" => {
            Some(deve_core::utils::notegit::repo_keys_dir(main_repo_root).join("repo.key"))
        }
        "mcp.json" => Some(host_dir.join("mcp.json")),
        "legacy-flat" => Some(deve_core::utils::notegit::repo_legacy_flat_dir(
            main_repo_root,
        )),
        "legacy-flat-conflicts" => Some(deve_core::utils::notegit::repo_legacy_flat_conflicts_dir(
            main_repo_root,
        )),
        "active_repo" | "peer_id" => Some(host_dir.join(name)),
        _ => None,
    }
}

fn archive_unknown(src: &Path, conflict_dir: &Path) -> Result<()> {
    let name = src.file_name().and_then(|v| v.to_str()).unwrap_or("legacy");
    let archived = unique_conflict_path(conflict_dir, name);
    std::fs::rename(src, &archived)?;
    tracing::warn!("Legacy runtime entry archived {:?} -> {:?}", src, archived);
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
    tracing::warn!("Legacy runtime entry archived {:?} -> {:?}", src, archived);
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
