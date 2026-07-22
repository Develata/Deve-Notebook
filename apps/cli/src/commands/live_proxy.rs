//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 08_auth#jwt-cookie-contract
//!   - 14_commands#cli-commands
//!   - 18_release#runtime-observability

use crate::admin_api::{
    DumpResponse, ExportEntry, GitStatusResponse, NodeCheckResponse, ProjectionCheckResponse,
    ScStatusResponse,
};
use crate::local_cli_proxy_contract::LocalCliOwnerHint;
use anyhow::{Context, Result, anyhow};
use deve_core::config::RuntimeEnvironment;
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse, LoginRequest, LoginResponse};
use deve_core::security::AuthConfig;
use reqwest::{Client, RequestBuilder, header};
use serde::Deserialize;
use std::future::Future;
use std::io::Read;
use std::path::Path;

const DEFAULT_MAIN_PORT: u16 = 3001;

#[derive(Debug, Deserialize)]
struct NodeRoleResponse {
    role: String,
    ws_port: u16,
    main_port: u16,
    #[serde(default)]
    host_peer_id: Option<String>,
    #[serde(default)]
    runtime_incarnation: Option<uuid::Uuid>,
    #[serde(default)]
    environment: Option<String>,
}

pub fn is_db_lock_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<deve_core::ledger::LocalAuthorityError>(),
            Some(deve_core::ledger::LocalAuthorityError::Busy(_))
        )
    })
}

pub fn dump(ledger_dir: &Path, path: &str, repo_name: Option<&str>) -> Result<DumpResponse> {
    let base = main_base_url(ledger_dir)?;
    let client = local_client()?;
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/dump"))
            .query(&[("path", path)])
            .query(&repo_query(repo_name));
        let response = with_proxy_auth(response, &client, &base)
            .await?
            .send()
            .await?;
        parse_json(response).await
    })
}

pub fn export(ledger_dir: &Path, repo_name: Option<&str>) -> Result<Vec<ExportEntry>> {
    let base = main_base_url(ledger_dir)?;
    let client = local_client()?;
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/export"))
            .query(&repo_query(repo_name));
        let response = with_proxy_auth(response, &client, &base)
            .await?
            .send()
            .await?;
        parse_json(response).await
    })
}

pub fn node_check(
    ledger_dir: &Path,
    repo_name: Option<&str>,
    repair: bool,
) -> Result<Vec<NodeCheckResponse>> {
    let base = main_base_url(ledger_dir)?;
    let client = local_client()?;
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/node-check"))
            .query(&[("repair", repair)])
            .query(&repo_query(repo_name));
        let response = with_proxy_auth(response, &client, &base)
            .await?
            .send()
            .await?;
        parse_json(response).await
    })
}

pub fn projection_check(
    ledger_dir: &Path,
    repo_name: Option<&str>,
) -> Result<Vec<ProjectionCheckResponse>> {
    let base = main_base_url(ledger_dir)?;
    let client = local_client()?;
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/projection-check"))
            .query(&repo_query(repo_name));
        let response = with_proxy_auth(response, &client, &base)
            .await?
            .send()
            .await?;
        parse_json(response).await
    })
}

pub fn sc_status(ledger_dir: &Path, repo_name: Option<&str>) -> Result<Vec<ScStatusResponse>> {
    let base = main_base_url(ledger_dir)?;
    let client = local_client()?;
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/sc-status"))
            .query(&repo_query(repo_name));
        let response = with_proxy_auth(response, &client, &base)
            .await?
            .send()
            .await?;
        parse_json(response).await
    })
}

pub fn git_status(ledger_dir: &Path, repo_name: Option<&str>) -> Result<Vec<GitStatusResponse>> {
    let base = main_base_url(ledger_dir)?;
    let client = local_client()?;
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/git-status"))
            .query(&repo_query(repo_name));
        let response = with_proxy_auth(response, &client, &base)
            .await?
            .send()
            .await?;
        parse_json(response).await
    })
}

pub(crate) fn main_base_url(ledger_dir: &Path) -> Result<String> {
    let port = match read_main_port_hint(ledger_dir)? {
        Some(port) => port,
        None => block_on_safe(detect_main_port())?,
    };
    Ok(format!("http://127.0.0.1:{port}"))
}

