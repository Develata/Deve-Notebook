use crate::ledger::database::cached_or_create_database;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use anyhow::Result;
use redb::Database;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl RepoManager {
    pub(crate) fn repair_local_repo_metadata(
        ledger_dir: &Path,
        main_repo_name: &str,
        main_db: &Database,
        vault_root: Option<&Path>,
    ) -> Result<()> {
        let local_dir = ledger_dir.join("local");
        if !local_dir.exists() {
            return Ok(());
        }

        let mut entries = std::fs::read_dir(&local_dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("redb"))
            .collect::<Vec<_>>();
        entries.sort();
        entries.sort_by_key(|path| {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            usize::from(stem != main_repo_name)
        });

        let mut seen = HashMap::new();
        let mut seen_urls = HashMap::new();
        for path in entries {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let db = if stem == main_repo_name {
                None
            } else {
                Some(cached_or_create_database(&path)?)
            };
            let db = db.as_deref().unwrap_or(main_db);
            let mut info = Self::read_repo_info_from_db(db)?.unwrap_or_else(|| RepoInfo {
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
                repair_workspace_root(vault_root, &previous_name, &stem)?;
                tracing::warn!("Repaired local repo metadata: {} -> {}", stem, info.uuid);
            }
        }
        Ok(())
    }
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
    if !old_root.exists() || new_root.exists() {
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
