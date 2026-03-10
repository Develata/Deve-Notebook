//! Editor 主容器，负责挂载 CodeMirror、只读闸门与大纲侧栏。

use crate::api::WsService;
use crate::components::icons::PanelLeft;
use crate::components::layout_context::EditorContentContext;
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_outline::use_outline;
use deve_core::models::DocId;
use leptos::html::Div;
use leptos::prelude::*;

mod delta_input;
pub mod ffi;
pub mod hook;
pub mod op_id;
pub mod playback;
pub mod prefetch;
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
    let local_version = state.local_version;
    let playback_version = state.playback_version;
    let content = state.content;
    let core = expect_context::<EditorContext>();
    let ws = use_context::<WsService>().expect("WsService should be provided");
    provide_context(EditorContentContext { content });
    Effect::new(move |_| {
        let spectator = core.is_spectator.get();
        let is_pb = playback_version.get() < local_version.get_untracked();
        let loading = core.load_state.get() != "ready";
        let handshake_ready = core.handshake_ready.get();
        let writer_ready = ws.writer_ready_repo_id.get() == core.current_repo_id.get();
        let should_readonly = spectator || is_pb || loading || !handshake_ready || !writer_ready;
        ffi::set_read_only(should_readonly);
    });
    let (outline_pref, set_outline_pref) = use_outline();
    let show_outline = Signal::derive(move || !embedded && outline_pref.get());
    let on_toggle_outline = Callback::new(move |_| set_outline_pref.update(|b| *b = !*b));
    let on_scroll = Callback::new(move |line: usize| ffi::scroll_global(line));

    view! {
        <div class="relative w-full h-full flex flex-col overflow-hidden">
            <div class="flex-1 flex overflow-hidden relative">
                <div class="flex-1 relative border-r border-gray-200 bg-white shadow-sm overflow-hidden">
                    <div
                        node_ref=editor_ref
                        class="absolute inset-0"
                        class:bg-gray-100=move || playback_version.get() < local_version.get()
                    ></div>
                    {move || if !embedded && playback_version.get() < local_version.get() {
                        view! {
                            <div class="absolute top-2 left-1/2 -translate-x-1/2 z-50 px-3 py-1 bg-yellow-100 text-yellow-800 text-xs font-semibold rounded-full shadow-sm border border-yellow-200 pointer-events-none opacity-80 backdrop-blur-sm">
                                "Spectator Mode (Read Only)"
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                     {if !embedded {
                         view! {
                             <button
                                on:click=move |_| on_toggle_outline.run(())
                                class="absolute top-2 right-4 z-50 p-1.5 text-gray-500 hover:text-gray-700 hover:bg-gray-100 bg-white/90 border border-gray-200 rounded shadow-sm transition-all"
                                title="Toggle Outline"
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
