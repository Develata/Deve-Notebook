//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#web-ws-runtime
//!
use super::context::SyncContext;
use crate::api::WsService;
use crate::editor::EditorStats;
use crate::editor::ffi::getEditorContent;
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

#[derive(Clone)]
pub(super) struct LoadFinish {
    doc_id: DocId,
    version: u64,
    load_start: f64,
    request_id: u64,
    scope_nonce: u64,
    ws: WsService,
    set_content: WriteSignal<String>,
    set_playback_version: WriteSignal<u64>,
    set_load_progress: WriteSignal<(usize, usize)>,
    set_load_eta_ms: WriteSignal<u64>,
    on_stats: Option<Callback<EditorStats>>,
}

impl LoadFinish {
    pub(super) fn from_ctx(
        ctx: &SyncContext,
        version: u64,
        load_start: f64,
        request_id: u64,
    ) -> Self {
        Self {
            doc_id: ctx.doc_id,
            version,
            load_start,
            request_id,
            scope_nonce: ctx.current_scope_nonce.get_untracked(),
            ws: ctx.ws.clone(),
            set_content: ctx.set_content,
            set_playback_version: ctx.set_playback_version,
            set_load_progress: ctx.set_load_progress,
            set_load_eta_ms: ctx.set_load_eta_ms,
            on_stats: ctx.on_stats,
        }
    }

    pub(super) fn complete(self) {
        let text = getEditorContent();
        self.complete_with_content(text);
    }

    pub(super) fn complete_with_content(self, text: String) {
        emit_stats(self.on_stats, &text);
        self.set_content.set(text);
        self.set_playback_version.set(self.version);
        self.set_load_progress.set((0, 0));
        self.set_load_eta_ms.set(0);
        leptos::logging::log!(
            "Snapshot load complete: doc={}, elapsed_ms={}",
            self.doc_id,
            (now_ms() - self.load_start) as u64
        );
        self.ws.send(ClientMessage::RequestHistory {
            doc_id: self.doc_id,
            request_id: self.request_id,
            scope_nonce: Some(self.scope_nonce),
        });
    }
}

pub(super) fn finalize_load(ctx: &SyncContext, version: u64, load_start: f64) {
    LoadFinish::from_ctx(
        ctx,
        version,
        load_start,
        ctx.open_request_id.get_untracked(),
    )
    .complete();
}

pub(super) fn emit_stats(on_stats: Option<Callback<EditorStats>>, text: &str) {
    if let Some(cb) = on_stats {
        cb.run(EditorStats {
            chars: text.len(),
            words: text.split_whitespace().count(),
            lines: text.lines().count(),
        });
    }
}

pub(super) fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|perf| perf.now())
        .unwrap_or(0.0)
}
