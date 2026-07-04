//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Read-only Git mirror repair review API.

use super::query::encode_query_component;
use gloo_net::http::Request;
use serde::Deserialize;
use web_sys::RequestCredentials;

use super::native_http::api_url;

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
    scope_nonce: u64,
) -> Result<GitMirrorRepairReview, GitMirrorRepairReviewFetchError> {
    let api = api_url(&git_mirror_repair_review_url(
        repo_id.as_deref(),
        scope_nonce,
    ));
    let mut request = Request::get(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
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

fn git_mirror_repair_review_url(repo_id: Option<&str>, scope_nonce: u64) -> String {
    match repo_id {
        Some(repo_id) => {
            let repo_id = encode_query_component(repo_id);
            format!("/api/sc/git-mirror/repair-review?scope_nonce={scope_nonce}&repo_id={repo_id}")
        }
        None => format!("/api/sc/git-mirror/repair-review?scope_nonce={scope_nonce}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{GitMirrorRepairReview, git_mirror_repair_review_url};

    #[test]
    fn repair_review_url_uses_repo_id_when_available() {
        assert_eq!(
            git_mirror_repair_review_url(Some("repo-1"), 7),
            "/api/sc/git-mirror/repair-review?scope_nonce=7&repo_id=repo-1"
        );
        assert_eq!(
            git_mirror_repair_review_url(None, 7),
            "/api/sc/git-mirror/repair-review?scope_nonce=7"
        );
    }

    #[test]
    fn repair_review_url_encodes_repo_id_query_component() {
        assert_eq!(
            git_mirror_repair_review_url(Some("repo 1&x=1/雪"), 7),
            "/api/sc/git-mirror/repair-review?scope_nonce=7&repo_id=repo%201%26x%3D1%2F%E9%9B%AA"
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
                "retry_command": "deve_cli ngit export --repo default --retry-out-of-sync",
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
