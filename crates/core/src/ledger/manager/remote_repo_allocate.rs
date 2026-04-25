//! plan_ref:
//!   - 06_repository#repo-catalog-repair-contract

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
        let base = normalized_repo_stem(&info.name, &info.uuid.to_string());
        for suffix in 0.. {
            let stem = if suffix == 0 {
                base.clone()
            } else {
                format!("{}-{}", base, suffix)
            };
            let path = peer_dir.join(format!("{}.redb", stem));
            if !path
                .try_exists()
                .with_context(|| format!("Failed to stat remote repo path candidate: {:?}", path))?
            {
                return Ok(path);
            }
            match Self::read_repo_info_from_path(&path)? {
                Some(current) if current.uuid == info.uuid => {
                    if stem_matches_base(&stem, &base) {
                        return Ok(path);
                    }
                    continue;
                }
                Some(_) => {}
                None if stem == info.uuid.to_string() => return Ok(path),
                None => {
                    anyhow::bail!(
                        "Broken shadow repo {:?} while allocating remote repo path: repository metadata missing",
                        path
                    );
                }
            }
        }
        unreachable!("remote repo path allocator must terminate")
    }
}

fn normalized_repo_stem(name: &str, fallback: &str) -> String {
    let trimmed = name.trim_end_matches(".redb").trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    trimmed.replace(['/', '\\'], "_")
}

fn stem_matches_base(stem: &str, base: &str) -> bool {
    stem == base
        || stem
            .strip_prefix(base)
            .and_then(|suffix| suffix.strip_prefix('-'))
            .map(|suffix| suffix.chars().all(|ch| ch.is_ascii_digit()))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::stem_matches_base;
    use crate::ledger::{RepoInfo, RepoManager};
    use crate::models::PeerId;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn collision_suffix_is_treated_as_same_selector_family() {
        assert!(stem_matches_base("wiki", "wiki"));
        assert!(stem_matches_base("wiki-1", "wiki"));
        assert!(stem_matches_base("wiki-23", "wiki"));
        assert!(!stem_matches_base("legacy", "wiki"));
        assert!(!stem_matches_base("wiki-copy", "wiki"));
    }

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
    fn allocate_remote_repo_path_fails_closed_on_named_path_missing_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))
            .expect("repo");
        let peer = PeerId::new("peer-a");
        let peer_dir = repo.remotes_dir().join(peer.to_filename());
        std::fs::create_dir_all(&peer_dir).expect("peer dir");
        crate::test_support::create_repo_db_missing_metadata(peer_dir.join("notes.redb"));

        let err = repo
            .allocate_remote_repo_path(
                &peer,
                &RepoInfo {
                    uuid: uuid::Uuid::new_v4(),
                    name: "notes".into(),
                    url: Some("urn:test:notes".into()),
                },
            )
            .expect_err("named shadow path missing metadata must fail closed");

        assert!(err.to_string().contains("repository metadata missing"));
    }
}
