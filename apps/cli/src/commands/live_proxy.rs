//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 08_auth#jwt-cookie-contract
//!   - 14_commands#cli-commands
//!   - 18_release#runtime-observability

use crate::admin_api::{
    DumpResponse, ExportEntry, GitStatusResponse, NodeCheckResponse, ProjectionCheckResponse,
    ScStatusResponse,
};
use anyhow::{Context, Result, anyhow};
use deve_core::config::RuntimeEnvironment;
use deve_core::security::AuthConfig;
use reqwest::{Client, RequestBuilder, header};
use serde::Deserialize;
use std::future::Future;
use std::path::Path;

const DEFAULT_MAIN_PORT: u16 = 3001;

#[derive(Debug, Deserialize)]
struct NodeRoleResponse {
    role: String,
    ws_port: u16,
    main_port: u16,
    #[serde(default)]
    environment: Option<String>,
}

pub fn is_db_lock_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        let text = cause.to_string();
        text.contains("Database already open") || text.contains("Cannot acquire lock")
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
    let path = ledger_dir.join(".host").join("main_port");
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Ok(None),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("Failed to stat main port hint file: {:?}", path));
        }
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read main port hint file: {:?}", path))?;
    let port = raw.trim().parse().with_context(|| {
        format!(
            "Invalid main port hint in {:?}: expected u16, got {:?}",
            path, raw
        )
    })?;
    Ok(Some(port))
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
