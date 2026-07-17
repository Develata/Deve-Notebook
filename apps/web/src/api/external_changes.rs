//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
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
    Rejected {
        status: u16,
        error: Option<ServerError>,
    },
}

impl ExternalChangesMutationError {
    pub fn server_error(&self) -> Option<&ServerError> {
        match self {
            Self::Rejected { error, .. } => error.as_ref(),
            Self::RequestBuild | Self::RequestFailed => None,
        }
    }
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
    let error = response
        .text()
        .await
        .ok()
        .and_then(|body| server_error_from_body(&body));
    ExternalChangesMutationError::Rejected { status, error }
}

fn server_error_from_body(body: &str) -> Option<ServerError> {
    if body.trim().is_empty() {
        return None;
    }
    serde_json::from_str::<ServerError>(body).ok()
}

#[cfg(test)]
#[path = "external_changes_tests.rs"]
mod tests;
