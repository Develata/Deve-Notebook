use crate::ledger::manager::types::RepoManager;
use anyhow::Result;

impl RepoManager {
    /// Invariants:
    /// - 返回值始终是可执行的本地 repo 文件 stem，而不是显示别名。
    /// - 返回前必须先修复本地 repo catalog，避免 name drift 污染执行路径。
    pub fn list_local_repo_names_for_execution(&self) -> Result<Vec<String>> {
        self.repair_local_repo_catalog()?;
        let local_dir = self.ledger_dir.join("local");
        if !local_dir.exists() {
            return Ok(vec![]);
        }

        let mut repos = std::fs::read_dir(local_dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("redb"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        repos.sort();
        Ok(repos)
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::{REPO_METADATA, RepoInfo, RepoManager};
    use tempfile::TempDir;

    fn write_info(db: &redb::Database, info: &RepoInfo) {
        let txn = db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA)
            .expect("repo metadata")
            .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
            .expect("write metadata");
        txn.commit().expect("commit");
    }

    #[test]
    fn execution_repo_names_use_repaired_file_stems() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
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

        assert_eq!(
            main.list_local_repo_names_for_execution()
                .expect("execution repo names"),
            vec!["main".to_string(), "wiki".to_string()]
        );
    }
}
