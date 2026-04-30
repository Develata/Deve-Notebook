//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Read-only Git mirror repair review API.

use gloo_net::http::Request;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GitMirrorRepairReview {
    pub repo_name: String,
    pub manual_only: bool,
    pub authority: String,
    pub records: Vec<GitMirrorRepairReviewRecord>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GitMirrorRepairReviewRecord {
    pub deve_commit_id: String,
    pub ledger_seq: u64,
    pub action_code: String,
    pub retryable_after_fix: bool,
    pub subject: String,
    pub next_step: String,
    pub retry_command: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_command: Option<String>,
    pub failure_exit_status: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitMirrorRepairReviewFetchError {
    RequestFailed,
}

pub async fn fetch_git_mirror_repair_review(
    repo_id: Option<String>,
) -> Result<GitMirrorRepairReview, GitMirrorRepairReviewFetchError> {
    let response = Request::get(&git_mirror_repair_review_url(repo_id.as_deref()))
        .send()
        .await
        .map_err(|_| GitMirrorRepairReviewFetchError::RequestFailed)?;
    response
        .ok()
        .then_some(response)
        .ok_or(GitMirrorRepairReviewFetchError::RequestFailed)?
        .json::<GitMirrorRepairReview>()
        .await
        .map_err(|_| GitMirrorRepairReviewFetchError::RequestFailed)
}

fn git_mirror_repair_review_url(repo_id: Option<&str>) -> String {
    repo_id.map_or_else(
        || "/api/sc/git-mirror/repair-review".to_string(),
        |repo_id| format!("/api/sc/git-mirror/repair-review?repo_id={repo_id}"),
    )
}

#[cfg(test)]
mod tests {
    use super::{GitMirrorRepairReview, git_mirror_repair_review_url};

    #[test]
    fn repair_review_url_uses_repo_id_when_available() {
        assert_eq!(
            git_mirror_repair_review_url(Some("repo-1")),
            "/api/sc/git-mirror/repair-review?repo_id=repo-1"
        );
        assert_eq!(
            git_mirror_repair_review_url(None),
            "/api/sc/git-mirror/repair-review"
        );
    }

    #[test]
    fn repair_review_dto_accepts_server_exit_status_shape() {
        let review: GitMirrorRepairReview = serde_json::from_value(serde_json::json!({
            "repo_name": "default",
            "manual_only": true,
            "authority": ".notegit authority",
            "records": [{
                "deve_commit_id": "deve-1",
                "ledger_seq": 7,
                "action_code": "inspect_git_command",
                "retryable_after_fix": true,
                "subject": "git commit",
                "next_step": "inspect_git_command_output",
                "retry_command": "deve_cli git export --repo default --retry-out-of-sync",
                "failure_stage": "git_command",
                "failure_command": "git commit",
                "failure_exit_status": "128",
                "last_error": "git command failed"
            }]
        }))
        .expect("server repair-review JSON shape should decode in Web");

        assert_eq!(
            review.records[0].failure_exit_status.as_deref(),
            Some("128")
        );
        assert_eq!(
            review.records[0].failure_command.as_deref(),
            Some("git commit")
        );
    }
}
