//! plan_ref:
//!   - 03_rendering#large-document-runtime
//!   - 03_rendering#document-authority-bridge
//!
//! Editor 主容器，负责挂载 CodeMirror、只读闸门与大纲侧栏。

use crate::api::WsService;
use crate::components::icons::PanelLeft;
use crate::components::layout_context::{EditorContentContext, OutlineControl};
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_tracked};
use crate::hooks::use_outline::use_outline;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::html::Div;
use leptos::prelude::*;

mod buffered_ops;
mod delta;
mod delta_input;
mod delta_input_forward;
mod delta_input_gate;
mod delta_input_state;
pub mod ffi;
mod handshake_reset;
mod hook;
mod hook_editor;
mod hook_effects;
mod hook_open;
mod hook_playback;
mod hook_runtime;
mod message_effect;
pub mod op_id;
mod open_scope;
pub mod playback;
pub mod prefetch;
mod request_key;
pub mod sync;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EditorStats {
    pub chars: usize,
    pub words: usize,
    pub lines: usize,
}

#[component]
pub fn Editor(
    doc_id: DocId,
    #[prop(optional)] on_stats: Option<Callback<EditorStats>>,
    #[prop(optional)] embedded: bool,
) -> impl IntoView {
    let editor_ref = NodeRef::<Div>::new();
    let state = hook::use_editor(doc_id, editor_ref, on_stats);
    let playback_version = state.playback_version;
    let content = state.content;
    let core = expect_context::<EditorContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let doc_version = core.doc_version;
    let ws = use_context::<WsService>().expect("WsService should be provided");
    provide_context(EditorContentContext { content });
    Effect::new(move |_| {
        let is_pb = playback_version.get() < doc_version.get();
        let write_blocked = repo_write_block_tracked(
            &ws,
            RepoWriteSignals {
                load_state: core.load_state,
                is_spectator: core.is_spectator,
                handshake_ready: core.handshake_ready,
                current_repo_id: core.current_repo_id,
                current_scope_nonce: core.current_scope_nonce,
                active_branch: core.active_branch,
                pending_branch_switch: core.pending_branch_switch,
                pending_repo_switch: core.pending_repo_switch,
            },
        )
        .is_some();
        let should_readonly = should_editor_be_read_only(is_pb, write_blocked);
        ffi::set_read_only(should_readonly);
    });
    let (outline_pref, set_outline_pref) = use_context::<OutlineControl>()
        .map(|outline| (outline.visible, outline.set_visible))
        .unwrap_or_else(use_outline);
    let show_outline = Signal::derive(move || !embedded && outline_pref.get());
    let on_toggle_outline = Callback::new(move |_| set_outline_pref.update(|b| *b = !*b));
    let on_scroll = Callback::new(move |line: usize| ffi::scroll_global(line));

    view! {
        <div class="relative w-full h-full flex flex-col overflow-hidden">
            <div class="flex-1 flex overflow-hidden relative">
                <div
                    data-deve-desktop-col="3-editor"
                    class="flex-1 relative border-r border-gray-200 bg-white shadow-sm overflow-hidden"
                >
                    <div
                        node_ref=editor_ref
                        class="absolute inset-0"
                        class:bg-gray-100=move || playback_version.get() < doc_version.get()
                    ></div>
                    {move || if !embedded && playback_version.get() < doc_version.get() {
                        view! {
                            <div class="absolute top-2 left-1/2 -translate-x-1/2 z-[var(--z-floating)] px-3 py-1 bg-yellow-100 text-yellow-800 text-xs font-semibold rounded-full shadow-sm border border-yellow-200 pointer-events-none opacity-80 backdrop-blur-sm">
                                {move || t::common::spectator_status(locale.get())}
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                     {if !embedded {
                         view! {
                             <button
                                on:click=move |_| on_toggle_outline.run(())
                                class="absolute top-2 right-4 z-[var(--z-floating)] p-1.5 text-gray-500 hover:text-gray-700 hover:bg-gray-100 bg-white/90 border border-gray-200 rounded shadow-sm transition-all"
                                title=move || t::header::toggle_outline(locale.get())
                             >
                                <PanelLeft class="w-5 h-5"/>
                             </button>
                         }.into_any()
                     } else {
                         view! {}.into_any()
                     }}
                </div>
                {if !embedded {
                    view! {
                        <div
                            data-deve-desktop-col="4-outline"
                            class="bg-[var(--bg-sidebar)] border-l border-gray-200 transition-all duration-300 ease-in-out overflow-hidden"
                            style=move || if show_outline.get() { "width: 250px; opacity: 1;" } else { "width: 0px; opacity: 0;" }
                        >
                            <crate::components::outline::Outline
                                content=content
                                on_scroll=on_scroll
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </div>
    }
}

fn should_editor_be_read_only(is_playback: bool, write_blocked: bool) -> bool {
    is_playback || write_blocked
}

#[cfg(test)]
mod tests {
    use super::should_editor_be_read_only;

    #[test]
    fn editor_read_only_gate_blocks_native_runtime_write_gate() {
        assert!(should_editor_be_read_only(false, true));
    }

    #[test]
    fn editor_read_only_gate_allows_ready_writable_document() {
        assert!(!should_editor_be_read_only(false, false));
    }

    #[test]
    fn editor_read_only_gate_blocks_playback() {
        assert!(should_editor_be_read_only(true, false));
    }
}
