// apps/web/src/hooks/use_core/mod.rs
//! # Core State Hook (核心状态钩子)
//!
//! 管理前端全局核心状态，并把 WebLightPeer 的浏览器存储分层接入 UI。

pub mod apply;
pub mod callbacks;
pub mod callbacks_sc;
mod callbacks_sc_scope;
mod callbacks_sc_target;
mod callbacks_scope;
mod callbacks_switch;
mod callbacks_sync;
pub mod contexts;
mod dashboard_context;
pub mod diff_session;
pub mod effects;
pub mod effects_msg;
pub mod effects_sc;
mod effects_sc_apply;
mod effects_sc_scope;
mod effects_sc_state;
pub mod effects_switch;
pub mod navigation;
pub mod pending;
mod provide;
pub mod state;
mod state_build;
mod state_init;
mod storage_runtime;
mod switch_nonce;
pub mod types;

pub use contexts::*;
pub use types::*;

use crate::api::WsService;
use leptos::prelude::*;

use self::state_build::{CoreStateCallbacks, build_core_state};
use self::storage_runtime::init_storage_runtime;

/// 初始化核心状态钩子。
pub fn use_core() -> CoreState {
    let ws = WsService::new();
    provide_context(ws.clone());

    let signals = state::init_signals(ws.status);
    let status_signal_for_text = ws.status;
    let degraded_for_text = signals.degraded_sync_mode;
    let status_text = Signal::derive(move || {
        let base = format!("{}", status_signal_for_text.get());
        if degraded_for_text.get().is_some() {
            format!("{base} · Read-only")
        } else {
            base
        }
    });

    // 浏览器 peer identity 现在必须经由 storage_runtime 间接初始化：
    // `localStorage` 只允许承载 UI 偏好，而 repo-scoped identity 需要走
    // `WebCrypto + IndexedDB`，这样才能满足 T3 定义的存储分层与降级语义。
    let (identity, repo_vector) = init_storage_runtime(&signals);

    effects::setup_handshake_effect(
        &ws,
        HandshakeSignals {
            identity,
            repo_vector,
            degraded: signals.degraded_sync_mode,
            current_repo: signals.current_repo,
            current_repo_id: signals.current_repo_id,
            active_branch: signals.active_branch,
            pending_branch_switch: signals.pending_branch_switch,
            set_pending_branch_switch: signals.set_pending_branch_switch,
            set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
            pending_repo_switch: signals.pending_repo_switch,
            set_pending_repo_switch: signals.set_pending_repo_switch,
            set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
            handshake_scope_nonce: signals.handshake_scope_nonce,
            set_handshake_scope_nonce: signals.set_handshake_scope_nonce,
            set_repo_list_request_id: signals.set_repo_list_request_id,
            set_doc_list_request_id: signals.set_doc_list_request_id,
            set_tree_request_id: signals.set_tree_request_id,
            set_handshake_ready: signals.set_handshake_ready,
        },
    );
    effects::setup_message_effect(&ws, &signals);

    let doc_callbacks = callbacks::create_doc_callbacks(
        &ws,
        signals.current_doc,
        callbacks_scope::LocalScopeSignals {
            current_repo_id: signals.current_repo_id,
            current_scope_nonce: signals.current_scope_nonce,
            active_branch: signals.active_branch,
            pending_branch_switch: signals.pending_branch_switch,
            pending_repo_switch: signals.pending_repo_switch,
        },
        signals.pending_local_edits,
        signals.set_pending_navigation,
        signals.set_current_doc,
        signals.set_explicit_home,
    );
    let sync_callbacks = callbacks::create_sync_callbacks(
        &ws,
        signals.current_doc,
        callbacks_scope::LocalScopeSignals {
            current_repo_id: signals.current_repo_id,
            current_scope_nonce: signals.current_scope_nonce,
            active_branch: signals.active_branch,
            pending_branch_switch: signals.pending_branch_switch,
            pending_repo_switch: signals.pending_repo_switch,
        },
        signals.set_shadow_list_request_id,
        signals.set_sync_mode_request_id,
        signals.set_pending_ops_request_id,
    );
    let sc_callbacks = callbacks::create_source_control_callbacks(
        &ws,
        signals.staged_changes,
        signals.unstaged_changes,
        callbacks_sc::SourceControlScopeSignals {
            current_repo_id: signals.current_repo_id,
            current_scope_nonce: signals.current_scope_nonce,
            pending_branch_switch: signals.pending_branch_switch,
            pending_repo_switch: signals.pending_repo_switch,
        },
        callbacks_sc::SourceControlRequestSignals {
            set_changes_request_id: signals.set_changes_request_id,
            set_commit_history_request_id: signals.set_commit_history_request_id,
            set_doc_diff_request_id: signals.set_doc_diff_request_id,
            set_commit_diff_request_id: signals.set_commit_diff_request_id,
        },
    );
    let misc_callbacks = callbacks::create_misc_callbacks(
        &ws,
        signals.set_stats,
        signals.load_state,
        signals.set_plugin_request_ids,
        signals.set_search_request_id,
    );
    let switch_callbacks = callbacks::create_switch_callbacks(
        &ws,
        SwitchScopeSignals {
            current_doc: signals.current_doc,
            pending_local_edits: signals.pending_local_edits,
            set_pending_navigation: signals.set_pending_navigation,
            current_repo: signals.current_repo,
            active_branch: signals.active_branch,
            pending_branch_switch: signals.pending_branch_switch,
            pending_branch_switch_nonce: signals.pending_branch_switch_nonce,
            pending_repo_switch: signals.pending_repo_switch,
            pending_repo_switch_nonce: signals.pending_repo_switch_nonce,
            set_handshake_ready: signals.set_handshake_ready,
            set_handshake_scope_nonce: signals.set_handshake_scope_nonce,
            set_pending_branch_switch: signals.set_pending_branch_switch,
            set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
            set_pending_repo_switch: signals.set_pending_repo_switch,
            set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
        },
    );

    let state = build_core_state(
        ws,
        &signals,
        status_text,
        CoreStateCallbacks::new(
            doc_callbacks,
            sync_callbacks,
            sc_callbacks,
            misc_callbacks,
            switch_callbacks,
        ),
    );

    provide_context(state.clone());
    provide::provide_sub_contexts(&state);
    provide_context(contexts::DashboardContext {
        metrics: signals.system_metrics,
    });
    state
}
