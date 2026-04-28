// plugins/ai-chat/main.rhai
// Native AI Chat Plugin — 默认只走流式原生聊天，不启用工具循环。

fn run_tool(name, arguments) {
    "Error: native AI tools are disabled by default"
}

const SYSTEM_PROMPT = "You are a helpful AI assistant for Deve-Notebook. \
Focus on the current markdown note and the explicitly provided context. \
Do not assume access to external tools unless the host explicitly adds them.";

fn _build_config() {
    let base_url = env("AI_BASE_URL");
    if base_url == () || base_url == "" {
        base_url = "https://api.openai.com/v1";
    }

    let api_key = env("AI_API_KEY");
    if api_key == () || api_key == "" {
        api_key = env("OPENAI_API_KEY");
    }
    if api_key == () || api_key == "" {
        api_key = env("ANTHROPIC_API_KEY");
    }

    let model = env("AI_MODEL");
    if model == () || model == "" {
        model = "gpt-4o-mini";
    }

    let max_tokens_str = env("AI_MAX_TOKENS");
    let max_tokens = if max_tokens_str == () || max_tokens_str == "" {
        4096
    } else {
        parse_int(max_tokens_str)
    };

    #{
        base_url: base_url,
        api_key: api_key,
        model: model,
        max_tokens: max_tokens
    }
}

fn build_system_prompt(context) {
    let system = SYSTEM_PROMPT;
    if context == () {
        return system;
    }

    let ctx = if type_of(context) == "string" {
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
    let config = _build_config();

    if config.api_key == () || config.api_key == "" {
        return #{
            type: "text",
            content: "Error: No AI API key configured. Set AI_API_KEY or OPENAI_API_KEY environment variable."
        };
    }

    let history = [
        #{role: "system", content: build_system_prompt(context)},
        #{role: "user", content: user_msg}
    ];

    ai_chat_stream(req_id, config, history)
}
