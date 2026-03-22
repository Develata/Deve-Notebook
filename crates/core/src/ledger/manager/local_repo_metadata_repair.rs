use crate::ledger::database::cached_or_create_database;
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use anyhow::{Context, Result, anyhow};
use redb::Database;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl RepoManager {
    pub(crate) fn validate_local_repo_metadata(
        ledger_dir: &Path,
        main_repo_name: &str,
        main_db: &Database,
    ) -> Result<()> {
        let local_dir = RepoManager::checked_local_dir_for(ledger_dir, "validating local catalog")?;

        let mut seen = HashMap::new();
        let mut seen_urls = HashMap::new();
        validate_local_repo_info(
            main_repo_name,
            main_repo_name,
            Self::read_repo_info_from_db(main_db)?,
            &mut seen,
            &mut seen_urls,
        )?;

        let mut entries = redb_repo_entries(&local_dir, "validating local catalog")?;
        entries.sort_by(|(_, left_stem), (_, right_stem)| left_stem.cmp(right_stem));

        for (path, stem) in entries {
            let db = cached_or_create_database(&path).map_err(|err| {
                anyhow!(
                    "Broken local repo {} while validating catalog: {}",
                    stem,
                    err
                )
            })?;
            let info = Self::read_repo_info_from_db(db.as_ref()).map_err(|err| {
                anyhow!(
                    "Broken local repo {} while validating metadata: {}",
                    stem,
                    err
                )
            })?;
            validate_local_repo_info(&stem, &stem, info, &mut seen, &mut seen_urls)?;
        }
        Ok(())
    }

    pub(crate) fn repair_local_repo_metadata(
        ledger_dir: &Path,
        main_repo_name: &str,
        main_db: &Database,
        vault_root: Option<&Path>,
        allow_workspace_root_rewrite: bool,
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
            let read_info = Self::read_repo_info_from_db(db);
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
            if info.name != stem {
                info.name = stem.clone();
            }
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
            if info != original {
                Self::write_repo_info_to_db(db, &info)?;
                if allow_workspace_root_rewrite {
                    repair_workspace_root(vault_root, &previous_name, &stem)?;
                }
                tracing::warn!("Repaired local repo metadata: {} -> {}", stem, info.uuid);
            }
        }
        Ok(())
    }
}

fn validate_local_repo_info(
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

fn repair_workspace_root(
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
    if !old_exists || new_exists {
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
