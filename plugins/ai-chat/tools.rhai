// plugins/ai-chat/tools.rhai
// AI Chat 工具库
// 包含工具定义 (OpenAI Function Calling 格式) 和执行逻辑
//
// Invariant: 所有工具必须返回字符串结果供 LLM 处理

fn make_tool(name, desc, params) {
    let fn_map = #{};
    fn_map.name = name;
    fn_map.description = desc;
    fn_map.parameters = params;

    let tool = #{};
    tool.type = "function";
    tool.function = fn_map;
    return tool;
}

fn params_empty() {
    let p = #{};
    p.type = "object";
    p.properties = #{};
    p.required = [];
    return p;
}

fn params_one(name, schema, required_list) {
    let p = #{};
    p.type = "object";
    let props = #{};
    props[name] = schema;
    p.properties = props;
    p.required = required_list;
    return p;
}

fn schema_string(desc) {
    let s = #{};
    s.type = "string";
    s.description = desc;
    return s;
}

fn schema_object(desc) {
    let s = #{};
    s.type = "object";
    s.description = desc;
    return s;
}

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
    let p = #{};
    p.type = "object";
    let props = #{};
    props.pattern = schema_string("Regex pattern to search for");
    props.path = schema_string("Directory to search in (default: current)");
    p.properties = props;
    p.required = ["pattern"];
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
    let p = #{};
    p.type = "object";
    let props = #{};
    props.server = schema_string("MCP server name");
    props.tool = schema_string("Tool name");
    props.args = schema_object("Tool arguments");
    p.properties = props;
    p.required = ["server", "tool"];
    return make_tool("mcp_call_tool", "Call a tool on a MCP server", p);
}

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

// 内置工具定义 (OpenAI 格式)
fn get_tools() {
    let tools = [];
    tools.push(tool_read_file());
    tools.push(tool_git_status());
    tools.push(tool_git_diff());
    tools.push(tool_git_add());
    tools.push(tool_git_commit());
    tools.push(tool_search_content());
    tools.push(tool_list_skills());
    tools.push(tool_use_skill());
    tools.push(tool_mcp_list_tools());
    tools.push(tool_mcp_list_servers());
    tools.push(tool_mcp_call_tool());
    return append_mcp_tools(tools);
}

// 执行工具调用并返回结果
fn execute_tool(name, arguments) {
    let args = parse_json(arguments);
    
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
        let status = sc_status();
        return to_json(status);
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
        let commit = sc_commit(message);
        return to_json(commit);
    }

    if name == "list_skills" {
        let skills = list_skills();
        return to_json(skills);
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

    if name == "mcp_list_tools" {
        let tools = mcp_list_tools();
        return to_json(tools);
    }

    if name == "mcp_list_servers" {
        let servers = mcp_list_servers();
        return to_json(servers);
    }

    if name == "mcp_call_tool" {
        let server = args.server ?? "";
        let tool = args.tool ?? "";
        let mcp_args = args.args ?? #{};
        if server == "" || tool == "" { return "Error: server/tool is required"; }
        let res = mcp_call_tool(server, tool, mcp_args);
        return to_json(res);
    }

    if name.starts_with("mcp::") {
        let parts = name.split("::");
        if parts.len() >= 3 {
            let server = parts[1];
            let tool = parts[2];
            let res = mcp_call_tool(server, tool, args);
            return to_json(res);
        }
    }
    
    return `Error: Unknown tool "${name}"`;
}