fn read_main_port_hint(ledger_dir: &Path) -> Result<Option<u16>> {
    Ok(read_main_owner_hint(ledger_dir)?.map(|hint| hint.main_port))
}

fn read_main_owner_hint(ledger_dir: &Path) -> Result<Option<LocalCliOwnerHint>> {
    let path = ledger_dir.join(".host").join("main_port");
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Ok(None),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("Failed to stat main port hint file: {:?}", path));
        }
    }
    let raw = std::fs::read(&path)
        .with_context(|| format!("Failed to read main port hint file: {:?}", path))?;
    let hint: LocalCliOwnerHint = serde_json::from_slice(&raw)
        .with_context(|| format!("Invalid main port owner hint in {path:?}"))?;
    if !hint.is_valid() {
        return Err(anyhow!("Invalid main port owner hint in {path:?}"));
    }
    Ok(Some(hint))
}

async fn detect_main_port() -> Result<u16> {
    let client = local_client()?;
    for port in candidate_ports() {
        let url = format!("http://127.0.0.1:{port}/api/node/role");
        let Ok(response) = client.get(&url).send().await else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let Ok(role) = response.json::<NodeRoleResponse>().await else {
            continue;
        };
        if let Some(main_port) = trusted_main_port(&role, port) {
            return Ok(main_port);
        }
    }
    Err(anyhow!("Main process not detected on localhost"))
}

async fn with_proxy_auth(
    request: RequestBuilder,
    client: &Client,
    base: &str,
) -> Result<RequestBuilder> {
    let Some(cookie) = proxy_auth_cookie(client, base).await? else {
        return Ok(request);
    };
    Ok(request.header(header::COOKIE, cookie))
}

async fn proxy_auth_cookie(client: &Client, base: &str) -> Result<Option<String>> {
    let role = fetch_node_role(client, base).await.ok();
    let config = match proxy_auth_config(role.as_ref().and_then(|role| role.environment.as_deref()))
    {
        Ok(config) => config,
        Err(_) => return Ok(None),
    };
    let token = deve_core::security::auth::jwt::issue_token(
        &config.secret,
        &config.username,
        config.token_version,
    )
    .context("Failed to issue live-proxy auth token")?;
    Ok(Some(format!("token={token}")))
}

async fn fetch_node_role(client: &Client, base: &str) -> Result<NodeRoleResponse> {
    let response = client
        .get(format!("{base}/api/node/role"))
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json::<NodeRoleResponse>().await?)
}

fn proxy_auth_config(role_environment: Option<&str>) -> Result<AuthConfig> {
    if let Ok(config) =
        AuthConfig::from_env_with_runtime_environment(RuntimeEnvironment::from_env())
    {
        return Ok(config);
    }
    if role_environment.is_some_and(|environment| environment.eq_ignore_ascii_case("development")) {
        return AuthConfig::dev_default();
    }
    Err(anyhow!(
        "Live proxy authentication requires AUTH_SECRET/AUTH_PASS in this CLI environment"
    ))
}

pub(crate) fn local_client() -> Result<Client> {
    Client::builder()
        .no_proxy()
        .build()
        .context("Failed to build localhost proxy client")
}

pub(crate) struct LocalCliProxySession {
    client: Client,
    base: String,
    bearer: String,
}

impl LocalCliProxySession {
    pub(crate) fn post(&self, path: &str) -> RequestBuilder {
        self.client
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.bearer)
    }
}

