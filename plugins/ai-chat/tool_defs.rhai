// plugins/ai-chat/tool_defs.rhai
//! # 工具定义 (OpenAI Function Calling 格式)

fn make_tool(name, desc, params) {
    return #{ type: "function", function: #{ name: name, description: desc, parameters: params } };
}

fn params_empty() { return #{ type: "object", properties: #{}, required: [] }; }

fn params_one(name, schema, req) {
    let props = #{}; props[name] = schema;
    return #{ type: "object", properties: props, required: req };
}

fn schema_string(desc) { return #{ type: "string", description: desc }; }
fn schema_object(desc) { return #{ type: "object", description: desc }; }

// ============ 内置工具定义 ============

fn tool_read_file() {
    let params = params_one("path", schema_string("The file path to read"), ["path"]);
    return make_tool("read_file", "Read the contents of a file at the given path", params);
}

fn tool_git_status() {
    return make_tool("git_status", "List unstaged changes in the Redb source control", params_empty());
}

fn tool_git_diff() {
    let params = params_one("path", schema_string("The document path to diff"), ["path"]);
    return make_tool("git_diff", "Show unified diff for a file path", params);
}

fn tool_git_add() {
    let params = params_one("path", schema_string("The document path to stage"), ["path"]);
    return make_tool("git_add", "Stage a document path in source control", params);
}

fn tool_git_commit() {
    let params = params_one("message", schema_string("Commit message"), ["message"]);
    return make_tool("git_commit", "Commit staged documents with a message", params);
}

fn tool_search_content() {
    let props = #{};
    props.pattern = schema_string("Regex pattern to search for");
    props.path = schema_string("Directory to search in (default: current)");
    let p = #{ type: "object", properties: props, required: ["pattern"] };
    return make_tool("search_content", "Search for a pattern in file contents using regex", p);
}

fn tool_list_skills() {
    return make_tool("list_skills", "List available skills and their descriptions", params_empty());
}

fn tool_use_skill() {
    let params = params_one("name", schema_string("Skill name to load"), ["name"]);
    return make_tool("use_skill", "Load a skill by name and return its instructions", params);
}

fn tool_mcp_list_tools() {
    return make_tool("mcp_list_tools", "List tools from connected MCP servers", params_empty());
}

fn tool_mcp_list_servers() {
    return make_tool("mcp_list_servers", "List MCP servers and status", params_empty());
}

fn tool_mcp_call_tool() {
    let props = #{};
    props.server = schema_string("MCP server name");
    props.tool = schema_string("Tool name");
    props.args = schema_object("Tool arguments");
    let p = #{ type: "object", properties: props, required: ["server", "tool"] };
    return make_tool("mcp_call_tool", "Call a tool on a MCP server", p);
}

/// 追加 MCP 动态工具
fn append_mcp_tools(tools) {
    let mcp = mcp_list_tools();
    if type_of(mcp) != "array" { return tools; }
    for t in mcp {
        let server = t.server ?? "";
        let name = t.name ?? "";
        if server == "" || name == "" { continue; }
        let full = `mcp::${server}::${name}`;
        let desc = t.description ?? "";
        let schema = t.input_schema ?? #{};
        tools.push(make_tool(full, desc, schema));
    }
    return tools;
}

/// 获取所有工具定义
fn get_all_tools() {
    let tools = [
        tool_read_file(),
        tool_git_status(),
        tool_git_diff(),
        tool_git_add(),
        tool_git_commit(),
        tool_search_content(),
        tool_list_skills(),
        tool_use_skill(),
        tool_mcp_list_tools(),
        tool_mcp_list_servers(),
        tool_mcp_call_tool(),
    ];
    return append_mcp_tools(tools);
}
