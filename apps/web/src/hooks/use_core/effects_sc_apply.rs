use crate::api::WsService;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::storage::DegradedSyncMode;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;

#[cfg(test)]
#[path = "effects_sc_apply_test.rs"]
mod tests;

pub(super) struct FsRefreshSignals {
    pub current_scope_nonce: u64,
    pub degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    pub sync_banner: ReadSignal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,
}

pub(super) fn apply_doc_diff(
    path: &str,
    old_content: &str,
    new_content: &str,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
) {
    leptos::logging::log!("收到 Diff: {}", path);
    set_diff.set(Some(DiffSessionWire::new(
        path.to_string(),
        old_content.to_string(),
        new_content.to_string(),
    )));
    let ranges =
        deve_core::source_control::line_diff::compute_line_ranges(old_content, new_content);
    if let Ok(json) = serde_json::to_string(&ranges) {
        crate::editor::ffi::update_gutter_diff(&json);
    }
}

pub(super) fn refresh_after_fs_change(
    path: &str,
    change_type: &str,
    has_conflict: bool,
    signals: FsRefreshSignals,
    schedule_refresh: &dyn Fn(),
    ws: &WsService,
) {
    let conflict_tag = if has_conflict { " [冲突]" } else { "" };
    if has_conflict || change_type != "dir_changed" {
        leptos::logging::log!("文件变更: {} ({}){}", path, change_type, conflict_tag);
    }
    show_file_op_feedback(path, change_type, has_conflict, &signals);
    schedule_refresh();
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_doc_list_request_id
        .set(Some(request_id.clone()));
    signals.set_tree_request_id.set(Some(request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::ListDocs {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce),
    });
}

fn show_file_op_feedback(
    path: &str,
    change_type: &str,
    has_conflict: bool,
    signals: &FsRefreshSignals,
) {
    if has_conflict || signals.degraded_sync_mode.get_untracked().is_some() {
        return;
    }
    let Some(message) = file_op_feedback_message(path, change_type) else {
        return;
    };
    let sync_banner = signals.sync_banner;
    let set_sync_banner = signals.set_sync_banner;
    set_sync_banner.set(Some(message.clone()));
    Timeout::new(1800, move || {
        if sync_banner.get_untracked().as_deref() == Some(message.as_str()) {
            set_sync_banner.set(None);
        }
    })
    .forget();
}

fn file_op_feedback_message(path: &str, change_type: &str) -> Option<String> {
    let action = match change_type {
        "added" => "Created",
        "dir-added" => "Created folder",
        "renamed" => "Renamed",
        "deleted" => "Deleted",
        "copied" => "Copied",
        _ => return None,
    };
    Some(format!("{}: {}", action, path))
}

pub(super) fn refresh_after_commit(
    commit_id: &str,
    current_scope_nonce: u64,
    set_changes_request_id: WriteSignal<Option<String>>,
    set_commit_history_request_id: WriteSignal<Option<String>>,
    ws: &WsService,
) {
    leptos::logging::log!("已提交: {}", commit_id);
    let changes_request_id = uuid::Uuid::new_v4().to_string();
    set_changes_request_id.set(Some(changes_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetChanges {
        request_id: changes_request_id,
        scope_nonce: Some(current_scope_nonce),
    });
    let history_request_id = uuid::Uuid::new_v4().to_string();
    set_commit_history_request_id.set(Some(history_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetCommitHistory {
        request_id: history_request_id,
        limit: 50,
        scope_nonce: Some(current_scope_nonce),
    });
}