pub(crate) async fn authenticated_session(
    ledger_dir: &Path,
    auth_user: Option<String>,
    auth_password_stdin: bool,
) -> Result<LocalCliProxySession> {
    let username = auth_user.ok_or_else(|| {
        anyhow!("--auth-user is required when the owner server holds the repo DB")
    })?;
    if !auth_password_stdin {
        return Err(anyhow!(
            "--auth-password-stdin is required when the owner server holds the repo DB"
        ));
    }
    let hint = read_main_owner_hint(ledger_dir)?
        .ok_or_else(|| anyhow!("Local CLI owner process hint is missing"))?;
    let base = format!("http://127.0.0.1:{}", hint.main_port);
    let client = local_client()?;
    let role = fetch_node_role(&client, &base).await?;
    if !trusted_owner_endpoint(&hint, &role) {
        return Err(anyhow!("Main process endpoint identity mismatch"));
    }
    let password = read_password_from_stdin()?;
    let response = client
        .post(format!("{base}/api/auth/login"))
        .json(&LoginRequest { username, password })
        .send()
        .await
        .context("Local CLI operator login failed")?;
    let status = response.status();
    let tokens = response
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
        return Err(anyhow!(auth_code_name(
            body.code.unwrap_or(AuthErrorCode::InternalError)
        )));
    }
    if tokens.len() != 1 {
        return Err(anyhow!("AUTH_TOKEN_MISSING"));
    }
    Ok(LocalCliProxySession {
        client,
        base,
        bearer: tokens.into_iter().next().expect("single token checked"),
    })
}

fn trusted_owner_endpoint(hint: &LocalCliOwnerHint, role: &NodeRoleResponse) -> bool {
    trusted_main_port(role, hint.main_port) == Some(hint.main_port)
        && role.host_peer_id.as_deref() == Some(hint.host_peer_id.as_str())
        && role.runtime_incarnation == Some(hint.runtime_incarnation)
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
        return Err(anyhow!("Operator password from stdin is empty"));
    }
    Ok(password)
}

fn extract_token_cookie(value: &str) -> Option<String> {
    let pair = value.split(';').next()?.trim();
    let token = pair.strip_prefix("token=")?;
    (!token.is_empty()).then(|| token.to_string())
}

pub(crate) fn auth_code_name(code: AuthErrorCode) -> &'static str {
    match code {
        AuthErrorCode::InvalidPassword => "AUTH_INVALID_PASSWORD",
        AuthErrorCode::TokenExpired => "AUTH_TOKEN_EXPIRED",
        AuthErrorCode::TokenMissing => "AUTH_TOKEN_MISSING",
        AuthErrorCode::RateLimited => "AUTH_RATE_LIMITED",
        AuthErrorCode::CsrfMismatch => "AUTH_CSRF_MISMATCH",
        AuthErrorCode::InternalError => "AUTH_INTERNAL_ERROR",
    }
}

pub(crate) fn decode_auth_rejection(bytes: &[u8]) -> Result<&'static str> {
    let response = serde_json::from_slice::<AuthErrorResponse>(bytes)
        .map_err(|_| anyhow!("AUTH_INTERNAL_ERROR"))?;
    Ok(auth_code_name(response.code))
}

fn trusted_main_port(role: &NodeRoleResponse, probed_port: u16) -> Option<u16> {
    match role.role.as_str() {
        "main" | "native-main" if role.ws_port == probed_port && role.main_port == probed_port => {
            Some(probed_port)
        }
        "proxy"
            if role.ws_port == probed_port
                && role.main_port != 0
                && role.main_port != probed_port =>
        {
            Some(role.main_port)
        }
        _ => None,
    }
}

async fn parse_json<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    if status.is_success() {
        return Ok(response.json::<T>().await?);
    }
    let body = response
        .text()
        .await
        .unwrap_or_else(|err| format!("<failed to read error body: {err}>"));
    Err(anyhow!("Proxy request failed ({status}): {body}"))
}

fn repo_query(repo_name: Option<&str>) -> Vec<(&'static str, String)> {
    repo_name
        .map(|name| vec![("repo_name", name.to_string())])
        .unwrap_or_default()
}

fn candidate_ports() -> Vec<u16> {
    let mut ports = vec![DEFAULT_MAIN_PORT];
    for port in DEFAULT_MAIN_PORT.saturating_sub(2)..=DEFAULT_MAIN_PORT + 4 {
        if !ports.contains(&port) {
            ports.push(port);
        }
    }
    ports
}

pub(crate) fn block_on_safe<F, T>(fut: F) -> T
where
    F: Future<Output = T>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

#[cfg(test)]
mod tests;
