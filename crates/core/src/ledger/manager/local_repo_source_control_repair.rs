use crate::ledger::database::cached_or_create_database;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::source_control;
use anyhow::{Result, anyhow};
use redb::Database;
use std::path::{Path, PathBuf};

impl RepoManager {
    pub(crate) fn validate_local_repo_source_control_tables(
        ledger_dir: &Path,
        main_repo_name: &str,
        main_db: &Database,
    ) -> Result<()> {
        for (path, stem) in local_repo_paths(ledger_dir, "validating source control tables")? {
            let db = if stem == main_repo_name {
                None
            } else {
                Some(cached_or_create_database(&path).map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while validating source control tables: {}",
                        stem,
                        err
                    )
                })?)
            };
            source_control::validate_tables(db.as_deref().unwrap_or(main_db)).map_err(|err| {
                anyhow!(
                    "Broken local repo {} while validating source control tables: {}",
                    stem,
                    err
                )
            })?;
        }
        Ok(())
    }

    pub(crate) fn repair_local_repo_source_control_tables(
        ledger_dir: &Path,
        main_repo_name: &str,
        main_db: &Database,
    ) -> Result<()> {
        for (path, stem) in local_repo_paths(ledger_dir, "repairing source control tables")? {
            let db = if stem == main_repo_name {
                None
            } else {
                Some(cached_or_create_database(&path).map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while repairing source control tables: {}",
                        stem,
                        err
                    )
                })?)
            };
            source_control::init_tables(db.as_deref().unwrap_or(main_db)).map_err(|err| {
                anyhow!(
                    "Broken local repo {} while repairing source control tables: {}",
                    stem,
                    err
                )
            })?;
        }
        Ok(())
    }
}

fn local_repo_paths(ledger_dir: &Path, action: &str) -> Result<Vec<(PathBuf, String)>> {
    let local_dir = ledger_dir.join("local");
    if !local_dir.exists() {
        return Err(anyhow!(
            "Broken local repo catalog: local repo directory missing at {:?}",
            local_dir
        ));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&local_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("redb") {
            let stem = RepoManager::repo_stem_from_path(&path, action)?;
            entries.push((path, stem));
        }
    }
    entries.sort_by(|(_, left), (_, right)| left.cmp(right));
    Ok(entries)
}
