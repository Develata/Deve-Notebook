//! plan_ref:
//!   - 04_repository#repo-catalog-contract

use crate::ledger::manager::types::RepoManager;
use anyhow::Result;

impl RepoManager {
    /// Invariants:
    /// - 返回值始终是可执行的本地 repo 文件 stem，而不是显示别名。
    /// - 返回前必须先修复本地 repo catalog，避免 name drift 污染执行路径。
    pub fn list_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        self.repo_catalog_runtime().list_local_execution_names()
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::RepoInfo;
    use tempfile::TempDir;

    #[test]
    fn execution_repo_names_fail_closed_on_physical_repo_id_drift() {
        let _guard = crate::test_support::local_repo_catalog_test_guard();
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let (main, main_id) =
            crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("main-notes"))
                .expect("main");
        let (_wiki, wiki_id) =
            crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("wiki-notes"))
                .expect("wiki");
        // Drift the secondary's physical RepoId so its stem no longer matches
        // its metadata RepoId.
        let wiki_db = main
            .open_database(None, &wiki_id.to_string())
            .expect("wiki db")
            .db;
        crate::test_support::write_repo_metadata(
            wiki_db.as_ref(),
            &RepoInfo {
                uuid: main_id,
                name: main_id.to_string(),
                url: Some(format!("urn:uuid:{}", main_id)),
            },
        )
        .expect("write metadata");

        let err = main
            .list_local_repo_names_for_execution()
            .expect_err("duplicate main metadata drift must fail closed");
        assert!(
            err.to_string()
                .contains("physical RepoId does not match metadata RepoId")
        );
    }

    #[test]
    fn execution_repo_names_fail_closed_on_missing_main_metadata() {
        let _guard = crate::test_support::local_repo_catalog_test_guard();
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let (main, main_id) =
            crate::test_support::init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))
                .expect("main");
        let main_db = main
            .open_database(None, &main_id.to_string())
            .expect("main db")
            .db;
        crate::test_support::delete_repo_metadata(main_db.as_ref()).expect("delete metadata");

        let err = main
            .list_local_repo_names_for_execution()
            .expect_err("missing main metadata must fail closed");
        assert!(err.to_string().contains("repository metadata missing"));
    }
}
