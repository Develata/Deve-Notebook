//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! 子上下文提供入口。

mod branch;
mod chat;
mod doc;
mod editor;
mod external_changes;
mod source_control;
mod sync;

use super::types::CoreState;
use leptos::prelude::*;

/// 从已构建的 CoreState 提取字段，构造并提供 6 个独立上下文。
///
/// ## Invariant
/// 子上下文与 CoreState 共享同一组 Signal —— 无额外分配。
pub fn provide_sub_contexts(state: &CoreState) {
    provide_context(state.runtime_clients.clone());
    provide_context(state.runtime_clients.session.clone());
    provide_context(state.runtime_clients.scope.clone());
    provide_context(state.runtime_clients.document.clone());
    provide_context(state.runtime_clients.source_control.clone());
    provide_context(state.runtime_clients.external_changes.clone());
    provide_context(state.runtime_clients.rendering.clone());
    provide_context(doc::build_doc_context(state));
    provide_context(editor::build_editor_context(state));
    provide_context(chat::build_chat_context(state));
    provide_context(sync::build_sync_context(state));
    provide_context(source_control::build_source_control_context(state));
    provide_context(external_changes::build_external_changes_context(state));
    provide_context(branch::build_branch_context(state));
}
