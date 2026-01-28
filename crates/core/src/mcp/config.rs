// crates/core/src/mcp/config.rs
//! # MCP 配置结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpServerKind {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpServerConfig {
    Local {
        name: String,
        command: String,
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        retries: Option<u32>,
        #[serde(default)]
        backoff_ms: Option<u64>,
    },
    Remote {
        name: String,
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        retries: Option<u32>,
        #[serde(default)]
        backoff_ms: Option<u64>,
    },
    RemoteSse {
        name: String,
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        timeout_ms: Option<u64>,
        #[serde(default)]
        retries: Option<u32>,
        #[serde(default)]
        backoff_ms: Option<u64>,
    },
}

impl McpServerConfig {
    pub fn name(&self) -> &str {
        match self {
            Self::Local { name, .. } => name,
            Self::Remote { name, .. } => name,
            Self::RemoteSse { name, .. } => name,
        }
    }

    pub fn kind(&self) -> McpServerKind {
        match self {
            Self::Local { .. } => McpServerKind::Local,
            Self::Remote { .. } | Self::RemoteSse { .. } => McpServerKind::Remote,
        }
    }

    pub fn timeout_ms(&self, default_ms: u64) -> u64 {
        match self {
            Self::Local { timeout_ms, .. }
            | Self::Remote { timeout_ms, .. }
            | Self::RemoteSse { timeout_ms, .. } => timeout_ms.unwrap_or(default_ms),
        }
    }

    pub fn retries(&self, default_retries: u32) -> u32 {
        match self {
            Self::Local { retries, .. }
            | Self::Remote { retries, .. }
            | Self::RemoteSse { retries, .. } => retries.unwrap_or(default_retries),
        }
    }

    pub fn backoff_ms(&self, default_ms: u64) -> u64 {
        match self {
            Self::Local { backoff_ms, .. }
            | Self::Remote { backoff_ms, .. }
            | Self::RemoteSse { backoff_ms, .. } => backoff_ms.unwrap_or(default_ms),
        }
    }
}
