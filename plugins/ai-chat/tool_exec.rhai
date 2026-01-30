// plugins/ai-chat/tool_exec.rhai
//! # 工具执行器
//!
//! **Invariant**: 所有工具必须返回字符串结果供 LLM 处理。
//! **Pre-condition**: arguments 是有效的 JSON 字符串。

/// 执行内置工具
fn execute_builtin(name, args) {
    if name == "read_file" {
        let path = args.path ?? "";
        if path == "" { return "Error: path is required"; }
        let content = fs_read(path);
        if content == () { return "Error: File not found or cannot be read"; }
        return content;
    }

    if name == "search_content" {
        let pattern = args.pattern ?? "";
        if pattern == "" { return "Error: pattern is required"; }
        return `Search for "${pattern}" - feature pending full implementation`;
    }

    if name == "git_status" {
        return to_json(sc_status());
    }

    if name == "git_diff" {
        let path = args.path ?? "";
        if path == "" { return "Error: path is required"; }
        return sc_diff(path);
    }

    if name == "git_add" {
        let path = args.path ?? "";
        if path == "" { return "Error: path is required"; }
        sc_stage(path);
        return "OK";
    }

    if name == "git_commit" {
        let message = args.message ?? "";
        if message == "" { return "Error: message is required"; }
        return to_json(sc_commit(message));
    }

    return ();
}

/// 执行 skill 相关工具
fn execute_skill_tool(name, args) {
    if name == "list_skills" {
        return to_json(list_skills());
    }

    if name == "use_skill" {
        let skill_name = args.name ?? "";
        if skill_name == "" { return "Error: name is required"; }
        let skill = get_skill(skill_name);
        if skill == () { return "Error: skill not found"; }
        let content = skill.content ?? "";
        let desc = skill.description ?? "";
        return `Skill: ${skill_name}\n${desc}\n\n${content}`;
    }

    return ();
}

/// 执行 MCP 相关工具
fn execute_mcp_tool(name, args) {
    if name == "mcp_list_tools" {
        return to_json(mcp_list_tools());
    }

    if name == "mcp_list_servers" {
        return to_json(mcp_list_servers());
    }

    if name == "mcp_call_tool" {
        let server = args.server ?? "";
        let tool = args.tool ?? "";
        let mcp_args = args.args ?? #{};
        if server == "" || tool == "" { return "Error: server/tool is required"; }
        return to_json(mcp_call_tool(server, tool, mcp_args));
    }

    // 动态 MCP 工具 (mcp::server::tool 格式)
    if name.starts_with("mcp::") {
        let parts = name.split("::");
        if parts.len() >= 3 {
            let server = parts[1];
            let tool = parts[2];
            return to_json(mcp_call_tool(server, tool, args));
        }
    }

    return ();
}

/// 执行工具 (主入口)
fn run_tool(name, arguments) {
    let args = parse_json(arguments);

    // 尝试内置工具
    let result = execute_builtin(name, args);
    if result != () { return result; }

    // 尝试 skill 工具
    result = execute_skill_tool(name, args);
    if result != () { return result; }

    // 尝试 MCP 工具
    result = execute_mcp_tool(name, args);
    if result != () { return result; }

    return `Error: Unknown tool "${name}"`;
}
