// plugins/ai-chat/tools.rhai
// Tool Execution Router — 将 AI function calling 路由到宿主函数
//
// Invariant: 每个工具名必须在 builtin_tools() 中有定义
// Pre-condition: arguments 为合法 JSON 字符串
// Post-condition: 返回字符串结果（成功或错误描述）

/// 安全调用宿主函数，捕获异常返回错误字符串
/// Rhai 的 try/catch 是 statement，需显式赋值 + return
fn safe_fs_read(path) {
    let r = (); try { r = fs_read(path); } catch(e) { return "Error: " + e; }
    r
}
fn safe_sc_status() {
    let r = (); try { r = to_json(sc_status()); } catch(e) { return "Error: " + e; }
    r
}
fn safe_sc_diff(path) {
    let r = (); try { r = sc_diff(path); } catch(e) { return "Error: " + e; }
    r
}
fn safe_sc_stage(path) {
    try { sc_stage(path); } catch(e) { return "Error: " + e; }
    "OK: staged " + path
}
fn safe_sc_commit(msg) {
    let r = (); try { r = to_json(sc_commit(msg)); } catch(e) { return "Error: " + e; }
    r
}
fn safe_project_tree() {
    let r = (); try { r = get_project_tree(); } catch(e) { return "Error: " + e; }
    r
}

/// 执行单个工具调用，返回字符串结果
fn execute_tool(name, arguments) {
    let args = parse_json(arguments);

    if name == "read_file" {
        let path = args["path"];
        if path == () { return "Error: missing 'path' argument"; }
        return safe_fs_read(path);
    }
    if name == "git_status" {
        return safe_sc_status();
    }
    if name == "git_diff" {
        let path = args["path"];
        if path == () { return "Error: missing 'path' argument"; }
        return safe_sc_diff(path);
    }
    if name == "git_add" {
        let path = args["path"];
        if path == () { return "Error: missing 'path' argument"; }
        return safe_sc_stage(path);
    }
    if name == "git_commit" {
        let msg = args["message"];
        if msg == () { return "Error: missing 'message' argument"; }
        return safe_sc_commit(msg);
    }
    if name == "search_files" {
        let pattern = args["pattern"];
        if pattern == () { return "Error: missing 'pattern' argument"; }
        return safe_project_tree();
    }
    if name == "grep" {
        let pattern = args["pattern"];
        if pattern == () { return "Error: missing 'pattern' argument"; }
        return safe_project_tree();
    }

    "Error: unknown tool '" + name + "'"
}
