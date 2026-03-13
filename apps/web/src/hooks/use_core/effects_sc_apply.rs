use crate::api::WsService;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use leptos::prelude::*;

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
    set_doc_list_request_id: WriteSignal<Option<String>>,
    set_tree_request_id: WriteSignal<Option<String>>,
    schedule_refresh: &dyn Fn(),
    ws: &WsService,
) {
    let conflict_tag = if has_conflict { " [冲突]" } else { "" };
    leptos::logging::log!("文件变更: {} ({}){}", path, change_type, conflict_tag);
    schedule_refresh();
    let request_id = uuid::Uuid::new_v4().to_string();
    set_doc_list_request_id.set(Some(request_id.clone()));
    set_tree_request_id.set(Some(request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::ListDocs { request_id });
}

pub(super) fn refresh_after_commit(
    commit_id: &str,
    set_changes_request_id: WriteSignal<Option<String>>,
    set_commit_history_request_id: WriteSignal<Option<String>>,
    ws: &WsService,
) {
    leptos::logging::log!("已提交: {}", commit_id);
    let changes_request_id = uuid::Uuid::new_v4().to_string();
    set_changes_request_id.set(Some(changes_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetChanges {
        request_id: changes_request_id,
    });
    let history_request_id = uuid::Uuid::new_v4().to_string();
    set_commit_history_request_id.set(Some(history_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetCommitHistory {
        request_id: history_request_id,
        limit: 50,
    });
}
