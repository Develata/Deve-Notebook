//! plan_ref:
//!   - 04_repository#repo-catalog-repair-contract
//!   - 14_commands#cli-commands

use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn quarantine_nil_shadow_repos(remotes_dir: &Path) -> Result<usize> {
    match remotes_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => return Ok(0),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("failed to stat remotes dir {:?}", remotes_dir));
        }
    }
    let nil_name = format!("{}.redb", uuid::Uuid::nil());
    let quarantine_root = remotes_dir.join(".invalid");
    let mut moved = 0usize;
    for entry in std::fs::read_dir(remotes_dir)? {
        let entry = entry?;
        let peer_dir = entry.path();
        let peer_name = peer_dir
            .file_name()
            .and_then(|s| s.to_str())
            .context("invalid peer dir name")?;
        if peer_name == ".invalid" {
            continue;
        }
        if peer_name.starts_with('.') {
            anyhow::bail!(
                "Broken shadow peer entry {:?} while quarantining nil shadows: unexpected hidden directory",
                peer_dir
            );
        }
        if !entry.file_type()?.is_dir() {
            anyhow::bail!(
                "Broken shadow peer entry {:?} while quarantining nil shadows: expected directory",
                peer_dir
            );
        }
        let invalid = peer_dir.join(&nil_name);
        match invalid.try_exists() {
            Ok(true) => {}
            Ok(false) => continue,
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to stat nil shadow {:?}", invalid));
            }
        }
        let dst_dir = quarantine_root.join(peer_name);
        std::fs::create_dir_all(&dst_dir)?;
        let dst = dst_dir.join(&nil_name);
        std::fs::rename(&invalid, &dst)?;
        println!(
            "repair: quarantined invalid shadow {:?} -> {:?}",
            invalid, dst
        );
        moved += 1;
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::quarantine_nil_shadow_repos;
    use tempfile::tempdir;

    #[test]
    fn quarantines_nil_shadow_repo_into_invalid_peer_dir() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let remotes = dir.path().join("remotes");
        let peer_dir = remotes.join("peer-a");
        std::fs::create_dir_all(&peer_dir)?;
        let nil_db = peer_dir.join(format!("{}.redb", uuid::Uuid::nil()));
        std::fs::write(&nil_db, b"broken")?;

        let moved = quarantine_nil_shadow_repos(&remotes)?;

        assert_eq!(moved, 1);
        assert!(!nil_db.exists());
        assert!(
            remotes
                .join(".invalid")
                .join("peer-a")
                .join(format!("{}.redb", uuid::Uuid::nil()))
                .exists()
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn fails_closed_on_invalid_peer_dir_name() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let dir = tempdir().expect("tempdir");
        let remotes = dir.path().join("remotes");
        let invalid = remotes.join(OsString::from_vec(vec![0xff, b'p', b'e', b'e', b'r']));
        std::fs::create_dir_all(&invalid).expect("invalid dir");

        let err = quarantine_nil_shadow_repos(&remotes)
            .expect_err("invalid peer dir name must fail closed");
        assert!(err.to_string().contains("invalid peer dir name"));
    }

    #[test]
    fn fails_closed_on_unexpected_non_directory_entry() {
        let dir = tempdir().expect("tempdir");
        let remotes = dir.path().join("remotes");
        std::fs::create_dir_all(&remotes).expect("remotes");
        let bogus = remotes.join("peer-a");
        std::fs::write(&bogus, b"not-a-dir").expect("bogus file");

        let err = quarantine_nil_shadow_repos(&remotes)
            .expect_err("non-directory peer entry must fail closed");
        assert!(err.to_string().contains("expected directory"));
    }

    #[test]
    fn fails_closed_on_unexpected_hidden_peer_dir() {
        let dir = tempdir().expect("tempdir");
        let remotes = dir.path().join("remotes");
        let hidden = remotes.join(".peer-a");
        std::fs::create_dir_all(&hidden).expect("hidden dir");

        let err = quarantine_nil_shadow_repos(&remotes)
            .expect_err("unexpected hidden peer dir must fail closed");
        assert!(err.to_string().contains("unexpected hidden directory"));
    }

    #[cfg(unix)]
    #[test]
    fn fails_closed_on_unstatable_nil_shadow_path() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let remotes = dir.path().join("remotes");
        let peer_dir = remotes.join("peer-a");
        std::fs::create_dir_all(&peer_dir).expect("peer dir");
        std::fs::write(
            peer_dir.join(format!("{}.redb", uuid::Uuid::nil())),
            b"broken",
        )
        .expect("nil db");
        let original = std::fs::metadata(&peer_dir)
            .expect("metadata")
            .permissions();
        let mut blocked = original.clone();
        blocked.set_mode(0o000);
        std::fs::set_permissions(&peer_dir, blocked).expect("chmod 000");

        let err = quarantine_nil_shadow_repos(&remotes)
            .expect_err("unstatable nil shadow must fail closed");

        std::fs::set_permissions(&peer_dir, original).expect("restore perms");
        assert!(
            err.to_string().contains("failed to stat nil shadow")
                || err.to_string().contains("Permission denied")
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_peer_dir_is_not_misclassified_as_non_directory() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let remotes = dir.path().join("remotes");
        let peer_dir = remotes.join("peer-a");
        std::fs::create_dir_all(&peer_dir).expect("peer dir");
        let original = std::fs::metadata(&peer_dir)
            .expect("metadata")
            .permissions();
        let mut blocked = original.clone();
        blocked.set_mode(0o000);
        std::fs::set_permissions(&peer_dir, blocked).expect("chmod 000");

        let err = quarantine_nil_shadow_repos(&remotes)
            .expect_err("unreadable peer dir must fail closed");

        std::fs::set_permissions(&peer_dir, original).expect("restore perms");
        let detail = err.to_string();
        assert!(
            detail.contains("failed to stat nil shadow") || detail.contains("Permission denied"),
            "unexpected error detail: {detail}"
        );
        assert!(!detail.contains("expected directory"));
    }
}
