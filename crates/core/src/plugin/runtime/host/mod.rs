// crates/core/src/plugin/runtime/host/mod.rs
//! # Host Functions (宿主函数模块)
//!
//! **功能**:
//! 向 Rhai 引擎注册宿主环境提供的能力（如文件 IO、版本控制、AI 聊天）。
//!
//! **模块结构**:
//! - `fs`: 文件系统操作 (fs_read, fs_write, get_project_tree) [仅非 WASM]
//! - `git`: 版本控制操作 (sc_status, sc_diff, sc_stage, sc_commit) [仅非 WASM]
//! - `chat`: AI 聊天流式处理 (ai_chat_stream, ai_chat_stream_with_tools) [仅非 WASM]
//! - `util`: 辅助函数 (to_json, parse_json, env, log_info)
//!
//! **安全**:
//! 所有敏感操作必须经过 `Capability` 检查。

#[cfg(not(target_arch = "wasm32"))]
mod chat;
#[cfg(not(target_arch = "wasm32"))]
mod fs;
#[cfg(not(target_arch = "wasm32"))]
mod git;
#[cfg(not(target_arch = "wasm32"))]
mod mcp;
#[cfg(not(target_arch = "wasm32"))]
mod search;
#[cfg(not(target_arch = "wasm32"))]
mod skill;
mod util;

use crate::plugin::manifest::PluginManifest;
use rhai::Engine;

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::traits::Repository;
#[cfg(not(target_arch = "wasm32"))]
use crate::mcp::McpManager;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
static REPOSITORY: OnceLock<Arc<dyn Repository>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static MCP_MANAGER: OnceLock<Arc<McpManager>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
pub fn set_repository(repo: Arc<dyn Repository>) -> Result<(), anyhow::Error> {
    REPOSITORY
        .set(repo)
        .map_err(|_| anyhow::anyhow!("Repository already set"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_mcp_manager(manager: Arc<McpManager>) -> Result<(), anyhow::Error> {
    MCP_MANAGER
        .set(manager)
        .map_err(|_| anyhow::anyhow!("McpManager already set"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn repository() -> Result<Arc<dyn Repository>, anyhow::Error> {
    REPOSITORY
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Repository not configured"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn mcp_manager() -> Option<Arc<McpManager>> {
    MCP_MANAGER.get().cloned()
}

/// 注册核心 API 到 Rhai 引擎
#[allow(unused_variables)]
pub fn register_core_api(engine: &mut Engine, manifest: &PluginManifest) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::Arc;
        let caps = Arc::new(manifest.capabilities.clone());

        // 注册各领域 API (仅非 WASM 环境)
        fs::register_fs_api(engine, caps.clone());
        git::register_git_api(engine, caps.clone());
        chat::register_chat_api(engine, caps.clone());
        util::register_util_api(engine, caps.clone());
        skill::register_skill_api(engine);
        search::register_search_api(engine);
        let manager = mcp_manager().unwrap_or_else(|| Arc::new(McpManager::new()));
        mcp::register_mcp_api(engine, manager);
    }

    // 通用 API (跨平台)
    util::register_log_api(engine);
}
