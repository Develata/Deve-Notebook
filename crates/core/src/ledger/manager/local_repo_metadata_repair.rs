//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-catalog-repair-contract

use crate::ledger::manager::authority_storage_runtime::LocalAuthorityRuntime;
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::Path;

use crate::ledger::manager::local_repo_metadata_repair_support::{
    ensure_local_repo_metadata_identity, validate_local_repo_info,
};

pub(super) fn validate_local_repo_metadata(
    main_repo_name: &str,
    authority: &LocalAuthorityRuntime,
    normal_repo_ids: &[RepoId],
) -> Result<()> {
    let mut seen = HashMap::new();
    let mut seen_urls = HashMap::new();
    let mut stems = normal_repo_ids
        .iter()
        .map(|repo_id| repo_id.to_string())
        .collect::<Vec<_>>();
    stems.sort_by_key(|stem| usize::from(stem != main_repo_name));

    for stem in stems {
        let info = authority
            .inspect_existing_stem(&stem, RepoManager::read_local_repo_info_from_db)
            .map_err(|err| {
                anyhow!(
                    "Broken local repo {} while validating catalog: {}",
                    stem,
                    err
                )
            })?;
        if let Some(info) = info.as_ref() {
            ensure_local_repo_metadata_identity(&stem, info)?;
        }
        validate_local_repo_info(&stem, info, &mut seen, &mut seen_urls)?;
    }
    Ok(())
}

pub(crate) fn repair_local_repo_metadata(
    ledger_dir: &Path,
    main_repo_name: &str,
    authority: &LocalAuthorityRuntime,
) -> Result<()> {
    let local_dir = ledger_dir.join("local");
    match local_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "Failed to stat local repo directory while repairing local catalog: {:?}",
                    local_dir
                )
            });
        }
    }
    if !std::fs::metadata(&local_dir)
        .with_context(|| {
            format!(
                "Failed to read local repo directory metadata while repairing local catalog: {:?}",
                local_dir
            )
        })?
        .is_dir()
    {
        return Err(anyhow!(
            "Broken local repo catalog: expected directory at {:?}",
            local_dir
        ));
    }

    let mut entries = redb_repo_entries(&local_dir, "repairing local catalog")?;
    entries.sort_by(|(left_path, left_stem), (right_path, right_stem)| {
        usize::from(left_stem != main_repo_name)
            .cmp(&usize::from(right_stem != main_repo_name))
            .then_with(|| left_path.cmp(right_path))
    });

    let mut seen = HashMap::new();
    let mut seen_urls = HashMap::new();
    for (_path, stem) in entries {
        authority.inspect_existing_stem(&stem, |db| {
            let read_info = RepoManager::read_local_repo_info_from_db(db);
            let mut info = match read_info {
                Ok(info) => info,
                Err(err) if stem != main_repo_name => {
                    return Err(anyhow!(
                        "Broken local repo {} while reading metadata during repair: {}",
                        stem,
                        err
                    ));
                }
                Err(err) => return Err(err),
            }
            .ok_or_else(|| {
                anyhow!(
                    "Broken local repo {} while repairing catalog: repository metadata missing",
                    stem
                )
            })?;
            let original = info.clone();
            ensure_local_repo_metadata_identity(&stem, &info)?;
            if let Some(existing_owner) = seen.insert(info.uuid, stem.clone())
                && existing_owner != stem
            {
                return Err(anyhow!(
                    "Broken local repo {} while repairing catalog: duplicate local repository UUID {} also used by {}",
                    stem,
                    info.uuid,
                    existing_owner
                ));
            }
            if info.url.is_none() {
                info.url = Some(format!("urn:uuid:{}", info.uuid));
            }
            if let Some(url) = info.url.clone()
                && let Some(existing_owner) = seen_urls.insert(url.clone(), stem.clone())
                && existing_owner != stem
            {
                return Err(anyhow!(
                    "Broken local repo {} while repairing catalog: duplicate local repository URL {} also used by {}",
                    stem,
                    url,
                    existing_owner
                ));
            }
            if info != original {
                RepoManager::write_local_repo_info_to_db(db, &info)?;
                tracing::warn!("Repaired local repo metadata: {} -> {}", stem, info.uuid);
            }
            Ok(())
        })?;
    }
    Ok(())
}
