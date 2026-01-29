// apps/web/src/hooks/use_core/callbacks_sc.rs
//! # Source Control 回调函数
//!
//! 处理 Git 风格的版本控制操作回调。

use crate::api::WsService;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

/// Source Control 回调结构体
pub struct SourceControlCallbacks {
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<String>,
    pub on_unstage_file: Callback<String>,
    pub on_discard_file: Callback<String>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub on_get_doc_diff: Callback<String>,
}

/// 创建 Source Control 回调
pub fn create_source_control_callbacks(ws: &WsService) -> SourceControlCallbacks {
    let ws1 = ws.clone();
    let on_get_changes = Callback::new(move |_: ()| {
        ws1.send(ClientMessage::GetChanges);
    });

    let ws2 = ws.clone();
    let on_stage_file = Callback::new(move |path: String| {
        ws2.send(ClientMessage::StageFile { path });
    });

    let ws3 = ws.clone();
    let on_unstage_file = Callback::new(move |path: String| {
        ws3.send(ClientMessage::UnstageFile { path });
    });

    let ws4 = ws.clone();
    let on_commit = Callback::new(move |message: String| {
        ws4.send(ClientMessage::Commit { message });
    });

    let ws5 = ws.clone();
    let on_get_history = Callback::new(move |limit: u32| {
        ws5.send(ClientMessage::GetCommitHistory { limit });
    });

    let ws6 = ws.clone();
    let on_get_doc_diff = Callback::new(move |path: String| {
        ws6.send(ClientMessage::GetDocDiff { path });
    });

    let ws7 = ws.clone();
    let on_discard_file = Callback::new(move |path: String| {
        leptos::logging::log!("on_discard_file callback triggered for: {}", path);
        ws7.send(ClientMessage::DiscardFile { path });
    });

    SourceControlCallbacks {
        on_get_changes,
        on_stage_file,
        on_unstage_file,
        on_discard_file,
        on_commit,
        on_get_history,
        on_get_doc_diff,
    }
}
