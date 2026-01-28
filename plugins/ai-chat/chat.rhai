// plugins/ai-chat/chat.rhai

const HISTORY_PATH = ".deve/chat/history.json";

fn chat(req_id, user_message, context) {
    let history_json = fs_read(HISTORY_PATH) ?? "[]";
    let history = parse_json(history_json);
    if type_of(history) != "array" { history = []; }

    let cmd_res = commands::handle_command(user_message);
    if cmd_res != () {
        history.push(#{ role: "user", content: user_message });
        history.push(#{ role: "assistant", content: cmd_res });
        history = history::compact_history(history);
        fs_write(HISTORY_PATH, to_json(history));
        return cmd_res;
    }

    let tree = get_project_tree();
    let current_file = "";
    if type_of(context) == "map" && context.current_file != () {
        current_file = context.current_file;
    }
    let git_status = format::format_git_status();
    let skills_text = format::format_skills();
    let active_skills = format::format_active_skills(state::get_active_skills());
    let mcp_text = format::format_mcp_tools();
    let mcp_servers = format::format_mcp_servers();

    let mode = state::get_mode();
    let system_prompt = format::build_system_prompt(
        mode,
        tree,
        current_file,
        git_status,
        skills_text,
        active_skills,
        `${mcp_servers}\n\n${mcp_text}`
    );

    if history.len() == 0 || history[0].role != "system" {
        history.insert(0, #{ role: "system", content: system_prompt });
    } else {
        history[0].content = system_prompt;
    }

    history.push(#{ role: "user", content: user_message });
    history = history::compact_history(history);

    let config = #{
        base_url: env("AI_BASE_URL") ?? "https://api.openai.com/v1",
        api_key: env("AI_API_KEY"),
        model: env("AI_MODEL") ?? "gpt-4o",
        max_tokens: 4096,
        headers: #{}
    };

    let extra_headers_json = env("AI_EXTRA_HEADERS") ?? "{}";
    let extra_headers = parse_json(extra_headers_json);
    if type_of(extra_headers) == "map" { config.headers = extra_headers; }

    if config.api_key == () {
        return "Error: AI_API_KEY not set in environment.";
    }

    let tool_defs = if mode == state::mode_plan() { [] } else { tools::get_tools() };
    let result = tool_loop::run_tool_loop(req_id, config, history, tool_defs);
    let final_response = result.response ?? "";
    history = result.history ?? history;

    history.push(#{ role: "assistant", content: final_response });
    history = history::compact_history(history);
    fs_write(HISTORY_PATH, to_json(history));

    return final_response;
}

fn clear_history() {
    fs_write(HISTORY_PATH, "[]");
    return "Chat history cleared.";
}
