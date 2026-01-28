// apps/cli/src/server/mcp/sse.rs
//! # MCP SSE Executor

use super::parse_tools;
use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Result};
use deve_core::mcp::{McpCallResult, McpExecutor, McpToolSpec};
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::collections::HashMap;

pub struct SseExecutor {
    url: String,
    headers: HashMap<String, String>,
    client: reqwest::Client,
    timeout_ms: u64,
    retries: u32,
    backoff_ms: u64,
}

impl SseExecutor {
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

            let result = tokio::runtime::Handle::current().block_on(async {
                let mut es = EventSource::new(request)
                    .map_err(|e| anyhow!("MCP SSE create error: {}", e))?;

                let timeout = tokio::time::sleep(std::time::Duration::from_millis(self.timeout_ms));
                tokio::pin!(timeout);

                loop {
                    tokio::select! {
                        _ = &mut timeout => return Err(anyhow!("MCP SSE timeout")),
                        msg = es.next() => {
                            match msg {
                                Some(Ok(Event::Message(m))) => {
                                    let resp: JsonRpcResponse = serde_json::from_str(&m.data)
                                        .map_err(|e| anyhow!("MCP SSE parse error: {}", e))?;
                                    if let Some(err) = resp.error {
                                        return Err(anyhow!("MCP error: {}", err));
                                    }
                                    return resp.result.ok_or_else(|| anyhow!("Missing MCP result"));
                                }
                                Some(Ok(_)) => continue,
                                Some(Err(e)) => return Err(anyhow!("MCP SSE error: {}", e)),
                                None => return Err(anyhow!("MCP SSE closed")),
                            }
                        }
                    }
                }
            });

            match result {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt <= self.retries {
                        std::thread::sleep(std::time::Duration::from_millis(self.backoff_ms));
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}

impl McpExecutor for SseExecutor {
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
