use super::*;
use deve_core::plugin::runtime::chat_stream::ToolCallInfo;
use deve_core::protocol::ServerMessage;
use std::sync::{Arc, Mutex};

fn provider_request_builder() -> reqwest::RequestBuilder {
    reqwest::Client::new()
        .post("https://api.example.test/v1/chat/completions")
        .bearer_auth("api-key")
        .json(&serde_json::json!({ "model": "test" }))
}

#[test]
fn native_ai_http_client_creation_is_result_based() {
    let client = get_http_client().expect("native AI HTTP client should build");
    let _ = client.clone();
}

#[test]
fn configured_headers_reject_reserved_request_headers() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer injected".to_string());

    let err = apply_configured_headers(provider_request_builder(), &headers)
        .expect_err("configured authorization header must fail closed");

    assert_eq!(
        err.to_string(),
        "AI custom headers must not include authorization, host, content-length, or transfer-encoding"
    );
}

#[test]
fn configured_headers_keep_provider_metadata_and_bearer_auth() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("OpenAI-Organization".to_string(), "org_test".to_string());
    headers.insert("X-Provider-Beta".to_string(), "enabled".to_string());

    let request = apply_configured_headers(provider_request_builder(), &headers)
        .expect("provider metadata headers should be accepted")
        .build()
        .expect("request should remain buildable");

    assert_eq!(
        request
            .headers()
            .get("OpenAI-Organization")
            .unwrap()
            .to_str()
            .unwrap(),
        "org_test"
    );
    assert_eq!(
        request
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer api-key"
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
    let event = parse_sse_message(
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
