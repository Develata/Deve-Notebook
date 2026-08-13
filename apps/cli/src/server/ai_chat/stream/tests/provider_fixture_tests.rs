//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime

use super::super::*;
use crate::server::ai_chat::providers::prepare;
use crate::server::ai_chat::settings::{ProviderProtocol, ProviderSettingsSnapshot};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn run_fixture(
    protocol: ProviderProtocol,
    events: &str,
) -> (
    String,
    deve_core::plugin::runtime::chat_stream::ChatStreamResponse,
) {
    let (request, response) = run_fixture_result(protocol, events).await;
    (request, response.expect("provider fixture must complete"))
}

async fn run_fixture_result(
    protocol: ProviderProtocol,
    events: &str,
) -> (
    String,
    anyhow::Result<deve_core::plugin::runtime::chat_stream::ChatStreamResponse>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let events = events.to_string();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let header_end = loop {
            let mut chunk = [0_u8; 1024];
            let read = socket.read(&mut chunk).await.unwrap();
            assert!(read > 0, "provider fixture request ended before headers");
            request.extend_from_slice(&chunk[..read]);
            if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
        };
        let headers = String::from_utf8(request[..header_end].to_vec()).unwrap();
        let content_length = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok());
        let chunked = headers.lines().any(|line| {
            line.split_once(':').is_some_and(|(name, value)| {
                name.eq_ignore_ascii_case("transfer-encoding")
                    && value.trim().eq_ignore_ascii_case("chunked")
            })
        });
        while content_length.is_some_and(|length| request.len() < header_end + length)
            || (chunked
                && !request[header_end..]
                    .windows(7)
                    .any(|tail| tail == b"\r\n0\r\n\r\n"))
        {
            let mut chunk = [0_u8; 1024];
            let read = socket.read(&mut chunk).await.unwrap();
            assert!(read > 0, "provider fixture request ended before body");
            request.extend_from_slice(&chunk[..read]);
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{}",
            events.len(),
            events
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        let body = if chunked {
            decode_chunked(&request[header_end..])
        } else {
            request[header_end..header_end + content_length.unwrap_or_default()].to_vec()
        };
        format!("{headers}{}", String::from_utf8(body).unwrap())
    });

    let settings = ProviderSettingsSnapshot {
        provider: protocol,
        base_url: format!("http://{address}/v1"),
        api_key: "fixture-secret".into(),
        model: "model-a".into(),
        max_tokens: 128,
        revision: 1,
    };
    let prepared = prepare(
        &settings,
        vec![
            json!({"role":"system","content":"rules"}),
            json!({"role":"user","content":"hello"}),
        ],
    )
    .unwrap();
    let sink = deve_core::plugin::runtime::chat_stream::ChatStreamSink::new(|_| {});
    let response = execute_stream("req-fixture", &settings, prepared, &sink).await;
    (server.await.unwrap(), response)
}

fn decode_chunked(mut input: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::new();
    loop {
        let line_end = input.windows(2).position(|pair| pair == b"\r\n").unwrap();
        let size = usize::from_str_radix(
            std::str::from_utf8(&input[..line_end])
                .unwrap()
                .split(';')
                .next()
                .unwrap(),
            16,
        )
        .unwrap();
        input = &input[line_end + 2..];
        if size == 0 {
            return decoded;
        }
        decoded.extend_from_slice(&input[..size]);
        input = &input[size + 2..];
    }
}

#[tokio::test]
async fn exact_provider_http_fixtures_stream_plain_text() {
    let (chat_request, chat_response) = run_fixture(
        ProviderProtocol::OpenaiChatCompletions,
        "data: {\"choices\":[{\"delta\":{\"content\":\"chat\"}}]}\n\ndata: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}]}\n\ndata: [DONE]\n\n",
    )
    .await;
    assert!(chat_request.starts_with("POST /v1/chat/completions HTTP/1.1"));
    assert!(
        chat_request
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture-secret")
    );
    assert!(chat_request.contains("\"messages\""));
    assert_text(chat_response, "chat");

    let (responses_request, responses_response) = run_fixture(
        ProviderProtocol::OpenaiResponses,
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"responses\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"responses\"}]}]}}\n\n",
    )
    .await;
    assert!(responses_request.starts_with("POST /v1/responses HTTP/1.1"));
    assert!(
        responses_request
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture-secret")
    );
    assert!(responses_request.contains("\"instructions\":\"rules\""));
    assert_text(responses_response, "responses");

    let (anthropic_request, anthropic_response) = run_fixture(
        ProviderProtocol::AnthropicMessages,
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"anthropic\"}}\n\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
    )
    .await;
    assert!(anthropic_request.starts_with("POST /v1/messages HTTP/1.1"));
    let anthropic_headers = anthropic_request.to_ascii_lowercase();
    assert!(anthropic_headers.contains("x-api-key: fixture-secret"));
    assert!(anthropic_headers.contains("anthropic-version: 2023-06-01"));
    assert!(anthropic_request.contains("\"system\":\"rules\""));
    assert_text(anthropic_response, "anthropic");
}

#[tokio::test]
async fn truncated_provider_streams_fail_before_success_projection() {
    let fixtures = [
        (
            ProviderProtocol::OpenaiChatCompletions,
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
        ),
        (
            ProviderProtocol::OpenaiResponses,
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n\n",
        ),
        (
            ProviderProtocol::AnthropicMessages,
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n",
        ),
    ];
    for (protocol, events) in fixtures {
        let (_, result) = run_fixture_result(protocol, events).await;
        assert_eq!(
            result
                .expect_err("truncated provider stream must fail")
                .to_string(),
            "Native AI provider stream ended before a valid terminal event"
        );
    }
}

#[tokio::test]
async fn openai_done_without_protocol_terminal_fails_closed() {
    let (_, result) = run_fixture_result(
        ProviderProtocol::OpenaiChatCompletions,
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\ndata: [DONE]\n\n",
    )
    .await;
    assert_eq!(
        result
            .expect_err("[DONE] must not replace a Chat stop terminal")
            .to_string(),
        "Native AI provider stream ended before a valid terminal event"
    );
}

fn assert_text(
    response: deve_core::plugin::runtime::chat_stream::ChatStreamResponse,
    expected: &str,
) {
    match response {
        deve_core::plugin::runtime::chat_stream::ChatStreamResponse::Text { content } => {
            assert_eq!(content, expected)
        }
        deve_core::plugin::runtime::chat_stream::ChatStreamResponse::ToolCalls { .. } => {
            panic!("provider fixture returned tool calls")
        }
    }
}
