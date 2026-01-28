// plugins/ai-chat/history.rhai

const MAX_HISTORY_ROUNDS = 10;
const MAX_TOTAL_CHARS = 24000;

// Invariant: history[0] is system prompt
fn compact_history(history) {
    let max_messages = MAX_HISTORY_ROUNDS * 2 + 1;
    while history.len() > max_messages { history.remove(1); }

    let keep_tail = 4;
    let cutoff = if history.len() > keep_tail { history.len() - keep_tail } else { 1 };
    let i = 1;
    while i < cutoff {
        if history[i].role == "tool" { history[i].content = "[Tool output compacted]"; }
        i += 1;
    }

    let total_chars = 0;
    for msg in history { total_chars += msg.content.len(); }
    while total_chars > MAX_TOTAL_CHARS && history.len() > 2 {
        let removed = history.remove(1);
        total_chars -= removed.content.len();
    }

    return history;
}
