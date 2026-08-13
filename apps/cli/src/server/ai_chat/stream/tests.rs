use super::*;
use deve_core::plugin::runtime::chat_stream::ToolCallInfo;
use deve_core::protocol::ServerMessage;
use std::sync::{Arc, Mutex};

mod provider_fixture_tests;

#[test]
fn native_ai_http_client_creation_is_result_based() {
    let client = get_http_client().expect("native AI HTTP client should build");
    let _ = client.clone();
}

fn provider_settings(provider: ProviderProtocol) -> ProviderSettingsSnapshot {
    ProviderSettingsSnapshot {
        provider,
        base_url: "https://provider.example/v1".to_string(),
        api_key: "fixture-secret".to_string(),
        model: "model-a".to_string(),
        max_tokens: 1024,
        revision: 1,
    }
}

fn prepared(protocol: ProviderProtocol) -> PreparedProviderRequest {
    PreparedProviderRequest {
        endpoint: "https://provider.example/v1/stream".to_string(),
        body: serde_json::json!({"stream": true}),
        protocol,
    }
}

#[test]
fn openai_provider_request_owns_bearer_auth() {
    let client = reqwest::Client::new();
    let settings = provider_settings(ProviderProtocol::OpenaiResponses);
    let request = build_provider_request(&client, &settings, &prepared(settings.provider))
        .build()
        .unwrap();
    assert_eq!(
        request.headers()[reqwest::header::AUTHORIZATION],
        "Bearer fixture-secret"
    );
    assert!(!request.headers().contains_key("x-api-key"));
}

#[test]
fn anthropic_provider_request_owns_exact_auth_headers() {
    let client = reqwest::Client::new();
    let settings = provider_settings(ProviderProtocol::AnthropicMessages);
    let request = build_provider_request(&client, &settings, &prepared(settings.provider))
        .build()
        .unwrap();
    assert_eq!(request.headers()["x-api-key"], "fixture-secret");
    assert_eq!(request.headers()["anthropic-version"], "2023-06-01");
    assert!(
        !request
            .headers()
            .contains_key(reqwest::header::AUTHORIZATION)
    );
}

#[test]
fn finalize_stream_response_rejects_provider_tool_calls() {
    let err = finalize_stream_response(
        "partial".to_string(),
        vec![ToolCallInfo {
            id: "call_1".to_string(),
            name: "write_file".to_string(),
            arguments: "{}".to_string(),
        }],
    )
    .expect_err("native AI must fail closed on provider tool calls");

    assert_eq!(err.to_string(), NATIVE_AI_TOOL_CALLS_DISABLED_ERROR);
}

#[test]
fn finalize_stream_response_accepts_plain_text() {
    let response = finalize_stream_response("hello".to_string(), vec![])
        .expect("plain text response should be accepted");

    match response {
        ChatStreamResponse::Text { content } => assert_eq!(content, "hello"),
        ChatStreamResponse::ToolCalls { .. } => panic!("unexpected tool response"),
    }
}

#[test]
fn provider_tool_call_rejection_does_not_send_finish_chunk() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let sent_for_sink = sent.clone();
    let sink = ChatStreamSink::new(move |msg| {
        sent_for_sink.lock().unwrap().push(msg);
    });

    let err = finish_stream_response(
        "req-1",
        "partial".to_string(),
        vec![ToolCallInfo {
            id: "call_1".to_string(),
            name: "write_file".to_string(),
            arguments: "{}".to_string(),
        }],
        Some("tool_calls".to_string()),
        &sink,
    )
    .expect_err("native AI must reject provider tool calls before finish");

    assert_eq!(err.to_string(), NATIVE_AI_TOOL_CALLS_DISABLED_ERROR);
    assert!(sent.lock().unwrap().is_empty());
}

#[test]
fn provider_tool_call_delta_is_rejected_immediately() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let sent_for_sink = sent.clone();
    let sink = ChatStreamSink::new(move |msg| {
        sent_for_sink.lock().unwrap().push(msg);
    });
    let mut output = "partial".to_string();
    let mut finish_reason = None;

    let err = apply_sse_event(
        "req-1",
        ParsedSseEvent::ToolCallDelta,
        &mut output,
        &mut finish_reason,
        &sink,
    )
    .expect_err("native AI must reject provider tool call deltas immediately");

    assert_eq!(err.to_string(), NATIVE_AI_TOOL_CALLS_DISABLED_ERROR);
    assert_eq!(output, "partial");
    assert_eq!(finish_reason, None);
    assert!(sent.lock().unwrap().is_empty());
}

#[test]
fn provider_tool_call_payload_is_rejected_before_content_chunk() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let sent_for_sink = sent.clone();
    let sink = ChatStreamSink::new(move |msg| {
        sent_for_sink.lock().unwrap().push(msg);
    });
    let event = crate::server::ai_chat::sse_parser::parse_sse_message(
        r#"{"choices":[{"delta":{"content":"unsafe partial","tool_calls":[{"index":0}]}}]}"#,
    )
    .expect("tool call payload should parse");
    let mut output = String::new();
    let mut finish_reason = None;

    let err = apply_sse_event("req-1", event, &mut output, &mut finish_reason, &sink)
        .expect_err("native AI must reject tool call payload before forwarding content");

    assert_eq!(err.to_string(), NATIVE_AI_TOOL_CALLS_DISABLED_ERROR);
    assert_eq!(output, "");
    assert_eq!(finish_reason, None);
    assert!(sent.lock().unwrap().is_empty());
}

#[test]
fn plain_text_finish_sends_finish_chunk_after_validation() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let sent_for_sink = sent.clone();
    let sink = ChatStreamSink::new(move |msg| {
        sent_for_sink.lock().unwrap().push(msg);
    });

    finish_stream_response(
        "req-1",
        "hello".to_string(),
        vec![],
        Some("stop".to_string()),
        &sink,
    )
    .expect("plain text response should finish normally");

    let sent = sent.lock().unwrap();
    assert!(matches!(
        sent.as_slice(),
        [ServerMessage::ChatChunk {
            req_id,
            delta: None,
            finish_reason: Some(reason),
        }] if req_id == "req-1" && reason == "stop"
    ));
}

#[test]
fn missing_provider_terminal_event_is_rejected() {
    let sink = ChatStreamSink::new(|_| {});
    let error = finish_stream_response("req-1", "partial".to_string(), vec![], None, &sink)
        .expect_err("an incomplete provider stream must not become a successful response");

    assert_eq!(
        error.to_string(),
        "Native AI provider stream ended before a valid terminal event"
    );
}
