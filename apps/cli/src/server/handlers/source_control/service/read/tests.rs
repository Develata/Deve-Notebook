//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
//! Source-control read service tests.

use super::diff_doc_target;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{
    ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo, SourceControlApi,
};
use std::sync::Mutex;

struct FakeRepo {
    seen_target: Mutex<Option<ScPathTarget>>,
}

impl SourceControlApi for FakeRepo {
    fn list_pending_fs_in_repo(&self, _: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        Ok(vec![])
    }

    fn list_staged_in_repo(&self, _: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        Ok(vec![])
    }

    fn stage_pending_in_repo(&self, _: &RepoSelector, _: &ScPathTarget) -> anyhow::Result<()> {
        Ok(())
    }

    fn discard_pending_in_repo(&self, _: &RepoSelector, _: &ScPathTarget) -> anyhow::Result<()> {
        Ok(())
    }

    fn unstage_file_in_repo(&self, _: &RepoSelector, _: &ScPathTarget) -> anyhow::Result<()> {
        Ok(())
    }

    fn list_changes_in_repo(&self, _: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        Ok(vec![ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: None,
            status: ChangeStatus::Added,
            has_conflict: false,
        }])
    }

    fn diff_doc_path_in_repo(
        &self,
        _: &RepoSelector,
        target: &ScPathTarget,
    ) -> anyhow::Result<String> {
        *self.seen_target.lock().unwrap() = Some(target.clone());
        Ok(String::new())
    }

    fn list_commits_in_repo(&self, _: &RepoSelector, _: u32) -> anyhow::Result<Vec<CommitInfo>> {
        Ok(vec![])
    }

    fn diff_commits_in_repo(
        &self,
        _: &RepoSelector,
        _: Option<&str>,
        _: &str,
    ) -> anyhow::Result<Vec<CommitFileDiff>> {
        Ok(vec![])
    }

    fn commit_staged_in_repo(&self, _: &RepoSelector, _: &str) -> anyhow::Result<CommitInfo> {
        unreachable!("unused in this test")
    }
}

#[test]
fn diff_doc_target_forwards_resolved_target() {
    let repo = FakeRepo {
        seen_target: Mutex::new(None),
    };
    diff_doc_target(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget::from_path("notes/old.md"),
    )
    .expect("diff target should resolve");
    assert_eq!(
        repo.seen_target.lock().unwrap().clone(),
        Some(ScPathTarget::from_path("notes/new.md"))
    );
}
