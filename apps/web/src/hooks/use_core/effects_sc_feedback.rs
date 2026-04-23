use crate::storage::DegradedSyncMode;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;

#[cfg(test)]
#[path = "effects_sc_feedback_test.rs"]
mod tests;

pub(crate) fn show_file_op_feedback(
    path: &str,
    change_type: &str,
    has_conflict: bool,
    degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    sync_banner: ReadSignal<Option<String>>,
    set_sync_banner: WriteSignal<Option<String>>,
) {
    if has_conflict || degraded_sync_mode.get_untracked().is_some() {
        return;
    }
    if let Some(message) = file_op_feedback_message(path, change_type) {
        show_temporary_banner(sync_banner, set_sync_banner, message);
    }
}

pub(crate) fn show_sc_ack_feedback(
    message: String,
    degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    sync_banner: ReadSignal<Option<String>>,
    set_sync_banner: WriteSignal<Option<String>>,
) {
    if degraded_sync_mode.get_untracked().is_none() {
        show_temporary_banner(sync_banner, set_sync_banner, message);
    }
}

fn show_temporary_banner(
    sync_banner: ReadSignal<Option<String>>,
    set_sync_banner: WriteSignal<Option<String>>,
    message: String,
) {
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

pub(crate) fn source_control_ack_message(action: &str, path: &str) -> String {
    format!("{}: {}", action, path)
}

pub(crate) fn commit_ack_message(commit_id: &str) -> String {
    format!(
        "Committed: {}",
        commit_id.chars().take(7).collect::<String>()
    )
}
