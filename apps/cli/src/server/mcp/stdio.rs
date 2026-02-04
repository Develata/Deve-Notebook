// apps/cli/src/server/mcp/stdio.rs
//! # MCP Stdio Executor

use super::parse_tools;
use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{Result, anyhow};
use deve_core::mcp::{McpCallResult, McpExecutor, McpToolSpec};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

pub struct StdioExecutor {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    timeout_ms: u64,
    retries: u32,
    backoff_ms: u64,
}

impl StdioExecutor {
    pub fn new(
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        timeout_ms: u64,
        retries: u32,
        backoff_ms: u64,
    ) -> Self {
        Self {
            command,
            args,
            env,
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
        let payload = serde_json::to_string(&req)? + "\n";

        let mut attempt = 0;
        'retry: loop {
            attempt += 1;
            let mut cmd = Command::new(&self.command);
            cmd.args(&self.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped());
            for (k, v) in &self.env {
                cmd.env(k, v);
            }

            let mut child = cmd.spawn()?;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(payload.as_bytes())?;
            }

            let start = std::time::Instant::now();
            loop {
                if let Some(_status) = child.try_wait()? {
                    break;
                }
                if start.elapsed().as_millis() as u64 > self.timeout_ms {
                    let _ = child.kill();
                    if attempt <= self.retries {
                        std::thread::sleep(std::time::Duration::from_millis(self.backoff_ms));
                        continue 'retry;
                    }
                    return Err(anyhow!("MCP stdio timeout"));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let mut output = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                stdout.read_to_string(&mut output)?;
            }

            let line = output
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .ok_or_else(|| anyhow!("Empty MCP response"))?;

            let resp: JsonRpcResponse = serde_json::from_str(line)?;
            if let Some(err) = resp.error {
                return Err(anyhow!("MCP error: {}", err));
            }
            return resp.result.ok_or_else(|| anyhow!("Missing MCP result"));
        }
    }
}

impl McpExecutor for StdioExecutor {
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
