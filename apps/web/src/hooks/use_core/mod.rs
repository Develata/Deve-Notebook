// apps/web/src/hooks/use_core/mod.rs
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! # Core State Hook (核心状态钩子)
//!
//! 管理前端全局核心状态，并把 WebLightPeer 的浏览器存储分层接入 UI。

pub mod apply;
pub mod callbacks;
mod callbacks_build;
pub mod callbacks_sc;
mod callbacks_sc_scope;
mod callbacks_sc_target;
pub(crate) mod callbacks_scope;
mod callbacks_switch;
mod callbacks_sync;
pub mod contexts;
mod dashboard_context;
pub mod diff_session;
pub(crate) mod doc_name;
pub mod effects;
pub mod effects_sc;
mod effects_sc_apply;
mod effects_sc_feedback;
mod effects_sc_scope;
mod effects_sc_state;
pub mod effects_switch;
pub mod navigation;
mod provide;
mod scope_prefs;
pub(crate) mod source_control_notice;
pub mod state;
mod state_build;
mod state_callbacks;
mod state_init;
pub(crate) mod status_summary;
mod status_text;
mod storage_runtime;
mod switch_nonce;
pub(crate) mod sync_banner_notice;
pub mod types;
pub(crate) mod write_gate;
pub(crate) mod write_gate_banner;

pub(crate) use callbacks_sc_target::can_request_doc_diff;
pub use contexts::*;
pub use types::*;

use crate::api::{ConnectionStatus, WsService};
use leptos::prelude::*;

use self::callbacks_build::build_callbacks;
use self::scope_prefs::{restore_scope_pref, setup_scope_pref_effect};
use self::state_build::build_core_state;
use self::status_text::build_status_text;
use self::storage_runtime::init_storage_runtime;

/// 初始化核心状态钩子。
pub fn use_core() -> CoreState {
    let ws = WsService::new();
    provide_context(ws.clone());

    let signals = state::init_signals(ws.status);
    reset_dashboard_metrics_live_on_disconnect(ws.status, signals.set_system_metrics_live);
    restore_scope_pref(&signals);
    setup_scope_pref_effect(&signals);
    let status_text = build_status_text(&ws, &signals);

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
            current_scope_nonce: signals.current_scope_nonce,
            active_branch: signals.active_branch,
            pending_branch_switch: signals.pending_branch_switch,
            set_pending_branch_switch: signals.set_pending_branch_switch,
            set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
            pending_repo_switch: signals.pending_repo_switch,
            set_pending_repo_switch: signals.set_pending_repo_switch,
            set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
            handshake_scope_nonce: signals.handshake_scope_nonce,
            set_handshake_scope_nonce: signals.set_handshake_scope_nonce,
            handshake_retry_nonce: signals.handshake_retry_nonce,
            set_repo_list_request_id: signals.set_repo_list_request_id,
            set_doc_list_request_id: signals.set_doc_list_request_id,
            set_tree_request_id: signals.set_tree_request_id,
            set_handshake_ready: signals.set_handshake_ready,
        },
    );
    effects::setup_message_effect(&ws, &signals);

    let callbacks = build_callbacks(&ws, &signals);
    let state = build_core_state(ws, &signals, status_text, callbacks);

    provide_context(state.clone());
    provide::provide_sub_contexts(&state);
    provide_context(contexts::DashboardContext {
        metrics: signals.system_metrics,
        metrics_live: signals.system_metrics_live,
    });
    state
}

fn reset_dashboard_metrics_live_on_disconnect(
    status: ReadSignal<ConnectionStatus>,
    set_metrics_live: WriteSignal<bool>,
) {
    Effect::new(move |_| {
        if should_reset_dashboard_metrics_live(status.get()) {
            set_metrics_live.set(false);
        }
    });
}

fn should_reset_dashboard_metrics_live(status: ConnectionStatus) -> bool {
    status != ConnectionStatus::Connected
}

#[cfg(test)]
mod tests {
    use super::should_reset_dashboard_metrics_live;
    use crate::api::ConnectionStatus;

    #[test]
    fn dashboard_metrics_live_resets_on_disconnect_states() {
        assert!(!should_reset_dashboard_metrics_live(
            ConnectionStatus::Connected
        ));
        assert!(should_reset_dashboard_metrics_live(
            ConnectionStatus::Disconnected
        ));
        assert!(should_reset_dashboard_metrics_live(
            ConnectionStatus::Connecting
        ));
        assert!(should_reset_dashboard_metrics_live(
            ConnectionStatus::Unauthorized
        ));
    }
}
