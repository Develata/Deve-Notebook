// crates/core/src/plugin/runtime/host/mod.rs
//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
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
mod managed_note;
#[cfg(not(target_arch = "wasm32"))]
mod managed_source_control;
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
use crate::ledger::traits::{RepoSelector, Repository};
#[cfg(not(target_arch = "wasm32"))]
use crate::protocol::{ServerError, ServerErrorCode};
#[cfg(not(target_arch = "wasm32"))]
use crate::source_control::CommitInfo;
#[cfg(not(target_arch = "wasm32"))]
use crate::source_control::{DelegatedSourceControlApi, SourceControlApi};
#[cfg(not(target_arch = "wasm32"))]
use crate::sync::SyncManager;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
pub use managed_note::{
    ManagedNoteMutationHost, ManagedNoteWriteIntent, set_managed_note_mutation_host,
};
#[cfg(not(target_arch = "wasm32"))]
pub use managed_source_control::{
    ManagedSourceControlCommitIntent, ManagedSourceControlMutationHost,
    set_managed_source_control_mutation_host,
};

#[cfg(not(target_arch = "wasm32"))]
static REPOSITORY: OnceLock<Arc<dyn Repository>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static SOURCE_CONTROL_API: OnceLock<Arc<dyn SourceControlApi>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static SOURCE_CONTROL_MODE: OnceLock<SourceControlHostMode> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static REPO_MANAGER: OnceLock<Arc<RepoManager>> = OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static SYNC_MANAGER: OnceLock<Arc<SyncManager>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceControlHostMode {
    Local,
    RemoteDelegated,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_repository(repo: Arc<dyn Repository>) -> Result<(), anyhow::Error> {
    REPOSITORY
        .set(repo)
        .map_err(|_| anyhow::anyhow!("Repository already set"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_source_control_api(api: Arc<dyn SourceControlApi>) -> Result<(), anyhow::Error> {
    set_source_control_api_with_mode(api, SourceControlHostMode::Local)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_delegated_source_control_api(
    api: Arc<dyn DelegatedSourceControlApi>,
) -> Result<(), anyhow::Error> {
    set_source_control_api_with_mode(api, SourceControlHostMode::RemoteDelegated)
}

#[cfg(not(target_arch = "wasm32"))]
fn set_source_control_api_with_mode(
    api: Arc<dyn SourceControlApi>,
    mode: SourceControlHostMode,
) -> Result<(), anyhow::Error> {
    SOURCE_CONTROL_API
        .set(api)
        .map_err(|_| anyhow::anyhow!("SourceControlApi already set"))?;
    SOURCE_CONTROL_MODE
        .set(mode)
        .map_err(|_| anyhow::anyhow!("SourceControl host mode already set"))
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
pub fn source_control_api() -> Result<Arc<dyn SourceControlApi>, anyhow::Error> {
    SOURCE_CONTROL_API
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("SourceControlApi not configured"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_source_control_write_allowed(selector: &RepoSelector) -> Result<(), ServerError> {
    match SOURCE_CONTROL_MODE.get().copied() {
        Some(SourceControlHostMode::RemoteDelegated) => return Ok(()),
        Some(SourceControlHostMode::Local) => {}
        None => {
            return Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "Plugin source-control write gate is not configured",
            ));
        }
    }
    let (Some(repo), Some(sync)) = (REPO_MANAGER.get().cloned(), SYNC_MANAGER.get().cloned())
    else {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "Plugin source-control write gate missing local RepoManager or SyncManager",
        ));
    };
    ensure_source_control_write_allowed_for(repo.as_ref(), sync.as_ref(), selector)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn commit_source_control_changes_in_repo(
    selector: &RepoSelector,
    message: &str,
) -> Result<CommitInfo, anyhow::Error> {
    match source_control_mode()? {
        SourceControlHostMode::Local => {
            managed_source_control::managed_source_control_mutation_host()?.commit_source_control(
                ManagedSourceControlCommitIntent {
                    selector: selector.clone(),
                    message: message.to_owned(),
                },
            )
        }
        SourceControlHostMode::RemoteDelegated => {
            // RemoteSourceControlApi forwards to the authoritative main process.
            source_control_api()?.commit_source_control_changes_in_repo(selector, message)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn source_control_mode() -> Result<SourceControlHostMode, anyhow::Error> {
    SOURCE_CONTROL_MODE
        .get()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("SourceControl host mode not configured"))
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_source_control_write_allowed_for(
    repo: &RepoManager,
    sync: &SyncManager,
    selector: &RepoSelector,
) -> Result<(), ServerError> {
    let repo_name = repo
        .resolve_local_repo_name_for_execution(selector.repo_id, selector.repo_name.as_deref())
        .map_err(|err| {
            ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, err.to_string())
        })?;
    if sync.is_projection_degraded(&repo_name) {
        return Err(ServerError::with_detail(
            ServerErrorCode::StoragePersistFailed,
            format!("Local repo projection is degraded; repair before writing: {repo_name}"),
        ));
    }
    repo.check_projection_locator_for_local_repo(&repo_name)
        .map_err(|err| {
            ServerError::with_detail(
                ServerErrorCode::StoragePersistFailed,
                format!(
                    "Local repo Projection workspace identity is invalid; repair before writing: {repo_name}: {err}"
                ),
            )
        })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn repo_manager() -> Result<Arc<RepoManager>, anyhow::Error> {
    REPO_MANAGER
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("RepoManager not configured"))
}

/// 注册核心 API 到 Rhai 引擎。
#[cfg(not(target_arch = "wasm32"))]
pub fn register_core_api(engine: &mut Engine, manifest: &PluginManifest) {
    use std::sync::Arc;
    let caps = Arc::new(manifest.capabilities.clone());

    fs::register_fs_api(engine, caps.clone());
    note::register_note_api(engine, caps.clone());
    git::register_git_api(engine, caps.clone());
    chat::register_chat_api(engine, caps.clone());
    util::register_util_api(engine, caps.clone());
    skill::register_skill_api(engine, caps.clone());
    search::register_search_api(engine, caps.clone());

    // 通用 API (跨平台)
    util::register_log_api(engine);
}

#[cfg(target_arch = "wasm32")]
pub fn register_core_api(engine: &mut Engine, _manifest: &PluginManifest) {
    util::register_log_api(engine);
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
