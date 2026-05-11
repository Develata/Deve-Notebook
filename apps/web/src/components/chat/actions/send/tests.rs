use super::{
    MAX_CHAT_CONTEXT_CHARS, MAX_CHAT_HISTORY_MESSAGES, bounded_chat_history, build_chat_context,
    truncate_markdown_context,
};
use crate::components::chat::slash_commands::ChatSessionMode;
use crate::hooks::use_core::ChatMessage;

#[test]
fn markdown_context_is_bounded_on_char_boundaries() {
    let content = "é".repeat(MAX_CHAT_CONTEXT_CHARS + 3);
    let bounded = truncate_markdown_context(content);
    assert_eq!(bounded.chars().count(), MAX_CHAT_CONTEXT_CHARS);
}

#[test]
fn markdown_context_keeps_short_content() {
    let content = "# Note\n\nbody".to_string();
    assert_eq!(truncate_markdown_context(content.clone()), content);
}

#[test]
fn chat_context_includes_current_doc_markdown_selection_and_mode() {
    let context = build_chat_context(
        "default/note.md".to_string(),
        "# Note\n\nbody".to_string(),
        serde_json::json!({"from": 1, "to": 4}),
        ChatSessionMode::Build,
    );

    assert_eq!(context["current_file"], "default/note.md");
    assert_eq!(context["current_markdown"], "# Note\n\nbody");
    assert_eq!(
        context["selection"],
        serde_json::json!({"from": 1, "to": 4})
    );
    assert_eq!(context["chat_mode"], "build");
}

#[test]
fn chat_context_allows_empty_doc_context_without_inventing_path() {
    let context = build_chat_context(
        String::new(),
        String::new(),
        serde_json::Value::Null,
        ChatSessionMode::Plan,
    );

    assert_eq!(context["current_file"], "");
    assert_eq!(context["current_markdown"], "");
    assert_eq!(context["selection"], serde_json::Value::Null);
    assert_eq!(context["chat_mode"], "plan");
}

#[test]
fn bounded_history_keeps_recent_user_and_assistant_turns() {
    let history = bounded_chat_history(vec![
        ChatMessage {
            role: "system".into(),
            content: "ignored".into(),
            req_id: None,
            ts_ms: 0,
        },
        ChatMessage {
            role: "user".into(),
            content: "first".into(),
            req_id: None,
            ts_ms: 0,
        },
        ChatMessage {
            role: "assistant".into(),
            content: "second".into(),
            req_id: Some("req-1".into()),
            ts_ms: 0,
        },
    ]);

    assert_eq!(
        history,
        vec![
            serde_json::json!({"role": "user", "content": "first"}),
            serde_json::json!({"role": "assistant", "content": "second"}),
        ]
    );
}

#[test]
fn bounded_history_limits_message_count() {
    let messages = (0..12)
        .map(|idx| ChatMessage {
            role: if idx % 2 == 0 { "user" } else { "assistant" }.into(),
            content: format!("m{idx}"),
            req_id: None,
            ts_ms: 0,
        })
        .collect();

    let history = bounded_chat_history(messages);

    assert_eq!(history.len(), MAX_CHAT_HISTORY_MESSAGES);
    assert_eq!(history[0]["content"], "m4");
    assert_eq!(history[7]["content"], "m11");
}
