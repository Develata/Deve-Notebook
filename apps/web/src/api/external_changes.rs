//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!
//! External Changes HTTP intent facade.

use super::native_http::api_url;
use super::query::encode_query_component;
use deve_core::models::DocId;
use deve_core::protocol::ServerError;
use deve_core::source_control::{ChangeDomain, ChangeEntry};
use gloo_net::http::Request;
use serde::Serialize;
use serde::de::DeserializeOwned;
use web_sys::RequestCredentials;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalChangesTargetOp {
    Stage,
    Unstage,
    Discard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalChangesMutationError {
    RequestBuild,
    RequestFailed,
    Rejected { status: u16, detail: Option<String> },
}

#[derive(Clone, Debug)]
pub struct ExternalChangesSnapshot {
    pub staged: Vec<ChangeEntry>,
    pub unstaged: Vec<ChangeEntry>,
}

pub async fn mutate_external_change_target(
    op: ExternalChangesTargetOp,
    repo_id: Option<String>,
    scope_nonce: u64,
    entry: ChangeEntry,
) -> Result<(), ExternalChangesMutationError> {
    let payload = TargetMutationPayload::from_entry(repo_id, scope_nonce, &entry);
    post_json(op.endpoint(), &payload).await
}

pub async fn apply_external_changes_to_ledger(
    repo_id: Option<String>,
    scope_nonce: u64,
) -> Result<Vec<ChangeEntry>, ExternalChangesMutationError> {
    post_json_for_response(
        "/api/sc/apply-external-changes",
        &ApplyExternalChangesPayload {
            scope_nonce,
            repo_id,
        },
    )
    .await
}

pub async fn fetch_external_changes(
    repo_id: Option<String>,
    scope_nonce: u64,
) -> Result<ExternalChangesSnapshot, ExternalChangesMutationError> {
    let staged = get_json::<Vec<ChangeEntry>>(&sc_query_url(
        "/api/sc/staged",
        repo_id.as_deref(),
        scope_nonce,
    ))
    .await?;
    let unstaged = get_json::<Vec<ChangeEntry>>(&sc_query_url(
        "/api/sc/pending",
        repo_id.as_deref(),
        scope_nonce,
    ))
    .await?;
    Ok(ExternalChangesSnapshot { staged, unstaged })
}

impl ExternalChangesTargetOp {
    fn endpoint(self) -> &'static str {
        match self {
            Self::Stage => "/api/sc/stage-pending",
            Self::Unstage => "/api/sc/unstage",
            Self::Discard => "/api/sc/discard-pending",
        }
    }
}

#[derive(Serialize)]
struct TargetMutationPayload {
    scope_nonce: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<String>,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc_id: Option<DocId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<ChangeDomain>,
}

impl TargetMutationPayload {
    fn from_entry(repo_id: Option<String>, scope_nonce: u64, entry: &ChangeEntry) -> Self {
        Self {
            scope_nonce,
            repo_id,
            path: deve_core::utils::path::to_forward_slash(&entry.path),
            doc_id: entry.doc_id,
            domain: Some(entry.domain),
        }
    }
}

#[derive(Serialize)]
struct ApplyExternalChangesPayload {
    scope_nonce: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<String>,
}

fn sc_query_url(path: &str, repo_id: Option<&str>, scope_nonce: u64) -> String {
    let mut url = format!("{path}?scope_nonce={scope_nonce}");
    if let Some(repo_id) = repo_id {
        url.push_str("&repo_id=");
        url.push_str(&encode_query_component(repo_id));
    }
    url
}

