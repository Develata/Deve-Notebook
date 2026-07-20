//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-catalog-repair-contract

use crate::ledger::manager::types::RepoInfo;
use anyhow::{Result, anyhow};
use std::collections::HashMap;

pub(crate) fn ensure_local_repo_metadata_identity(stem: &str, info: &RepoInfo) -> Result<()> {
    let physical_repo_id = uuid::Uuid::parse_str(stem).map_err(|_| {
        anyhow!(
            "Broken local repo {}: physical database stem must be a RepoId",
            stem
        )
    })?;
    if physical_repo_id != info.uuid {
        return Err(anyhow!(
            "Broken local repo {}: physical RepoId does not match metadata RepoId {}",
            stem,
            info.uuid
        ));
    }
    crate::ledger::manager::projection_locator::safe_repo_path_segment(&info.name).map_err(
        |err| {
            anyhow!(
                "Broken local repo {} metadata machine name {} is invalid: {}",
                stem,
                info.name,
                err
            )
        },
    )?;
    Ok(())
}

/// Fail-closed drift gate for catalog-backed selector resolution: a cataloged
/// repo whose metadata name left the canonical RepoId stem must not resolve
/// through display-name matching. Repair/validate paths stay tolerant — they
/// must still be able to read drifted metadata to classify it.
pub(crate) fn ensure_cataloged_repo_name_canonical(stem: &str, info: &RepoInfo) -> Result<()> {
    if info.name != stem {
        return Err(anyhow!(
            "Broken local repo {}: metadata name drifted to {}",
            stem,
            info.name
        ));
    }
    Ok(())
}

pub(super) fn validate_local_repo_info(
    stem: &str,
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
