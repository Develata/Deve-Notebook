use super::super::errors::{self, ScOp};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};

pub fn list_pending(
    repo: &dyn Repository,
    selector: &RepoSelector,
) -> super::ScResult<Vec<ChangeEntry>> {
    repo.list_pending_fs_in_repo(selector)
        .map(super::super::present::collapse_rename_candidates)
        .map_err(|e| errors::map_repo_error(ScOp::ListPending, e))
}

pub fn list_changes(
    repo: &dyn Repository,
    selector: &RepoSelector,
) -> super::ScResult<Vec<ChangeEntry>> {
    repo.list_changes_in_repo(selector)
        .map(super::super::present::collapse_rename_candidates)
        .map_err(|e| errors::map_repo_error(ScOp::ListChanges, e))
}

pub fn diff_doc_target(
    repo: &dyn Repository,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> super::ScResult<String> {
    let entries = list_changes(repo, selector)?;
    let resolved = super::resolve_target(&entries, target);
    let path = resolved.path.clone();
    repo.diff_doc_path_in_repo(selector, &resolved)
        .map_err(|e| errors::map_repo_error(ScOp::DiffDoc(path), e))
}

pub fn list_commit_history(
    repo: &dyn Repository,
    selector: &RepoSelector,
    limit: u32,
) -> super::ScResult<Vec<CommitInfo>> {
    repo.list_commits_in_repo(selector, limit)
        .map_err(|e| errors::map_repo_error(ScOp::CommitHistory, e))
}

pub fn diff_commits(
    repo: &dyn Repository,
    selector: &RepoSelector,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
) -> super::ScResult<Vec<CommitFileDiff>> {
    repo.diff_commits_in_repo(selector, commit_a_id, commit_b_id)
        .map_err(|e| errors::map_repo_error(ScOp::CommitDiff(commit_b_id.to_string()), e))
}

#[cfg(test)]
mod tests {
    use super::diff_doc_target;
    use deve_core::ledger::traits::{RepoSelector, Repository};
    use deve_core::models::DocId;
    use deve_core::protocol::ScPathTarget;
    use deve_core::source_control::{ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo};
    use std::sync::Mutex;

    struct FakeRepo {
        seen_target: Mutex<Option<ScPathTarget>>,
    }

    impl Repository for FakeRepo {
        fn list_docs_in_repo(&self, _: &RepoSelector) -> anyhow::Result<Vec<(DocId, String)>> {
            Ok(vec![])
        }
        fn get_doc_content_in_repo(&self, _: &RepoSelector, _: DocId) -> anyhow::Result<String> {
            Ok(String::new())
        }
        fn list_pending_fs_in_repo(&self, _: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
            Ok(vec![])
        }
        fn stage_pending_in_repo(&self, _: &RepoSelector, _: &ScPathTarget) -> anyhow::Result<()> {
            Ok(())
        }
        fn discard_pending_in_repo(
            &self,
            _: &RepoSelector,
            _: &ScPathTarget,
        ) -> anyhow::Result<()> {
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
        fn list_commits_in_repo(
            &self,
            _: &RepoSelector,
            _: u32,
        ) -> anyhow::Result<Vec<CommitInfo>> {
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
}
