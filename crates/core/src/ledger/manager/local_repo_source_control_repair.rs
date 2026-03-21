use crate::ledger::database::cached_or_create_database;
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
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
        source_control::validate_tables(main_db).map_err(|err| {
            anyhow!(
                "Broken local repo {} while validating source control tables: {}",
                main_repo_name,
                err
            )
        })?;
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
        source_control::init_tables(main_db).map_err(|err| {
            anyhow!(
                "Broken local repo {} while repairing source control tables: {}",
                main_repo_name,
                err
            )
        })?;
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
    let local_dir = RepoManager::checked_local_dir_for(ledger_dir, action)?;
    let mut entries = redb_repo_entries(&local_dir, action)?;
    entries.sort_by(|(_, left), (_, right)| left.cmp(right));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::RepoManager;
    use crate::ledger::source_control;
    use crate::source_control::staging;
    use tempfile::tempdir;

    #[test]
    fn validates_and_repairs_main_repo_tables_even_without_main_catalog_entry() -> anyhow::Result<()>
    {
        let repo_dir = tempdir()?;
        let repo = RepoManager::init(repo_dir.path(), 8, Some("main"), Some("urn:main"))?;

        let write = repo.local_db.begin_write()?;
        let _ = write.delete_table(staging::STAGED_TABLE)?;
        write.commit()?;

        let shadow_catalog = tempdir()?;
        std::fs::create_dir_all(shadow_catalog.path().join("local"))?;

        let err = RepoManager::validate_local_repo_source_control_tables(
            shadow_catalog.path(),
            &repo.local_repo_name,
            repo.local_db.as_ref(),
        )
        .expect_err("main repo validation must fail closed");
        assert!(err.to_string().contains("main"));
        assert!(err.to_string().contains("source control tables"));

        RepoManager::repair_local_repo_source_control_tables(
            shadow_catalog.path(),
            &repo.local_repo_name,
            repo.local_db.as_ref(),
        )?;
        source_control::validate_tables(repo.local_db.as_ref())?;
        Ok(())
    }
}
