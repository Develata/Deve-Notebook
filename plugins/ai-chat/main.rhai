// plugins/ai-chat/main.rhai
// AI Chat Plugin — 通过 OpenAI 兼容 API 实现流式对话 + 工具循环
//
// Invariants:
// 1. tool_loop 最多迭代 MAX_TOOL_ROUNDS 轮，防止无限循环
// 2. 每轮 tool_calls 结果追加到 history 后重新提交
// 3. 所有 API 配置从环境变量读取 (secure)

import "tools" as tools;

/// 顶层 wrapper — 供 call_fn 外部调用 tool router
fn run_tool(name, arguments) {
    tools::execute_tool(name, arguments)
}

/// 最大工具调用轮次 (防止 AI 无限循环调用工具)
const MAX_TOOL_ROUNDS = 3;

/// 默认系统提示词
const SYSTEM_PROMPT = "You are a helpful AI assistant for Deve-Notebook. \
You can read files, check git status, view diffs, stage and commit changes. \
Be concise and precise. When the user asks about code, use the read_file tool. \
When asked about changes, use git_status and git_diff tools.";

/// 构建 AI 配置 (从环境变量读取)
///
/// Post-condition: 返回的 config 包含 base_url, api_key, model 三个必填字段
fn build_config() {
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

    let max_tokens = env("AI_MAX_TOKENS");
    if max_tokens == () || max_tokens == "" {
        max_tokens = 4096;
    }

    #{
        base_url: base_url,
        api_key: api_key,
        model: model,
        max_tokens: max_tokens
    }
}

/// 构建内置工具列表 (OpenAI function calling 格式)
fn get_builtin_tools() {
    [
        #{type: "function", function: #{
            name: "read_file",
            description: "Read the contents of a file at the given path",
            parameters: #{type: "object",
                properties: #{path: #{type: "string", description: "The file path to read"}},
                required: ["path"]}
        }},
        #{type: "function", function: #{
            name: "git_status",
            description: "List unstaged changes in source control",
            parameters: #{type: "object", properties: #{}, required: []}
        }},
        #{type: "function", function: #{
            name: "git_diff",
            description: "Show unified diff for a document path",
            parameters: #{type: "object",
                properties: #{path: #{type: "string", description: "The document path to diff"}},
                required: ["path"]}
        }},
        #{type: "function", function: #{
            name: "git_add",
            description: "Stage a document path in source control",
            parameters: #{type: "object",
                properties: #{path: #{type: "string", description: "The document path to stage"}},
                required: ["path"]}
        }},
        #{type: "function", function: #{
            name: "git_commit",
            description: "Commit staged documents with a message",
            parameters: #{type: "object",
                properties: #{message: #{type: "string", description: "Commit message"}},
                required: ["message"]}
        }}
    ]
}

/// 主入口: 处理聊天请求 (带工具循环)
///
/// Pre-condition: req_id 非空, user_msg 非空
/// Post-condition: 流式输出已完成, 返回最终结果
/// Invariant: 最多循环 MAX_TOOL_ROUNDS 轮
fn chat(req_id, user_msg, context) {
    let config = build_config();

    if config.api_key == () || config.api_key == "" {
        return #{
            type: "text",
            content: "Error: No AI API key configured. Set AI_API_KEY or OPENAI_API_KEY environment variable."
        };
    }

    // 构建历史消息
    let system = SYSTEM_PROMPT;

    // 若有当前文件上下文, 附加到系统提示
    if context != () {
        let ctx = if type_of(context) == "string" {
            parse_json(context)
        } else {
            context
        };
        if ctx["current_file"] != () && ctx["current_file"] != "" {
            system += "\n\nThe user is currently editing: " + ctx["current_file"];
        }
    }

    let history = [
        #{role: "system", content: system},
        #{role: "user", content: user_msg}
    ];

    let tool_defs = get_builtin_tools();

    // Tool Loop: AI 可能请求工具调用, 最多 MAX_TOOL_ROUNDS 轮
    let round = 0;
    loop {
        round += 1;
        if round > MAX_TOOL_ROUNDS {
            log_info("ai-chat: max tool rounds reached (" + MAX_TOOL_ROUNDS + ")");
            break;
        }

        let result = ai_chat_stream_with_tools(req_id, config, history, tool_defs);

        if result["type"] == "text" {
            return result;
        }

        if result["type"] != "tool_calls" || result["calls"] == () {
            return result;
        }

        // 处理工具调用
        let calls = result["calls"];
        log_info("ai-chat: round " + round + ", " + calls.len() + " tool call(s)");

        // 追加 assistant 的 tool_calls 消息
        let assistant_tool_calls = [];
        for tc in calls {
            assistant_tool_calls.push(#{
                id: tc["id"],
                type: "function",
                function: #{name: tc["name"], arguments: tc["arguments"]}
            });
        }
        history.push(#{role: "assistant", content: (), tool_calls: assistant_tool_calls});

        // 执行每个工具, 追加结果
        for tc in calls {
            let tool_result = tools::execute_tool(tc["name"], tc["arguments"]);
            history.push(#{
                role: "tool",
                tool_call_id: tc["id"],
                content: tool_result
            });
        }

        // 继续循环, 让 AI 处理工具结果
    }

    #{type: "text", content: "Tool loop limit reached."}
}
