// plugins/ai-chat/main.rhai
// AI Chat 插件主入口 (模块化)

import "tools" as tools;
import "state" as state;
import "history" as history;
import "format" as format;
import "commands" as commands;
import "tool_loop" as tool_loop;
import "chat" as chat;

fn chat(req_id, user_message, context) {
    return chat::chat(req_id, user_message, context);
}

fn clear_history() {
    return chat::clear_history();
}
