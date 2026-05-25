//! plan_ref:
//!   - 16_ai_agent#trusted-agent-bridge
//!
pub(super) fn extract_user_message(args: &[serde_json::Value]) -> String {
    let msg = if args.len() >= 2
        && let Some(s) = args[1].as_str()
    {
        s.to_string()
    } else if let Some(first) = args.first()
        && let Some(s) = first.as_str()
    {
        s.to_string()
    } else {
        return String::new();
    };

    if let Some(ctx) = args.get(2)
        && let Some(sel_text) = ctx
            .get("selection")
            .and_then(|s| s.get("text"))
            .and_then(|t| t.as_str())
        && !sel_text.is_empty()
    {
        return format!("{}\n\n---\nSelected code:\n```\n{}\n```", msg, sel_text);
    }

    msg
}
