use anyhow::Result;
use redb::Database;

use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;

impl RepoManager {
    /// 获取指定分支下指定仓库的 URL
    ///
    /// * `branch`: None 表示本地 (main repo or extra local), Some(peer_id) 表示远端影子库
    /// * `repo_name`: 仓库名称 (不含 .redb 后缀)
    pub fn get_repo_url(&self, branch: Option<&PeerId>, repo_name: &str) -> Result<Option<String>> {
        let name = repo_name.trim_end_matches(".redb");

        // Helper to read URL from a DB instance
        let read_url = |db: &Database| -> Result<Option<String>> {
            let info = Self::read_repo_info_from_db(db)?;
            Ok(info.and_then(|i| i.url))
        };

        if let Some(peer_id) = branch {
            // Remote (Shadow)
            let db_path = self
                .remotes_dir()
                .join(peer_id.to_filename())
                .join(format!("{}.redb", name));
            if !db_path.exists() {
                return Ok(None);
            }

            // Just open it. Redb handles file locking.
            let db = Database::create(&db_path)?;
            read_url(&db)
        } else {
            // Local
            // 1. Check Main Repo
            if name == self.local_repo_name {
                return read_url(&self.local_db);
            }

            // 2. Check Extra Local DBs Cache
            {
                let guard = self.extra_local_dbs.read().unwrap();
                if let Some(db) = guard.get(name) {
                    return read_url(db);
                }
            }

            // 3. Open temp
            let db_path = self.ledger_dir.join("local").join(format!("{}.redb", name));
            if !db_path.exists() {
                return Ok(None);
            }
            let db = Database::create(&db_path)?;
            read_url(&db)
        }
    }

    /// 查找具有指定 URL 的本地仓库 (Main 或 Extra)
    pub fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        // 1. Check Main Repo
        if let Ok(Some(info)) = Self::read_repo_info_from_db(&self.local_db)
            && info.url.as_deref() == Some(target_url) {
                return Ok(Some(self.local_repo_name.clone()));
            }

        // 2. Iterate all .redb files in ledger/local
        let local_dir = self.ledger_dir.join("local");
        if !local_dir.exists() {
            return Ok(None);
        }

        for entry in std::fs::read_dir(local_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("redb") {
                let file_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();

                // Skip if main (already checked)
                if file_stem == self.local_repo_name {
                    continue;
                }

                // Use run_on_local_repo to safely access/cache
                let is_match = self
                    .run_on_local_repo(file_stem, |db| {
                        let info = Self::read_repo_info_from_db(db)?;
                        Ok(info.and_then(|i| i.url).as_deref() == Some(target_url))
                    })
                    .unwrap_or(false);

                if is_match {
                    return Ok(Some(file_stem.to_string()));
                }
            }
        }

        Ok(None)
    }
}