async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, ExternalChangesMutationError> {
    let api = api_url(path);
    let mut request = Request::get(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .send()
        .await
        .map_err(|_| ExternalChangesMutationError::RequestFailed)?;
    if !response.ok() {
        return Err(rejected_error(response).await);
    }
    response
        .json::<T>()
        .await
        .map_err(|_| ExternalChangesMutationError::RequestFailed)
}

async fn post_json_for_response<T: Serialize, R: DeserializeOwned>(
    path: &str,
    payload: &T,
) -> Result<R, ExternalChangesMutationError> {
    let api = api_url(path);
    let mut request = Request::post(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|_| ExternalChangesMutationError::RequestBuild)?
        .send()
        .await
        .map_err(|_| ExternalChangesMutationError::RequestFailed)?;
    if !response.ok() {
        return Err(rejected_error(response).await);
    }
    response
        .json::<R>()
        .await
        .map_err(|_| ExternalChangesMutationError::RequestFailed)
}

async fn post_json<T: Serialize>(
    path: &str,
    payload: &T,
) -> Result<(), ExternalChangesMutationError> {
    let api = api_url(path);
    let mut request = Request::post(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|_| ExternalChangesMutationError::RequestBuild)?
        .send()
        .await
        .map_err(|_| ExternalChangesMutationError::RequestFailed)?;
    if !response.ok() {
        return Err(rejected_error(response).await);
    }
    Ok(())
}

async fn rejected_error(response: gloo_net::http::Response) -> ExternalChangesMutationError {
    let status = response.status();
    let detail = response
        .text()
        .await
        .ok()
        .and_then(|body| rejection_detail_from_body(&body));
    ExternalChangesMutationError::Rejected { status, detail }
}

fn rejection_detail_from_body(body: &str) -> Option<String> {
    if body.trim().is_empty() {
        return None;
    }
    serde_json::from_str::<ServerError>(body)
        .ok()
        .and_then(|error| error.detail)
        .filter(|detail| !detail.trim().is_empty())
        .or_else(|| Some(body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        ExternalChangesTargetOp, TargetMutationPayload, rejection_detail_from_body, sc_query_url,
    };
    use deve_core::models::DocId;
    use deve_core::protocol::{ServerError, ServerErrorCode};
    use deve_core::source_control::{ChangeDomain, ChangeEntry, ChangeStatus};

    fn entry() -> ChangeEntry {
        ChangeEntry {
            path: "notes\\a.md".into(),
            renamed_from: None,
            doc_id: Some(DocId::from_u128(9)),
            status: ChangeStatus::Modified,
            has_conflict: true,
            domain: ChangeDomain::WorkingDirectory,
            base_seq: None,
            target_seq: None,
        }
    }

    #[test]
    fn target_payload_preserves_repo_scope_identity_and_domain() {
        let payload = TargetMutationPayload::from_entry(Some("repo-1".into()), 7, &entry());
        let json = serde_json::to_value(payload).expect("payload json");

        assert_eq!(json["scope_nonce"], 7);
        assert_eq!(json["repo_id"], "repo-1");
        assert_eq!(json["path"], "notes/a.md");
        assert_eq!(json["doc_id"], DocId::from_u128(9).to_string());
        assert_eq!(json["domain"], "WorkingDirectory");
    }

    #[test]
    fn target_ops_use_external_change_mutation_endpoints() {
        assert_eq!(
            ExternalChangesTargetOp::Stage.endpoint(),
            "/api/sc/stage-pending"
        );
        assert_eq!(
            ExternalChangesTargetOp::Unstage.endpoint(),
            "/api/sc/unstage"
        );
        assert_eq!(
            ExternalChangesTargetOp::Discard.endpoint(),
            "/api/sc/discard-pending"
        );
    }

    #[test]
    fn external_changes_query_url_preserves_repo_scope() {
        assert_eq!(
            sc_query_url("/api/sc/pending", Some("repo 1&x=1/雪"), 9),
            "/api/sc/pending?scope_nonce=9&repo_id=repo%201%26x%3D1%2F%E9%9B%AA"
        );
        assert_eq!(
            sc_query_url("/api/sc/staged", None, 9),
            "/api/sc/staged?scope_nonce=9"
        );
    }

    #[test]
    fn rejected_error_detail_prefers_structured_server_detail() {
        let body = serde_json::to_string(&ServerError::with_detail(
            ServerErrorCode::ScPendingNotFound,
            "pending target vanished",
        ))
        .expect("server error json");

        assert_eq!(
            rejection_detail_from_body(&body).as_deref(),
            Some("pending target vanished")
        );
        assert_eq!(
            rejection_detail_from_body("plain backend failure").as_deref(),
            Some("plain backend failure")
        );
        assert_eq!(rejection_detail_from_body(""), None);
    }
}
