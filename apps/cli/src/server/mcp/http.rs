// apps/cli/src/server/mcp/http.rs
//! # MCP HTTP Executor

use super::parse_tools;
use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{Result, anyhow};
use deve_core::mcp::{McpCallResult, McpExecutor, McpToolSpec};
use serde_json::Value;
use std::collections::HashMap;

pub struct HttpExecutor {
    url: String,
    headers: HashMap<String, String>,
    client: reqwest::Client,
    timeout_ms: u64,
    retries: u32,
    backoff_ms: u64,
}

impl HttpExecutor {
    pub fn new(
        url: String,
        headers: HashMap<String, String>,
        timeout_ms: u64,
        retries: u32,
        backoff_ms: u64,
    ) -> Self {
        Self {
            url,
            headers,
            client: reqwest::Client::new(),
            timeout_ms,
            retries,
            backoff_ms,
        }
    }

    fn call_rpc(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };

        let mut attempt = 0;
        loop {
            attempt += 1;
            let mut request = self.client.post(&self.url).json(&req);
            for (k, v) in &self.headers {
                request = request.header(k, v);
            }
            let resp = tokio::runtime::Handle::current().block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_millis(self.timeout_ms),
                    request.send(),
                )
                .await
            });

            let resp = match resp {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    if attempt <= self.retries {
                        std::thread::sleep(std::time::Duration::from_millis(self.backoff_ms));
                        continue;
                    }
                    return Err(anyhow!("MCP http error: {}", e));
                }
                Err(_) => {
                    if attempt <= self.retries {
                        std::thread::sleep(std::time::Duration::from_millis(self.backoff_ms));
                        continue;
                    }
                    return Err(anyhow!("MCP http timeout"));
                }
            };

            let val: JsonRpcResponse =
                tokio::runtime::Handle::current().block_on(async { resp.json().await })?;

            if let Some(err) = val.error {
                return Err(anyhow!("MCP error: {}", err));
            }
            return val.result.ok_or_else(|| anyhow!("Missing MCP result"));
        }
    }
}

impl McpExecutor for HttpExecutor {
    fn list_tools(&self) -> Result<Vec<McpToolSpec>> {
        let result = self.call_rpc("tools/list", None)?;
        parse_tools(result)
    }

    fn call_tool(&self, name: &str, args: Value) -> Result<McpCallResult> {
        let params = serde_json::json!({ "name": name, "arguments": args });
        let result = self.call_rpc("tools/call", Some(params))?;
        Ok(McpCallResult { content: result })
    }
}
