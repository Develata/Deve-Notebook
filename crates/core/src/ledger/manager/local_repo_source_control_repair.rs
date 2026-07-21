//! plan_ref:
//!   - 04_repository#repo-catalog-repair-contract
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/index#repo-runtime-layout
//!
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::source_control;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

impl RepoManager {
    pub(crate) fn validate_local_repo_source_control_tables(&self) -> Result<()> {
        for repo_id in self.normal_repo_catalog_ids()? {
            let stem = repo_id.to_string();
            self.run_on_local_repo_stem(&stem, |db| {
                Self::validate_local_repo_schema(db).map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while validating local-authority schema: {}",
                        stem,
                        err
                    )
                })?;
                source_control::validate_tables(db).map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while validating source control tables: {}",
                        stem,
                        err
                    )
                })
            })?;
        }
        Ok(())
    }

    pub(crate) fn repair_local_repo_source_control_tables(&self) -> Result<()> {
        for (_path, stem) in local_repo_paths(&self.ledger_dir, "repairing source control tables")?
        {
            self.local_authority.inspect_existing_stem(&stem, |db| {
                Self::validate_local_repo_schema(db).map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while validating schema before source control repair: {}",
                        stem,
                        err
                    )
                })?;
                source_control::init_tables(db).map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while repairing source control tables: {}",
                        stem,
                        err
                    )
                })
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
    use crate::codec;
    use crate::ledger::schema::{
        REMOTE_IMPORT_RUNTIME, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    };
    use crate::ledger::source_control;
    use crate::source_control::staging;
    use tempfile::tempdir;

    #[test]
    fn validates_and_repairs_cataloged_main_repo_tables() -> anyhow::Result<()> {
        let repo_dir = tempdir()?;
        let ledger = repo_dir.path().join("ledger");
        let (repo, repo_id) =
            crate::test_support::init_cataloged_repo(&ledger, &repo_dir.path().join("notes"))?;
        let lease = repo.local_authority_lease_for_test(repo_id)?;

        let write = lease.db().begin_write()?;
        let _ = write.delete_table(staging::STAGED_TABLE)?;
        write.commit()?;

        let err = repo
            .validate_local_repo_source_control_tables()
            .expect_err("main repo validation must fail closed");
        assert!(err.to_string().contains(repo.local_repo_name()));
        assert!(err.to_string().contains("source control tables"));

        repo.repair_local_repo_source_control_tables()?;
        source_control::validate_tables(lease.db())?;
        Ok(())
    }

    #[test]
    fn repair_rejects_old_schema_before_recreating_source_control_tables() -> anyhow::Result<()> {
        let repo_dir = tempdir()?;
        let ledger = repo_dir.path().join("ledger");
        let (repo, repo_id) =
            crate::test_support::init_cataloged_repo(&ledger, &repo_dir.path().join("notes"))?;
        let lease = repo.local_authority_lease_for_test(repo_id)?;
        let write = lease.db().begin_write()?;
        {
            let _ = write.delete_table(staging::STAGED_TABLE)?;
            let mut metadata = write.open_table(REPO_METADATA)?;
            let version = codec::encode(&3u16)?;
            metadata.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        }
        write.commit()?;

        let err = repo
            .repair_local_repo_source_control_tables()
            .expect_err("v3 authority must fail before source-control repair writes");
        assert!(err.to_string().contains("expected 4"));
        let read = lease.db().begin_read()?;
        assert!(matches!(
            read.open_table(staging::STAGED_TABLE),
            Err(redb::TableError::TableDoesNotExist(_))
        ));
        Ok(())
    }

    #[test]
    fn repair_rejects_incomplete_v4_profile_before_any_source_control_write() -> anyhow::Result<()>
    {
        let repo_dir = tempdir()?;
        let ledger = repo_dir.path().join("ledger");
        let (repo, repo_id) =
            crate::test_support::init_cataloged_repo(&ledger, &repo_dir.path().join("notes"))?;
        let lease = repo.local_authority_lease_for_test(repo_id)?;
        let write = lease.db().begin_write()?;
        let _ = write.delete_table(staging::STAGED_TABLE)?;
        let _ = write.delete_table(REMOTE_IMPORT_RUNTIME)?;
        write.commit()?;

        let err = repo
            .repair_local_repo_source_control_tables()
            .expect_err("incomplete v4 authority must fail before source-control repair writes");
        assert!(err.to_string().contains("remote_import_runtime"), "{err:#}");

        let read = lease.db().begin_read()?;
        assert!(matches!(
            read.open_table(staging::STAGED_TABLE),
            Err(redb::TableError::TableDoesNotExist(_))
        ));
        assert!(matches!(
            read.open_table(REMOTE_IMPORT_RUNTIME),
            Err(redb::TableError::TableDoesNotExist(_))
        ));
        Ok(())
    }
}
