// plugins/ai-chat/format.rhai

fn format_git_status() {
    let raw = tools::execute_tool("git_status", "{}");
    let parsed = parse_json(raw);
    if type_of(parsed) != "array" { return "(unavailable)"; }

    let lines = [];
    for item in parsed {
        let path = item.path ?? "";
        let status = item.status ?? "Modified";
        lines.push(`${status} ${path}`);
    }
    if lines.len() == 0 { return "(clean)"; }
    return lines.join("\n");
}

fn format_skills() {
    let list = list_skills();
    if type_of(list) != "array" || list.len() == 0 {
        return "(none)";
    }

    let lines = [];
    let count = 0;
    for s in list {
        if count >= 20 { break; }
        let name = s.name ?? "";
        let desc = s.description ?? "";
        lines.push(`${name}: ${desc}`);
        count += 1;
    }
    return lines.join("\n");
}

fn format_active_skills(names) {
    if type_of(names) != "array" || names.len() == 0 { return "(none)"; }
    let blocks = [];
    for n in names {
        let s = get_skill(n);
        if s == () { continue; }
        let desc = s.description ?? "";
        let content = s.content ?? "";
        blocks.push(`## ${n}\n${desc}\n\n${content}`);
    }
    if blocks.len() == 0 { return "(none)"; }
    return blocks.join("\n\n");
}

fn format_mcp_tools() {
    let raw = mcp_list_tools();
    if type_of(raw) != "array" || raw.len() == 0 { return "(none)"; }

    let lines = [];
    let count = 0;
    for t in raw {
        if count >= 30 { break; }
        let server = t.server ?? "";
        let name = t.name ?? "";
        let desc = t.description ?? "";
        lines.push(`${server}::${name} ${desc}`);
        count += 1;
    }
    return lines.join("\n");
}

fn format_mcp_servers() {
    let raw = mcp_list_servers();
    if type_of(raw) != "array" || raw.len() == 0 { return "(none)"; }

    let lines = [];
    for s in raw {
        let name = s.name ?? "";
        let status = s.status ?? "unknown";
        lines.push(`${name}: ${status}`);
    }
    return lines.join("\n");
}

fn build_system_prompt(mode, tree, current_file, git_status, skills_text, active_skills, mcp_text) {
    let mode_hint = if mode == state::mode_plan() {
        "You are in PLAN mode. Only analyze and produce a plan. Do not write code or call tools."
    } else {
        "You are in BUILD mode. Implement changes carefully and use tools when needed."
    };

    let prompt = `
<role>
You are the "Math-Architect", a senior Rust expert.
You must respond in Chinese.
${mode_hint}
</role>

<constraints>
1. File size target < 130 lines. Hard limit 250 lines.
2. Optimize for 768MB RAM VPS; avoid heavy dependencies.
3. Rust Edition 2024. Use anyhow::Result in apps, thiserror in libs.
4. For complex logic, document Invariants/Pre/Post conditions.
</constraints>

<context>
<project_structure>
${tree}
</project_structure>

<active_file>${current_file}</active_file>

<git_status>
${git_status}
</git_status>

<skills>
${skills_text}
</skills>

<skills_active>
${active_skills}
</skills_active>

<mcp>
${mcp_text}
</mcp>
</context>
`;

    return prompt;
}
