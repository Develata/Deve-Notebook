//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-catalog-repair-contract

use crate::ledger::database::cached_or_create_database;
use crate::ledger::manager::projection_locator::{
    locator_authorizes_repo_name, safe_repo_path_segment,
};
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use anyhow::{Context, Result, anyhow};
use redb::Database;
use std::collections::HashMap;
use std::path::Path;

use crate::ledger::manager::local_repo_metadata_repair_support::{
    ensure_local_repo_metadata_name_authorized, preflight_workspace_root_repair,
    prepare_workspace_root_repair, repair_workspace_root, validate_local_repo_info,
};

pub(super) fn validate_local_repo_metadata(
    ledger_dir: &Path,
    main_repo_name: &str,
    main_db: &Database,
) -> Result<()> {
    let local_dir = RepoManager::checked_local_dir_for(ledger_dir, "validating local catalog")?;

    let mut seen = HashMap::new();
    let mut seen_urls = HashMap::new();
    let mut seen_names = HashMap::new();
    let main_info = RepoManager::read_repo_info_from_db(main_db)?;
    if let Some(info) = main_info.as_ref() {
        ensure_local_repo_metadata_name_authorized(ledger_dir, main_repo_name, info)?;
    }
    validate_local_repo_info(
        main_repo_name,
        main_repo_name,
        main_info,
        &mut seen,
        &mut seen_urls,
        &mut seen_names,
    )?;

    let mut entries = redb_repo_entries(&local_dir, "validating local catalog")?;
    entries.sort_by(|(_, left_stem), (_, right_stem)| left_stem.cmp(right_stem));

    for (path, stem) in entries {
        if stem == main_repo_name {
            continue;
        }
        let db = cached_or_create_database(&path).map_err(|err| {
            anyhow!(
                "Broken local repo {} while validating catalog: {}",
                stem,
                err
            )
        })?;
        let info = RepoManager::read_repo_info_from_db(db.as_ref()).map_err(|err| {
            anyhow!(
                "Broken local repo {} while validating metadata: {}",
                stem,
                err
            )
        })?;
        let report_duplicate_name_first = info.as_ref().is_some_and(|info| {
            should_report_duplicate_display_name_first(&stem, info, &seen, &seen_urls, &seen_names)
        });
        if let Some(info) = info.as_ref()
            && !report_duplicate_name_first
        {
            ensure_local_repo_metadata_name_authorized(ledger_dir, &stem, info)?;
        }
        validate_local_repo_info(
            &stem,
            &stem,
            info,
            &mut seen,
            &mut seen_urls,
            &mut seen_names,
        )?;
    }
    Ok(())
}

fn should_report_duplicate_display_name_first(
    stem: &str,
    info: &RepoInfo,
    seen: &HashMap<uuid::Uuid, String>,
    seen_urls: &HashMap<String, String>,
    seen_names: &HashMap<String, String>,
) -> bool {
    if info.name == stem || !seen_names.contains_key(&info.name) {
        return false;
    }
    let url_conflict = info
        .url
        .as_ref()
        .is_some_and(|url| seen_urls.contains_key(url));
    !seen.contains_key(&info.uuid) && !url_conflict
}

pub(crate) fn repair_local_repo_metadata(
    ledger_dir: &Path,
    main_repo_name: &str,
    main_db: &Database,
    allow_workspace_root_rewrite: bool,
    repair_manager: Option<&RepoManager>,
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
    let mut seen_names = HashMap::new();
    for (path, stem) in entries {
        let db = if stem == main_repo_name {
            None
        } else {
            match cached_or_create_database(&path) {
                Ok(db) => Some(db),
                Err(err) => {
                    return Err(anyhow!(
                        "Broken local repo {} while repairing catalog: {}",
                        stem,
                        err
                    ));
                }
            }
        };
        let db = db.as_deref().unwrap_or(main_db);
        let read_info = RepoManager::read_repo_info_from_db(db);
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
        .unwrap_or_else(|| RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: stem.clone(),
            url: None,
        });
        let original = info.clone();
        let previous_name = info.name.clone();
        let display_name_owner = seen_names.get(&info.name).cloned();
        let unauthorized_name_drift = info.name != stem
            && (safe_repo_path_segment(&info.name).is_err()
                || !locator_authorizes_repo_name(ledger_dir, info.uuid, &info.name)?);
        if info.name.trim().is_empty()
            || display_name_owner.is_some_and(|owner| owner != stem)
            || unauthorized_name_drift
        {
            info.name = stem.clone();
        }
        seen_names.insert(info.name.clone(), stem.clone());
        if seen.insert(info.uuid, stem.clone()).is_some() {
            let old_uuid = info.uuid;
            info.uuid = uuid::Uuid::new_v4();
            let old_urn = format!("urn:uuid:{old_uuid}");
            if info.url.as_deref().is_none() || info.url.as_deref() == Some(old_urn.as_str()) {
                info.url = Some(format!("urn:uuid:{}", info.uuid));
            }
        }
        if info.url.is_none() {
            info.url = Some(format!("urn:uuid:{}", info.uuid));
        }
        if let Some(url) = info.url.clone()
            && let Some(existing_owner) = seen_urls.insert(url.clone(), stem.clone())
            && existing_owner != stem
        {
            tracing::warn!(
                "Repairing duplicate local repo URL: {} conflicts with {} on {}",
                stem,
                existing_owner,
                url
            );
            info.url = Some(format!("urn:uuid:{}", info.uuid));
        }
        let workspace_repair = if allow_workspace_root_rewrite {
            prepare_workspace_root_repair(ledger_dir, &stem, info.uuid, &previous_name, &info.name)?
        } else {
            None
        };
        if let Some(plan) = workspace_repair.as_ref() {
            let manager = repair_manager.ok_or_else(|| {
                anyhow!(
                    "Workspace root realign for {} refused: repair preflight manager missing",
                    stem
                )
            })?;
            preflight_workspace_root_repair(manager, plan)?;
        }
        if info != original {
            RepoManager::write_repo_info_to_db(db, &info)?;
            tracing::warn!("Repaired local repo metadata: {} -> {}", stem, info.uuid);
        }
        if let Some(plan) = workspace_repair {
            repair_workspace_root(plan)?;
        }
    }
    Ok(())
}
