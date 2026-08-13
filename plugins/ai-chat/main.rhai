// plugins/ai-chat/main.rhai
// Native AI Chat Plugin — 默认只走流式原生聊天，不启用工具循环。

fn run_tool(name, arguments) {
    "Error: native AI tools are disabled by default"
}

fn system_prompt_base() {
    "You are a helpful AI assistant for Deve-Notebook. \
Focus on the current markdown note and the explicitly provided context. \
Do not assume access to external tools unless the host explicitly adds them."
}

fn build_system_prompt(context) {
    let system = system_prompt_base();
    if context == () {
        return system;
    }

    let ctx = if type_of(context) == "string" {
        if context == "" {
            return system;
        }
        parse_json(context)
    } else {
        context
    };

    if ctx["current_file"] != () && ctx["current_file"] != "" {
        system += "\n\nCurrent file: " + ctx["current_file"];
    }
    if ctx["chat_mode"] == "plan" {
        system += "\n\nPLAN mode: analyze the note, propose a concise implementation plan, and do not claim to edit files or call tools.";
    } else if ctx["chat_mode"] == "build" {
        system += "\n\nBUILD mode: provide concrete code-oriented guidance and patches for the user to review; do not execute workspace, source-control, shell, MCP, or skill actions.";
    }
    if ctx["current_markdown"] != () && ctx["current_markdown"] != "" {
        let markdown = ctx["current_markdown"];
        system += "\n\nCurrent markdown content:\n```markdown\n" + markdown + "\n```";
    }
    if ctx["selection"] != () && ctx["selection"]["text"] != () {
        let sel_text = ctx["selection"]["text"];
        system += "\n\nSelected text:\n```\n" + sel_text + "\n```";
    }

    system
}

fn chat(req_id, user_msg, context) {
    chat(req_id, user_msg, context, [])
}

fn append_prior_history(history, prior_history) {
    if prior_history == () || type_of(prior_history) != "array" {
        return history;
    }

    let next = history;
    for message in prior_history {
        if message["role"] == () || message["content"] == () || message["content"] == "" {
            continue;
        }
        if message["role"] == "user" || message["role"] == "assistant" {
            next.push(#{
                role: message["role"],
                content: message["content"]
            });
        }
    }
    next
}

fn chat(req_id, user_msg, context, prior_history) {
    let base_history = [
        #{role: "system", content: build_system_prompt(context)}
    ];
    let history = append_prior_history(base_history, prior_history);
    history.push(#{role: "user", content: user_msg});

    ai_chat_stream(req_id, history)
}
