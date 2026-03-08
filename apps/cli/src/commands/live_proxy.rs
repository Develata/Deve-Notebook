use crate::admin_api::{DumpResponse, ExportEntry, NodeCheckResponse};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use std::future::Future;
use std::path::Path;

const DEFAULT_MAIN_PORT: u16 = 3001;

#[derive(Debug, Deserialize)]
struct NodeRoleResponse {
    main_port: u16,
}

pub fn is_db_lock_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        let text = cause.to_string();
        text.contains("Database already open") || text.contains("Cannot acquire lock")
    })
}

pub fn dump(ledger_dir: &Path, path: &str, repo_name: Option<&str>) -> Result<DumpResponse> {
    let base = main_base_url(ledger_dir)?;
    let client = Client::new();
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/dump"))
            .query(&[("path", path)])
            .query(&repo_query(repo_name))
            .send()
            .await?;
        parse_json(response).await
    })
}

pub fn export(ledger_dir: &Path, repo_name: Option<&str>) -> Result<Vec<ExportEntry>> {
    let base = main_base_url(ledger_dir)?;
    let client = Client::new();
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/export"))
            .query(&repo_query(repo_name))
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
    let client = Client::new();
    block_on_safe(async move {
        let response = client
            .get(format!("{base}/api/admin/node-check"))
            .query(&[("repair", repair)])
            .query(&repo_query(repo_name))
            .send()
            .await?;
        parse_json(response).await
    })
}

fn main_base_url(ledger_dir: &Path) -> Result<String> {
    let port = read_main_port_hint(ledger_dir)
        .or_else(|| block_on_safe(detect_main_port()).ok())
        .ok_or_else(|| anyhow!("Main process not detected on localhost"))?;
    Ok(format!("http://127.0.0.1:{port}"))
}

fn read_main_port_hint(ledger_dir: &Path) -> Option<u16> {
    let path = ledger_dir.join(".host").join("main_port");
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

async fn detect_main_port() -> Result<u16> {
    let client = Client::new();
    for port in candidate_ports() {
        let url = format!("http://127.0.0.1:{port}/api/node/role");
        let Ok(response) = client.get(&url).send().await else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let role = response.json::<NodeRoleResponse>().await?;
        return Ok(if role.main_port == 0 {
            port
        } else {
            role.main_port
        });
    }
    Err(anyhow!("Main process not detected on localhost"))
}

async fn parse_json<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    if status.is_success() {
        return Ok(response.json::<T>().await?);
    }
    let body = response.text().await.unwrap_or_default();
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

fn block_on_safe<F, T>(fut: F) -> T
where
    F: Future<Output = T>,
{
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}
