//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime

use super::*;
use crate::server::ai_chat::settings::{ProviderProtocol, ProviderSettingsSnapshot};
use serde_json::json;

fn settings(provider: ProviderProtocol) -> ProviderSettingsSnapshot {
    ProviderSettingsSnapshot {
        provider,
        base_url: "https://provider.example/v1".into(),
        api_key: "fixture-secret".into(),
        model: "model-a".into(),
        max_tokens: 1024,
        revision: 7,
    }
}

fn history() -> Vec<Value> {
    vec![
        json!({"role":"system","content":"rules"}),
        json!({"role":"user","content":"hello"}),
    ]
}

#[test]
fn openai_chat_completions_request_and_stream_exact() {
    let prepared = prepare(
        &settings(ProviderProtocol::OpenaiChatCompletions),
        history(),
    )
    .unwrap();
    assert_eq!(
        prepared.endpoint,
        "https://provider.example/v1/chat/completions"
    );
    assert_eq!(prepared.body["messages"][0]["role"], "system");
    assert_eq!(prepared.body["max_tokens"], 1024);
    assert!(prepared.body.get("instructions").is_none());
}

#[test]
fn openai_responses_request_and_stream_exact() {
    let prepared = prepare(&settings(ProviderProtocol::OpenaiResponses), history()).unwrap();
    assert_eq!(prepared.endpoint, "https://provider.example/v1/responses");
    assert_eq!(prepared.body["instructions"], "rules");
    assert_eq!(prepared.body["input"][0]["role"], "user");
    assert_eq!(prepared.body["max_output_tokens"], 1024);
    assert!(matches!(
        parse(
            prepared.protocol,
            r#"{"type":"response.output_text.delta","delta":"hi"}"#
        )
        .unwrap(),
        ParsedSseEvent::ContentDelta(text) if text == "hi"
    ));
    assert!(matches!(
        parse(
            prepared.protocol,
            r#"{"type":"response.function_call_arguments.delta","delta":"{}"}"#
        )
        .unwrap(),
        ParsedSseEvent::ToolCallDelta
    ));
}

#[test]
fn anthropic_messages_request_and_stream_exact() {
    let prepared = prepare(&settings(ProviderProtocol::AnthropicMessages), history()).unwrap();
    assert_eq!(prepared.endpoint, "https://provider.example/v1/messages");
    assert_eq!(prepared.body["system"], "rules");
    assert_eq!(prepared.body["messages"][0]["role"], "user");
    assert!(matches!(
        parse(
            prepared.protocol,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}"#
        )
        .unwrap(),
        ParsedSseEvent::ContentDelta(text) if text == "hi"
    ));
    assert!(matches!(
        parse(
            prepared.protocol,
            r#"{"type":"content_block_start","content_block":{"type":"tool_use"}}"#
        )
        .unwrap(),
        ParsedSseEvent::ToolCallDelta
    ));
}

#[test]
fn provider_tool_refusal_and_unknown_content_fail_closed() {
    assert!(matches!(
        openai_responses::parse(
            r#"{"type":"response.output_item.added","item":{"type":"web_search_call"}}"#
        ),
        Ok(ParsedSseEvent::ToolCallDelta)
    ));
    assert!(
        openai_responses::parse(
            r#"{"type":"response.content_part.added","part":{"type":"refusal"}}"#
        )
        .is_err()
    );
    assert!(
        openai_responses::parse(
            r#"{"type":"response.content_part.added","part":{"type":"future_content"}}"#
        )
        .is_err()
    );
    assert!(
        anthropic::parse(r#"{"type":"message_delta","delta":{"stop_reason":"refusal"}}"#).is_err()
    );
    assert!(
        anthropic::parse(
            r#"{"type":"content_block_start","content_block":{"type":"future_block"}}"#
        )
        .is_err()
    );
}

#[test]
fn openai_responses_completion_requires_validated_response_output() {
    assert!(openai_responses::parse(r#"{"type":"response.completed"}"#).is_err());
    assert!(
        openai_responses::parse(
            r#"{"type":"response.completed","response":{"status":"completed"}}"#
        )
        .is_err()
    );
    assert!(
        openai_responses::parse(
            r#"{"type":"response.completed","response":{"status":"failed","output":[]}}"#
        )
        .is_err()
    );
    assert!(
        openai_responses::parse(
            r#"{"type":"response.completed","response":{"status":"completed","output":[{"type":"message"}]}}"#
        )
        .is_err()
    );
    assert!(matches!(
        openai_responses::parse(
            r#"{"type":"response.completed","response":{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"done"}]}]}}"#
        ),
        Ok(ParsedSseEvent::Finished(reason)) if reason == "completed"
    ));
}
