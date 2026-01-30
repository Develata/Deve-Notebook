// plugins/ai-chat/tools.rhai
//! # 工具模块入口
//!
//! **功能**: 导出工具定义和执行接口。
//!
//! **模块结构**:
//! - `tool_defs`: 工具定义 (OpenAI Function Calling 格式)
//! - `tool_exec`: 工具执行逻辑

import "tool_defs" as defs;
import "tool_exec" as exec;

/// 获取所有工具定义 (OpenAI 格式)
fn get_tools() {
    return defs::get_all_tools();
}

/// 执行工具调用并返回结果
fn execute_tool(name, arguments) {
    return exec::run_tool(name, arguments);
}
