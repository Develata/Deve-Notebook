// crates/core/src/plugin/runtime/host/mod.rs
//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
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
mod note;
#[cfg(not(target_arch = "wasm32"))]
mod path_guard;
#[cfg(not(target_arch = "wasm32"))]
mod search;
#[cfg(not(target_arch = "wasm32"))]
mod skill;
mod util;

use crate::plugin::manifest::PluginManifest;
use rhai::Engine;

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::RepoManager;
#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::traits::Repository;
#[cfg(not(target_arch = "wasm32"))]
use crate::sync::SyncManager;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
static REPOSITORY: OnceLock<Arc<dyn Repository>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static REPO_MANAGER: OnceLock<Arc<RepoManager>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static SYNC_MANAGER: OnceLock<Arc<SyncManager>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
pub fn set_repository(repo: Arc<dyn Repository>) -> Result<(), anyhow::Error> {
    REPOSITORY
        .set(repo)
        .map_err(|_| anyhow::anyhow!("Repository already set"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_repo_manager(repo: Arc<RepoManager>) -> Result<(), anyhow::Error> {
    REPO_MANAGER
        .set(repo)
        .map_err(|_| anyhow::anyhow!("RepoManager already set"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_sync_manager(manager: Arc<SyncManager>) -> Result<(), anyhow::Error> {
    SYNC_MANAGER
        .set(manager)
        .map_err(|_| anyhow::anyhow!("SyncManager already set"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn repository() -> Result<Arc<dyn Repository>, anyhow::Error> {
    REPOSITORY
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Repository not configured"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn repo_manager() -> Result<Arc<RepoManager>, anyhow::Error> {
    REPO_MANAGER
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("RepoManager not configured"))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn sync_manager() -> Result<Arc<SyncManager>, anyhow::Error> {
    SYNC_MANAGER
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("SyncManager not configured"))
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
        note::register_note_api(engine, caps.clone());
        git::register_git_api(engine, caps.clone());
        chat::register_chat_api(engine, caps.clone());
        util::register_util_api(engine, caps.clone());
        skill::register_skill_api(engine, caps.clone());
        search::register_search_api(engine, caps.clone());
    }

    // 通用 API (跨平台)
    util::register_log_api(engine);
}
