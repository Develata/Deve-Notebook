use crate::ledger::manager::types::RepoInfo;
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) fn validate_local_repo_info(
    stem: &str,
    expected_name: &str,
    info: Option<RepoInfo>,
    seen: &mut HashMap<uuid::Uuid, String>,
    seen_urls: &mut HashMap<String, String>,
) -> Result<()> {
    let info = info.ok_or_else(|| {
        anyhow!(
            "Broken local repo {} while validating catalog: repository metadata missing",
            stem
        )
    })?;
    if info.name != expected_name {
        return Err(anyhow!(
            "Broken local repo {} while validating catalog: metadata name drifted to {}",
            stem,
            info.name
        ));
    }
    if let Some(owner) = seen.insert(info.uuid, stem.to_string())
        && owner != stem
    {
        return Err(anyhow!(
            "Broken local repo {} while validating catalog: duplicate local repository UUID {} also used by {}",
            stem,
            info.uuid,
            owner
        ));
    }
    let url = info.url.ok_or_else(|| {
        anyhow!(
            "Broken local repo {} while validating catalog: repository URL missing",
            stem
        )
    })?;
    if let Some(owner) = seen_urls.insert(url.clone(), stem.to_string())
        && owner != stem
    {
        return Err(anyhow!(
            "Broken local repo {} while validating catalog: duplicate local repository URL {} also used by {}",
            stem,
            url,
            owner
        ));
    }
    Ok(())
}

pub(super) fn repair_workspace_root(
    vault_root: Option<&Path>,
    previous_name: &str,
    current_name: &str,
) -> Result<()> {
    let Some(vault_root) = vault_root else {
        return Ok(());
    };
    if previous_name == current_name || previous_name.trim().is_empty() {
        return Ok(());
    }
    let old_root = repo_root(vault_root, previous_name);
    let new_root = repo_root(vault_root, current_name);
    let old_exists = old_root.try_exists().with_context(|| {
        format!(
            "Failed to stat previous workspace root while repairing local catalog: {old_root:?}"
        )
    })?;
    let new_exists = new_root.try_exists().with_context(|| {
        format!("Failed to stat current workspace root while repairing local catalog: {new_root:?}")
    })?;
    if old_exists && new_exists {
        return Err(anyhow!(
            "Broken local repo {} while repairing local catalog: current workspace root {:?} already exists",
            current_name,
            new_root
        ));
    }
    if !old_exists {
        return Ok(());
    }
    std::fs::rename(&old_root, &new_root)?;
    tracing::warn!(
        "Realigned local workspace root: {} -> {}",
        previous_name,
        current_name
    );
    Ok(())
}

fn repo_root(vault_root: &Path, repo_name: &str) -> PathBuf {
    vault_root.join(repo_name.trim_end_matches(".redb"))
}
