use crate::ledger::database::cached_or_create_database;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use anyhow::Result;
use redb::Database;
use std::collections::HashMap;
use std::path::Path;

impl RepoManager {
    pub(crate) fn repair_local_repo_metadata(
        ledger_dir: &Path,
        main_repo_name: &str,
        main_db: &Database,
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
            if info != original {
                Self::write_repo_info_to_db(db, &info)?;
                tracing::warn!("Repaired local repo metadata: {} -> {}", stem, info.uuid);
            }
        }
        Ok(())
    }
}
