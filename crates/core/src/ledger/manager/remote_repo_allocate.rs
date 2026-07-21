//! plan_ref:
//!   - 04_repository#repo-catalog-repair-contract

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;
use anyhow::{Context, Result};
use std::path::PathBuf;

impl RepoManager {
    pub(crate) fn allocate_remote_repo_path(
        &self,
        peer_id: &PeerId,
        info: &RepoInfo,
    ) -> Result<PathBuf> {
        let peer_dir = self.remotes_dir().join(peer_id.to_filename());
        std::fs::create_dir_all(&peer_dir)?;
        let path = peer_dir.join(format!("{}.redb", info.uuid));
        if !path
            .try_exists()
            .with_context(|| format!("Failed to stat remote repo path candidate: {:?}", path))?
        {
            return Ok(path);
        }
        match Self::read_shadow_repo_info_from_path(&path)? {
            Some(current) if current.uuid == info.uuid => Ok(path),
            Some(current) => anyhow::bail!(
                "Shadow authority path collision for RepoId {}: metadata belongs to {}",
                info.uuid,
                current.uuid
            ),
            // A UUID-named shadow may be created before authenticated remote metadata arrives.
            // The caller-supplied `info` is the only value admitted here; local catalog metadata
            // is never borrowed to infer the shadow display identity.
            None => Ok(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::{RepoInfo, RepoManager};
    use crate::models::PeerId;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    #[test]
    fn allocate_remote_repo_path_fails_closed_on_unstatable_peer_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
            .expect("repo");
        let peer = PeerId::new("peer-a");
        let peer_dir = repo.remotes_dir().join(peer.to_filename());
        std::fs::create_dir_all(&peer_dir).expect("peer dir");
        let original = std::fs::metadata(&peer_dir)
            .expect("metadata")
            .permissions();
        let mut blocked = original.clone();
        blocked.set_mode(0o000);
        std::fs::set_permissions(&peer_dir, blocked).expect("chmod 000");

        let err = repo
            .allocate_remote_repo_path(
                &peer,
                &RepoInfo {
                    uuid: uuid::Uuid::new_v4(),
                    name: "notes".into(),
                    url: Some("urn:test:notes".into()),
                },
            )
            .expect_err("unstatable peer dir must fail closed");

        std::fs::set_permissions(&peer_dir, original).expect("restore perms");
        assert!(
            err.to_string()
                .contains("Failed to stat remote repo path candidate")
                || err.to_string().contains("Permission denied")
        );
    }

    #[test]
    fn allocate_remote_repo_path_accepts_exact_uuid_shadow_awaiting_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
            .expect("repo");
        let peer = PeerId::new("peer-a");
        let peer_dir = repo.remotes_dir().join(peer.to_filename());
        std::fs::create_dir_all(&peer_dir).expect("peer dir");
        let repo_id = uuid::Uuid::new_v4();
        crate::test_support::create_repo_db_missing_metadata(
            peer_dir.join(format!("{}.redb", repo_id)),
        );

        let path = repo
            .allocate_remote_repo_path(
                &peer,
                &RepoInfo {
                    uuid: repo_id,
                    name: "notes".into(),
                    url: Some("urn:test:notes".into()),
                },
            )
            .expect("exact UUID shadow may await authenticated metadata");

        assert_eq!(path, peer_dir.join(format!("{}.redb", repo_id)));
    }
}
