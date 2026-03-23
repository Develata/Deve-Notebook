use anyhow::Result;

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;

impl RepoManager {
    pub fn get_local_repo_info_by_id(&self, repo_id: uuid::Uuid) -> Result<Option<RepoInfo>> {
        self.refresh_local_repo_catalog()?;
        self.get_local_repo_info_by_id_without_repair(repo_id)
    }

    pub(crate) fn get_local_repo_info_by_id_without_repair(
        &self,
        repo_id: uuid::Uuid,
    ) -> Result<Option<RepoInfo>> {
        let Some(repo_stem) = self.find_local_repo_name_by_id_without_repair(repo_id)? else {
            return Ok(None);
        };
        if repo_stem == self.local_repo_name {
            return Self::read_repo_info_from_db(&self.local_db);
        }
        self.run_on_local_repo_stem(&repo_stem, Self::read_repo_info_from_db)
    }

    pub fn get_repo_url(&self, branch: Option<&PeerId>, repo_name: &str) -> Result<Option<String>> {
        Ok(self
            .get_repo_info_for(branch, Some(repo_name))?
            .and_then(|info| info.url))
    }

    pub fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        self.refresh_local_repo_catalog()?;
        let mut matches = Vec::new();
        for repo_name in self.list_local_repo_names_for_execution()? {
            let info = self
                .get_repo_info_for(None, Some(&repo_name))?
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Broken local repo {} while resolving URL {}: repository metadata missing",
                        repo_name,
                        target_url
                    )
                })?;
            if info.url.as_deref() == Some(target_url) {
                matches.push(repo_name);
            }
        }
        Ok((matches.len() == 1).then(|| matches[0].clone()))
    }

    pub fn get_repo_info_for(
        &self,
        branch: Option<&PeerId>,
        repo_name: Option<&str>,
    ) -> Result<Option<RepoInfo>> {
        let name = repo_name
            .unwrap_or(&self.local_repo_name)
            .trim_end_matches(".redb");
        if let Some(peer_id) = branch {
            return self.read_remote_repo_info(peer_id, name);
        }
        if let Some(stem) = self.resolve_local_repo_stem(name)? {
            return if stem == self.local_repo_name {
                Self::read_repo_info_from_db(&self.local_db)
            } else {
                self.run_on_local_repo_stem(&stem, Self::read_repo_info_from_db)
            };
        }
        self.refresh_local_repo_catalog()?;
        if let Some(stem) = self.resolve_local_repo_stem(name)? {
            return if stem == self.local_repo_name {
                Self::read_repo_info_from_db(&self.local_db)
            } else {
                self.run_on_local_repo_stem(&stem, Self::read_repo_info_from_db)
            };
        }
        Ok(None)
    }

    fn read_remote_repo_info(&self, peer_id: &PeerId, repo_name: &str) -> Result<Option<RepoInfo>> {
        if let Some(entry) = self.resolve_remote_repo_entry(peer_id, repo_name)? {
            if let Some(info) = entry.info {
                return Ok(Some(info));
            }
            return Ok(None);
        }
        Ok(None)
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

    fn create_secondary_repo(ledger_dir: &std::path::Path, name: &str, url: &str) -> RepoInfo {
        let local_dir = ledger_dir.join("local");
        std::fs::create_dir_all(&local_dir).expect("create local dir");
        let path = local_dir.join(format!("{name}.redb"));
        let db = redb::Database::create(&path).expect("create secondary db");
        crate::ledger::source_control::init_tables(&db).expect("init source control tables");
        let txn = db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA).expect("repo metadata");
        txn.commit().expect("commit metadata table");
        let info = RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: name.into(),
            url: Some(url.into()),
        };
        write_info(&db, &info);
        drop(db);
        info
    }

    #[test]
    fn local_repo_info_lookup_without_repair_preserves_unrepaired_metadata() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        let wiki_info = create_secondary_repo(&ledger_dir, "wiki", "urn:wiki");
        let wiki_db = main.open_database(None, "wiki").expect("wiki db");
        write_info(
            wiki_db.db.as_ref(),
            &RepoInfo {
                uuid: wiki_info.uuid,
                name: "alias".into(),
                url: Some("urn:alias".into()),
            },
        );

        let looked_up = main
            .get_local_repo_info_by_id_without_repair(wiki_info.uuid)
            .expect("lookup")
            .expect("present");
        assert_eq!(looked_up.name, "alias");
        assert_eq!(looked_up.url.as_deref(), Some("urn:alias"));
    }

    #[test]
    fn local_repo_info_lookup_without_repair_fails_closed_on_broken_main_metadata() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        let main_info = main.get_repo_info().expect("main info").expect("present");
        let main_db = main.open_database(None, "main").expect("main db");

        let txn = main_db.db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA)
            .expect("repo metadata")
            .insert(&0, b"not-bincode".as_slice())
            .expect("poison metadata");
        txn.commit().expect("commit poison");

        let err = main
            .get_local_repo_info_by_id_without_repair(main_info.uuid)
            .expect_err("broken main metadata must fail closed");
        assert!(
            err.to_string()
                .contains("Broken local repo main while resolving UUID")
        );
    }

    #[test]
    fn local_repo_name_by_url_fails_closed_on_missing_main_metadata() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        let main_db = main.open_database(None, "main").expect("main db");

        let txn = main_db.db.begin_write().expect("write txn");
        txn.delete_table(REPO_METADATA).expect("delete metadata");
        txn.commit().expect("commit");

        let err = main
            .find_local_repo_name_by_url("urn:main")
            .expect_err("missing main metadata must fail closed");
        assert!(err.to_string().contains("repository metadata missing"));
    }
}
