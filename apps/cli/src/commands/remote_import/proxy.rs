//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract

use super::{LocalCliAuthArgs, RemoteImportAction, output};
use crate::commands::live_proxy;
use crate::local_cli_proxy_contract::{LocalCliRemoteImportRequest, LocalCliRemoteImportResponse};
use anyhow::{Context, Result, anyhow, bail};
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse, LoginRequest, LoginResponse};
use deve_core::protocol::{
    RemoteImportCandidateRevision, RemoteImportEntryId, RemoteImportPageCursor,
    RemoteImportRequest, RemoteImportRequestContext, RemoteImportSessionId, ScopeNonce,
};
use reqwest::header;
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

const LOCAL_CLI_SCOPE_NONCE: u64 = 1;

pub(super) fn run(
    ledger_dir: &Path,
    action: RemoteImportAction,
    auth: LocalCliAuthArgs,
) -> Result<()> {
    let username = auth.auth_user.ok_or_else(|| {
        anyhow!("--auth-user is required when the owner server holds the repo DB")
    })?;
    if !auth.auth_password_stdin {
        bail!("--auth-password-stdin is required when the owner server holds the repo DB");
    }
    let password = read_password_from_stdin()?;
    let base = live_proxy::main_base_url(ledger_dir)?;
    let client = live_proxy::local_client()?;
    live_proxy::block_on_safe(async move {
        verify_main_endpoint(&client, &base).await?;
        let jar = login(&client, &base, username, password).await?;
        let request = request_from_action(action);
        let response = client
            .post(format!("{base}/api/local-cli/remote-import"))
            .bearer_auth(jar.token())
            .json(&request)
            .send()
            .await
            .context("Local CLI Remote Import proxy request failed")?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .context("Local CLI Remote Import proxy response body failed")?;
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let auth = serde_json::from_slice::<AuthErrorResponse>(&bytes)
                .map_err(|_| anyhow!("AUTH_INTERNAL_ERROR"))?;
            bail!(auth_code_name(auth.code));
        }
        let response = serde_json::from_slice::<LocalCliRemoteImportResponse>(&bytes)
            .map_err(|_| anyhow!("REMOTE_IMPORT_INVALID_STATE"))?;
        output::print(&response)?;
        output::ensure_success(&response)?;
        if !status.is_success() {
            bail!("REMOTE_IMPORT_INVALID_STATE");
        }
        Ok(())
    })
}

fn request_from_action(action: RemoteImportAction) -> LocalCliRemoteImportRequest {
    let repo_id = action.repo_id();
    let request_id = match &action {
        RemoteImportAction::Apply { request_id, .. } => {
            let request_id = request_id.unwrap_or_else(Uuid::new_v4);
            eprintln!("remote_import_apply_request_id={request_id}");
            request_id
        }
        _ => Uuid::new_v4(),
    };
    let context = RemoteImportRequestContext {
        request_id,
        repo_id,
        branch: None,
        scope_nonce: ScopeNonce::new(LOCAL_CLI_SCOPE_NONCE),
    };
    match action {
        RemoteImportAction::Prepare { provider, .. } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Prepare {
                context,
                provider: provider.into(),
            },
        },
        RemoteImportAction::List { .. } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::List { context },
        },
        RemoteImportAction::Show {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: match revision {
                Some(revision) => RemoteImportRequest::Page {
                    context,
                    session_id: RemoteImportSessionId::new(session),
                    revision: RemoteImportCandidateRevision::new(revision),
                    cursor: None::<RemoteImportPageCursor>,
                    limit: deve_core::protocol::remote_import::REMOTE_IMPORT_DEFAULT_PAGE_SIZE,
                },
                None => RemoteImportRequest::Show {
                    context,
                    session_id: RemoteImportSessionId::new(session),
                    revision: None,
                },
            },
        },
        RemoteImportAction::Diff {
            session,
            revision,
            entry,
            ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Diff {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: RemoteImportCandidateRevision::new(revision),
                entry_id: RemoteImportEntryId::new(entry),
            },
        },
        RemoteImportAction::Refresh {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Refresh {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: RemoteImportCandidateRevision::new(revision),
            },
        },
        RemoteImportAction::Apply {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Apply {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: RemoteImportCandidateRevision::new(revision),
            },
        },
        RemoteImportAction::Discard {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Discard {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: revision.map(RemoteImportCandidateRevision::new),
            },
        },
        RemoteImportAction::Repair { apply, .. } => {
            LocalCliRemoteImportRequest::Repair { context, apply }
        }
    }
}

struct LocalCliTokenJar(String);

impl LocalCliTokenJar {
    fn token(&self) -> &str {
        &self.0
    }
}

async fn login(
    client: &reqwest::Client,
    base: &str,
    username: String,
    password: String,
) -> Result<LocalCliTokenJar> {
    let response = client
        .post(format!("{base}/api/auth/login"))
        .json(&LoginRequest { username, password })
        .send()
        .await
        .context("Local CLI operator login failed")?;
    let status = response.status();
    let token = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(extract_token_cookie)
        .collect::<Vec<_>>();
    let body = response
        .json::<LoginResponse>()
        .await
        .map_err(|_| anyhow!("AUTH_INTERNAL_ERROR"))?;
    if !status.is_success() || !body.success {
        bail!(auth_code_name(
            body.code.unwrap_or(AuthErrorCode::InternalError)
        ));
    }
    if token.len() != 1 {
        bail!("AUTH_TOKEN_MISSING");
    }
    Ok(LocalCliTokenJar(token.into_iter().next().unwrap()))
}

fn extract_token_cookie(value: &str) -> Option<String> {
    let pair = value.split(';').next()?.trim();
    let token = pair.strip_prefix("token=")?;
    (!token.is_empty()).then(|| token.to_string())
}

#[derive(Deserialize)]
struct NodeRole {
    role: String,
    ws_port: u16,
    main_port: u16,
}

async fn verify_main_endpoint(client: &reqwest::Client, base: &str) -> Result<()> {
    let expected_port = reqwest::Url::parse(base)?
        .port_or_known_default()
        .ok_or_else(|| anyhow!("Main process endpoint has no port"))?;
    let role = client
        .get(format!("{base}/api/node/role"))
        .send()
        .await?
        .error_for_status()?
        .json::<NodeRole>()
        .await?;
    if !matches!(role.role.as_str(), "main" | "native-main")
        || role.ws_port != expected_port
        || role.main_port != expected_port
    {
        bail!("Main process endpoint identity mismatch");
    }
    Ok(())
}

fn read_password_from_stdin() -> Result<String> {
    let mut password = String::new();
    std::io::stdin()
        .read_to_string(&mut password)
        .context("Failed to read operator password from stdin")?;
    while matches!(password.as_bytes().last(), Some(b'\n' | b'\r')) {
        password.pop();
    }
    if password.is_empty() {
        bail!("Operator password from stdin is empty");
    }
    Ok(password)
}

fn auth_code_name(code: AuthErrorCode) -> &'static str {
    match code {
        AuthErrorCode::InvalidPassword => "AUTH_INVALID_PASSWORD",
        AuthErrorCode::TokenExpired => "AUTH_TOKEN_EXPIRED",
        AuthErrorCode::TokenMissing => "AUTH_TOKEN_MISSING",
        AuthErrorCode::RateLimited => "AUTH_RATE_LIMITED",
        AuthErrorCode::CsrfMismatch => "AUTH_CSRF_MISMATCH",
        AuthErrorCode::InternalError => "AUTH_INTERNAL_ERROR",
    }
}
