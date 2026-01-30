// plugins/ai-chat/history.rhai
//! # History Compaction (历史压缩)
//!
//! **Invariant**: history[0] 始终是 system prompt，不可删除。
//! **Pre-condition**: history 是非空数组，history[0].role == "system"。
//! **Post-condition**: history.len() <= MAX_HISTORY_ROUNDS * 2 + 1
//!                    && total_chars(history) <= MAX_TOTAL_CHARS。

const MAX_HISTORY_ROUNDS = 10;
const MAX_TOTAL_CHARS = 24000;

/// 压缩对话历史
///
/// **算法**:
/// 1. 删除最旧的消息（保留 system prompt）直到满足轮次限制
/// 2. 压缩旧的 tool 输出（替换为占位符）
/// 3. 删除最旧的消息直到满足字符限制
///
/// **复杂度**: O(n) 时间，O(1) 空间（原地修改）
fn compact_history(history) {
    // Phase 1: 轮次限制 (最多 10 轮 = 21 条消息)
    let max_messages = MAX_HISTORY_ROUNDS * 2 + 1;
    while history.len() > max_messages {
        history.remove(1); // 保护 index 0 (system prompt)
    }

    // Phase 2: 压缩旧的 tool 输出 (保留最近 4 条完整)
    let keep_tail = 4;
    let cutoff = if history.len() > keep_tail { history.len() - keep_tail } else { 1 };
    let i = 1;
    while i < cutoff {
        if history[i].role == "tool" {
            history[i].content = "[Tool output compacted]";
        }
        i += 1;
    }

    // Phase 3: 字符限制 (最多 24KB)
    let total_chars = 0;
    for msg in history {
        total_chars += msg.content.len();
    }
    while total_chars > MAX_TOTAL_CHARS && history.len() > 2 {
        let removed = history.remove(1);
        total_chars -= removed.content.len();
    }

    return history;
}
