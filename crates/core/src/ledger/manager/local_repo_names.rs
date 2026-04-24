//! plan_ref:
//!   - 06_repository#repo-catalog-contract

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::RepoManager;
use anyhow::{Result, anyhow};

impl RepoManager {
    /// Invariants:
    /// - 返回值始终是可执行的本地 repo 文件 stem，而不是显示别名。
    /// - 返回前必须先修复本地 repo catalog，避免 name drift 污染执行路径。
    pub fn list_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        self.refresh_local_repo_catalog()?;
        let local_dir =
            RepoManager::checked_local_dir_for(&self.ledger_dir, "listing execution names")?;

        let mut repos = Vec::new();
        for (path, stem) in redb_repo_entries(&local_dir, "listing execution names")? {
            if stem == self.local_repo_name {
                self.get_repo_info()?.ok_or_else(|| {
                    anyhow!(
                        "Broken local repo {} while listing execution names: repository metadata missing",
                        stem
                    )
                })?;
                repos.push(stem);
                continue;
            }
            RepoManager::read_required_repo_info_from_path(&path, &stem, "listing execution names")
                .map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while listing execution names: {}",
                        stem,
                        err
                    )
                })?;
            repos.push(stem);
        }
        repos.sort();
        Ok(repos)
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::{REPO_METADATA, RepoInfo, RepoManager};
    use tempfile::TempDir;

    fn create_secondary_repo(ledger_dir: &std::path::Path, name: &str, url: &str) {
        let _repo =
            RepoManager::init(ledger_dir, 8, Some(name), Some(url)).expect("secondary repo");
    }

    fn write_info(db: &redb::Database, info: &RepoInfo) {
        let txn = db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA)
            .expect("repo metadata")
            .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
            .expect("write metadata");
        txn.commit().expect("commit");
    }

    #[test]
    fn execution_repo_names_fail_closed_on_duplicate_main_metadata_drift() {
        let _guard = crate::test_support::local_repo_catalog_test_guard();
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        create_secondary_repo(&ledger_dir, "wiki", "urn:wiki");
        let main_info = main.get_repo_info().expect("main info").expect("present");
        let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
        write_info(
            wiki_db.as_ref(),
            &RepoInfo {
                uuid: main_info.uuid,
                name: "main".into(),
                url: Some(format!("urn:uuid:{}", main_info.uuid)),
            },
        );

        let err = main
            .list_local_repo_names_for_execution()
            .expect_err("duplicate main metadata drift must fail closed");
        assert!(err.to_string().contains("metadata name drifted to main"));
    }

    #[test]
    fn execution_repo_names_fail_closed_on_missing_main_metadata() {
        let _guard = crate::test_support::local_repo_catalog_test_guard();
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        let main_db = main.open_database(None, "main").expect("main db").db;
        let txn = main_db.begin_write().expect("write txn");
        txn.delete_table(REPO_METADATA)
            .expect("delete repo metadata");
        txn.commit().expect("commit");

        let err = main
            .list_local_repo_names_for_execution()
            .expect_err("missing main metadata must fail closed");
        assert!(err.to_string().contains("repository metadata missing"));
    }
}
