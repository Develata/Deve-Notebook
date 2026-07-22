//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Exact, checkpointed quarantine cleanup of the Deve-owned `.notegit` tree.

use super::{IDENTITY_MARKER_MAX_BYTES, repo_dir, repo_identity_path};
use crate::models::RepoId;
use crate::utils::fs::{
    HostPathIdentity, HostPathKind, HostPathState, HostQuarantineCut, HostQuarantinePlan,
};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotegitRemovalPlan {
    repo_id: RepoId,
    workspace_root: HostPathIdentity,
    notegit_root: HostPathIdentity,
    identity_marker: HostPathIdentity,
    identity_marker_digest: String,
    marker_quarantine: HostQuarantinePlan,
    tree_quarantine: HostQuarantinePlan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotegitRemovalCheckpoint {
    state: NotegitRemovalCheckpointState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum NotegitRemovalCheckpointState {
    Prepared,
    MarkerQuarantined {
        marker: Box<HostQuarantineCut>,
    },
    TreeQuarantined {
        marker: Box<HostQuarantineCut>,
        tree: Box<HostQuarantineCut>,
    },
    TreeDeleted {
        marker: Box<HostQuarantineCut>,
    },
    MarkerDeleted,
}

pub fn prepare_removal(repo_root: &Path, repo_id: RepoId) -> Result<NotegitRemovalPlan> {
    let workspace_root_path = std::fs::canonicalize(repo_root)?;
    let marker_path = repo_identity_path(&workspace_root_path);
    let marker_file =
        crate::utils::fs::open_regular_file_read(&marker_path, "repo removal identity marker")?;
    let marker_len = marker_file.metadata()?.len();
    if marker_len > IDENTITY_MARKER_MAX_BYTES {
        return Err(anyhow!(
            "repo identity marker exceeds removal admission budget"
        ));
    }
    let mut marker_bytes = Vec::with_capacity(marker_len as usize);
    (&marker_file)
        .take(IDENTITY_MARKER_MAX_BYTES + 1)
        .read_to_end(&mut marker_bytes)?;
    if marker_bytes.len() as u64 > IDENTITY_MARKER_MAX_BYTES {
        return Err(anyhow!(
            "repo identity marker exceeds removal admission budget"
        ));
    }
    super::validate_repo_identity_marker_content(&marker_bytes, &workspace_root_path, repo_id)?;
    crate::utils::fs::ensure_open_file_matches_path(
        &marker_file,
        &marker_path,
        "repo removal identity marker",
    )?;

    let workspace_root = HostPathIdentity::capture(&workspace_root_path, HostPathKind::Directory)?;
    let notegit_root =
        HostPathIdentity::capture(&repo_dir(&workspace_root_path), HostPathKind::Directory)?;
    let identity_marker = HostPathIdentity::capture(&marker_path, HostPathKind::RegularFile)?;
    let quarantine_id = uuid::Uuid::new_v4().simple().to_string();
    let marker_quarantine = HostQuarantinePlan::distinct_parent_same_filesystem(
        identity_marker.clone(),
        workspace_root_path.join(format!(".deve-removing-{quarantine_id}-notegit-marker")),
    )?;
    let tree_quarantine = HostQuarantinePlan::same_parent(
        notegit_root.clone(),
        workspace_root_path.join(format!(".deve-removing-{quarantine_id}-notegit")),
    )?;
    Ok(NotegitRemovalPlan {
        repo_id,
        workspace_root,
        notegit_root,
        identity_marker,
        identity_marker_digest: format!("{:x}", Sha256::digest(marker_bytes)),
        marker_quarantine,
        tree_quarantine,
    })
}

impl NotegitRemovalPlan {
    pub const fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn workspace_root(&self) -> &HostPathIdentity {
        &self.workspace_root
    }

    pub fn notegit_root(&self) -> &HostPathIdentity {
        &self.notegit_root
    }

    pub fn initial_checkpoint(&self) -> NotegitRemovalCheckpoint {
        NotegitRemovalCheckpoint {
            state: NotegitRemovalCheckpointState::Prepared,
        }
    }

    pub fn revalidate(&self) -> Result<bool> {
        if self.workspace_root.classify()? != HostPathState::Exact
            || self.notegit_root.classify()? != HostPathState::Exact
            || self.identity_marker.classify()? != HostPathState::Exact
        {
            return Ok(false);
        }
        let marker_file = crate::utils::fs::open_regular_file_read(
            self.identity_marker.path(),
            "repo removal identity marker",
        )?;
        let marker_len = marker_file.metadata()?.len();
        if marker_len > IDENTITY_MARKER_MAX_BYTES {
            return Ok(false);
        }
        let mut bytes = Vec::with_capacity(marker_len as usize);
        (&marker_file)
            .take(IDENTITY_MARKER_MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > IDENTITY_MARKER_MAX_BYTES {
            return Ok(false);
        }
        super::validate_repo_identity_marker_content(
            &bytes,
            self.workspace_root.path(),
            self.repo_id,
        )?;
        crate::utils::fs::ensure_open_file_matches_path(
            &marker_file,
            self.identity_marker.path(),
            "repo removal identity marker",
        )?;
        Ok(format!("{:x}", Sha256::digest(bytes)) == self.identity_marker_digest)
    }

    /// Advances exactly one durable filesystem cut. Repeating this method
    /// after the mutation but before checkpoint persistence reconstructs the
    /// same next checkpoint from the original/quarantine identity pair.
    pub fn advance_cleanup(
        &self,
        checkpoint: &NotegitRemovalCheckpoint,
    ) -> Result<NotegitRemovalCheckpoint> {
        if self.workspace_root.classify()? != HostPathState::Exact {
            return Err(anyhow!(
                "projection workspace identity changed during .notegit cleanup"
            ));
        }
        let state = match &checkpoint.state {
            NotegitRemovalCheckpointState::Prepared => {
                if self.identity_marker.classify()? == HostPathState::Exact && !self.revalidate()? {
                    return Err(anyhow!(
                        "repo identity marker content changed before quarantine"
                    ));
                }
                NotegitRemovalCheckpointState::MarkerQuarantined {
                    marker: Box::new(self.marker_quarantine.cut()?),
                }
            }
            NotegitRemovalCheckpointState::MarkerQuarantined { marker } => {
                require_cut_exact(marker, "identity marker quarantine")?;
                NotegitRemovalCheckpointState::TreeQuarantined {
                    marker: marker.clone(),
                    tree: Box::new(self.tree_quarantine.cut()?),
                }
            }
            NotegitRemovalCheckpointState::TreeQuarantined { marker, tree } => {
                require_cut_exact(marker, "identity marker quarantine")?;
                tree.delete()?;
                NotegitRemovalCheckpointState::TreeDeleted {
                    marker: marker.clone(),
                }
            }
            NotegitRemovalCheckpointState::TreeDeleted { marker } => {
                marker.delete()?;
                NotegitRemovalCheckpointState::MarkerDeleted
            }
            NotegitRemovalCheckpointState::MarkerDeleted => {
                if self.notegit_root.classify()? != HostPathState::Missing {
                    return Err(anyhow!(
                        "completed .notegit cleanup observed a reappeared owner object"
                    ));
                }
                NotegitRemovalCheckpointState::MarkerDeleted
            }
        };
        Ok(NotegitRemovalCheckpoint { state })
    }

    pub fn verify_complete(&self, checkpoint: &NotegitRemovalCheckpoint) -> Result<()> {
        if !checkpoint.is_complete()
            || self.workspace_root.classify()? != HostPathState::Exact
            || self.notegit_root.classify()? != HostPathState::Missing
            || !self.marker_quarantine.quarantine_is_absent()?
            || !self.tree_quarantine.is_fully_absent()?
        {
            return Err(anyhow!(
                "completed .notegit cleanup does not match its exact owner plan"
            ));
        }
        Ok(())
    }
}

impl NotegitRemovalCheckpoint {
    pub fn is_complete(&self) -> bool {
        matches!(self.state, NotegitRemovalCheckpointState::MarkerDeleted)
    }
}

fn require_cut_exact(cut: &HostQuarantineCut, context: &str) -> Result<()> {
    if !cut.original_path_is_absent()? || !cut.is_quarantined_exact()? {
        return Err(anyhow!(
            "{context} is not exact: {:?}",
            cut.exclusive_quarantine_states()?
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Result<(tempfile::TempDir, RepoId, NotegitRemovalPlan)> {
        let dir = tempfile::tempdir()?;
        let repo_id = RepoId::new_v4();
        super::super::ensure_repo_identity_marker(dir.path(), repo_id, "local")?;
        std::fs::write(repo_dir(dir.path()).join("state.bin"), b"runtime")?;
        let plan = prepare_removal(dir.path(), repo_id)?;
        Ok((dir, repo_id, plan))
    }

    #[test]
    fn marker_is_quarantined_before_tree_and_deleted_last() -> Result<()> {
        let (dir, _, plan) = fixture()?;
        let prepared = plan.initial_checkpoint();
        let marker = plan.advance_cleanup(&prepared)?;
        assert!(!repo_identity_path(dir.path()).exists());
        assert!(repo_dir(dir.path()).exists());
        let tree = plan.advance_cleanup(&marker)?;
        assert!(!repo_dir(dir.path()).exists());
        let tree_deleted = plan.advance_cleanup(&tree)?;
        assert!(!tree_deleted.is_complete());
        let complete = plan.advance_cleanup(&tree_deleted)?;
        assert!(complete.is_complete());
        assert!(dir.path().exists());
        Ok(())
    }

    #[test]
    fn mutation_before_checkpoint_is_reconstructed_exactly() -> Result<()> {
        let (_dir, _, plan) = fixture()?;
        let prepared = plan.initial_checkpoint();
        let expected = plan.advance_cleanup(&prepared)?;
        assert_eq!(plan.workspace_root.classify()?, HostPathState::Exact);
        assert_eq!(plan.advance_cleanup(&prepared)?, expected);
        Ok(())
    }

    #[test]
    fn reappeared_marker_blocks_tree_cleanup() -> Result<()> {
        let (dir, _, plan) = fixture()?;
        let prepared = plan.initial_checkpoint();
        let marker = plan.advance_cleanup(&prepared)?;
        let replacement = repo_identity_path(dir.path());
        std::fs::write(&replacement, b"foreign")?;
        assert!(plan.advance_cleanup(&marker).is_err());
        assert_eq!(std::fs::read(replacement)?, b"foreign");
        assert!(repo_dir(dir.path()).exists());
        Ok(())
    }
}
