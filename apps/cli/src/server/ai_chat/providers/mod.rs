//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 15_settings#native-ai-provider-settings
//!
//! Exact peer adapters for supported provider request and stream protocols.

mod anthropic;
mod openai_responses;

use super::settings::{ProviderProtocol, ProviderSettingsSnapshot};
use super::types::ParsedSseEvent;
use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

pub(crate) struct PreparedProviderRequest {
    pub(crate) endpoint: String,
    pub(crate) body: Value,
    pub(crate) protocol: ProviderProtocol,
}

#[derive(Deserialize)]
struct HistoryMessage {
    role: String,
    content: String,
}

pub(crate) fn prepare(
    settings: &ProviderSettingsSnapshot,
    history: Vec<Value>,
) -> Result<PreparedProviderRequest> {
    let messages = history
        .into_iter()
        .map(|value| serde_json::from_value::<HistoryMessage>(value).map_err(anyhow::Error::from))
        .collect::<Result<Vec<_>>>()?;
    if messages.is_empty() {
        bail!("Chat history must not be empty");
    }
    for message in &messages {
        if !matches!(message.role.as_str(), "system" | "user" | "assistant") {
            bail!("Chat history contains an unsupported role");
        }
    }
    let body = match settings.provider {
        ProviderProtocol::OpenaiChatCompletions => json!({
            "model": settings.model,
            "messages": messages.iter().map(|message| json!({
                "role": message.role,
                "content": message.content,
            })).collect::<Vec<_>>(),
            "stream": true,
            "max_tokens": settings.max_tokens,
        }),
        ProviderProtocol::OpenaiResponses => openai_responses::request_body(settings, &messages),
        ProviderProtocol::AnthropicMessages => anthropic::request_body(settings, &messages),
    };
    Ok(PreparedProviderRequest {
        endpoint: settings.endpoint(),
        body,
        protocol: settings.provider,
    })
}

pub(crate) fn parse(protocol: ProviderProtocol, data: &str) -> Result<ParsedSseEvent, String> {
    match protocol {
        ProviderProtocol::OpenaiChatCompletions => super::sse_parser::parse_sse_message(data),
        ProviderProtocol::OpenaiResponses => openai_responses::parse(data),
        ProviderProtocol::AnthropicMessages => anthropic::parse(data),
    }
}

fn split_system(messages: &[HistoryMessage]) -> (Option<String>, Vec<Value>) {
    let mut system = Vec::new();
    let mut conversational = Vec::new();
    for message in messages {
        if message.role == "system" {
            system.push(message.content.as_str());
        } else {
            conversational.push(json!({
                "role": message.role,
                "content": message.content,
            }));
        }
    }
    let system = (!system.is_empty()).then(|| system.join("\n\n"));
    (system, conversational)
}

#[cfg(test)]
mod tests;
