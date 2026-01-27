// apps/cli/src/server/ai_chat.rs
//! # AI Chat Streaming (Server Runtime)
//!
//! Provides the concrete OpenAI-compatible streaming implementation
//! wired into the core Rhai host bridge.

use anyhow::{anyhow, Result};
use deve_core::plugin::runtime::chat_stream::{
    ChatStreamHandler, ChatStreamRequest, ChatStreamResponse, ChatStreamSink,
    ToolCallInfo, set_chat_stream_handler,
};
use futures::StreamExt;
use reqwest_eventsource::{Error as EventSourceError, Event, EventSource};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct ChatConfig {
    base_url: String,
    api_key: String,
    model: String,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
    #[serde(default)]
    headers: std::collections::HashMap<String, String>,
}

fn default_max_tokens() -> u32 {
    4096
}

pub fn init_chat_stream_handler() -> Result<()> {
    let handler = Arc::new(AiChatStreamHandler);
    set_chat_stream_handler(handler)
}

struct AiChatStreamHandler;

impl ChatStreamHandler for AiChatStreamHandler {
    fn stream(&self, request: ChatStreamRequest, sink: ChatStreamSink) -> Result<ChatStreamResponse> {
        let config: ChatConfig = serde_json::from_value(request.config)
            .map_err(|e| anyhow!("Invalid AI config: {}", e))?;

        if config.api_key.trim().is_empty() {
            return Err(anyhow!("Missing AI API key"));
        }

        let history = request
            .history
            .as_array()
            .ok_or_else(|| anyhow!("Chat history must be an array"))?
            .clone();

        let endpoint = format!(
            "{}/chat/completions",
            config.base_url.trim_end_matches('/')
        );

        let mut body = json!({
            "model": config.model,
            "messages": history,
            "stream": true,
            "max_tokens": config.max_tokens,
        });

        // Add tools if provided
        if let Some(tools) = &request.tools {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("tools".to_string(), tools.clone());
            }
        }

        let req_id = request.req_id.clone();
        tokio::runtime::Handle::current().block_on(async move {
            let client = reqwest::Client::new();
            let mut req = client
                .post(endpoint)
                .bearer_auth(&config.api_key)
                .json(&body);

            // Apply custom headers from config
            for (key, value) in &config.headers {
                req = req.header(key.as_str(), value.as_str());
            }

            let mut stream = EventSource::new(req)
                .map_err(|e| anyhow!("Failed to create SSE stream: {}", e))?;
            let mut output = String::new();
            let mut tool_calls: Vec<ToolCallInfo> = Vec::new();
            let mut current_tool_call: Option<(String, String, String)> = None; // (id, name, args)
            let mut finished = false;

            while let Some(event) = stream.next().await {
                match event {
                    Ok(Event::Open) => {}
                    Ok(Event::Message(message)) => {
                        if message.data == "[DONE]" {
                            if !finished {
                                sink.send_chunk(&req_id, None, Some("done".to_string()));
                                finished = true;
                            }
                            break;
                        }

                        let payload: serde_json::Value = serde_json::from_str(&message.data)
                            .map_err(|e| anyhow!("Invalid SSE payload: {}", e))?;
                        let choice = payload
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .ok_or_else(|| anyhow!("Missing choices in SSE payload"))?;

                        let delta = choice.get("delta");

                        // Handle content delta (normal text response)
                        if let Some(content) = delta
                            .and_then(|d| d.get("content"))
                            .and_then(|v| v.as_str())
                        {
                            output.push_str(content);
                            sink.send_chunk(&req_id, Some(content.to_string()), None);
                        }

                        // Handle tool_calls delta (function calling)
                        if let Some(tc_delta) = delta.and_then(|d| d.get("tool_calls")) {
                            if let Some(tc_arr) = tc_delta.as_array() {
                                for tc in tc_arr {
                                    let index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                    
                                    // New tool call starting
                                    if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                        // Save previous if exists
                                        if let Some((prev_id, prev_name, prev_args)) = current_tool_call.take() {
                                            tool_calls.push(ToolCallInfo {
                                                id: prev_id,
                                                name: prev_name,
                                                arguments: prev_args,
                                            });
                                        }
                                        let name = tc.get("function")
                                            .and_then(|f| f.get("name"))
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        current_tool_call = Some((id.to_string(), name, String::new()));
                                    }
                                    
                                    // Append arguments
                                    if let Some(args) = tc.get("function")
                                        .and_then(|f| f.get("arguments"))
                                        .and_then(|a| a.as_str())
                                    {
                                        if let Some((_, _, ref mut acc_args)) = current_tool_call {
                                            acc_args.push_str(args);
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                            // Finalize any pending tool call
                            if let Some((id, name, args)) = current_tool_call.take() {
                                tool_calls.push(ToolCallInfo { id, name, arguments: args });
                            }
                            
                            if !finished {
                                sink.send_chunk(&req_id, None, Some(reason.to_string()));
                                finished = true;
                            }
                            break;
                        }
                    }
                    Err(EventSourceError::StreamEnded) => break,
                    Err(err) => return Err(anyhow!("SSE stream error: {}", err)),
                }
            }

            // Finalize any pending tool call
            if let Some((id, name, args)) = current_tool_call.take() {
                tool_calls.push(ToolCallInfo { id, name, arguments: args });
            }

            // Return appropriate response type
            if !tool_calls.is_empty() {
                Ok(ChatStreamResponse::ToolCalls { calls: tool_calls })
            } else {
                Ok(ChatStreamResponse::Text { content: output })
            }
        })
    }
}
