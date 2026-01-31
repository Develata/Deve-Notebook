// apps\web\src\editor
//! # Editor Component (Editor 组件)
//!
//! **架构作用**:
//! 编辑器的主 UI 容器。
//! 整合了 CodeMirror (通过 `hook.rs`)，大纲视图 (`Outline`)，以及旁观者模式/历史回放的状态展示。
//!
//! **核心功能清单**:
//! - `Editor`: 主组件。
//! - 渲染 CodeMirror 的挂载点。
//! - 显示 "Spectator Mode" (旁观者模式) 提示。
//! - 管理大纲视图的显示/隐藏。

use crate::hooks::use_core::CoreState;
use deve_core::models::DocId;
use leptos::html::Div;
use leptos::prelude::*;

pub mod ffi;
pub mod hook;
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

    // 使用 hook 逻辑
    let state = hook::use_editor(doc_id, editor_ref, on_stats);

    // 解包状态
    let local_version = state.local_version;
    let playback_version = state.playback_version;
    let content = state.content;

    // 获取 CoreState 用于 Spectator 模式
    let core = expect_context::<CoreState>();

    // 监听 is_spectator 信号，切换编辑器只读状态
    Effect::new(move |_| {
        let spectator = core.is_spectator.get();
        let is_pb = playback_version.get() < local_version.get_untracked();
        let loading = core.load_state.get() != "ready";
        let should_readonly = spectator || is_pb || loading;
        ffi::set_read_only(should_readonly);
    });

    // 大纲状态 (嵌入模式下默认禁用且不显示)
    let (show_outline, set_show_outline) = signal(!embedded);
    let on_toggle_outline = Callback::new(move |_| set_show_outline.update(|b| *b = !*b));

    let on_scroll = Callback::new(move |line: usize| {
        ffi::scroll_global(line);
    });

    view! {
        // 主容器: 相对定位用于回放定位，100% 尺寸
        <div class="relative w-full h-full flex flex-col overflow-hidden">
            // 内容区域 (Flex Row)
            <div class="flex-1 flex overflow-hidden relative">
                // 编辑器包装器
                <div class="flex-1 relative border-r border-gray-200 bg-white shadow-sm overflow-hidden">
                    <div
                        node_ref=editor_ref
                        class="absolute inset-0"
                        class:bg-gray-100=move || playback_version.get() < local_version.get()
                    ></div>

                    // 旁观者模式徽章 (嵌入模式不显示, 或者 DiffMode 可能不需要?)
                    // 嵌入模式下我们通常不需要 spectator badge, 因为 Diff View 本身有上下文
                    {move || if !embedded && playback_version.get() < local_version.get() {
                        view! {
                            <div class="absolute top-2 left-1/2 -translate-x-1/2 z-50 px-3 py-1 bg-yellow-100 text-yellow-800 text-xs font-semibold rounded-full shadow-sm border border-yellow-200 pointer-events-none opacity-80 backdrop-blur-sm">
                                "Spectator Mode (Read Only)"
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}

                     // 切换大纲按钮 (嵌入模式下隐藏)
                     {if !embedded {
                         view! {
                             <button
                                on:click=move |_| on_toggle_outline.run(())
                                class="absolute top-2 right-4 z-50 p-1.5 text-gray-500 hover:text-gray-700 hover:bg-gray-100 bg-white/90 border border-gray-200 rounded shadow-sm transition-all"
                                title="Toggle Outline"
                             >
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5">
                                  <path fill-rule="evenodd" d="M3 4.25A2.25 2.25 0 015.25 2h9.5A2.25 2.25 0 0117 4.25v11.5A2.25 2.25 0 0114.75 18h-9.5A2.25 2.25 0 013 15.75V4.25zM6 13a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm3 10a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm7 5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2z" clip-rule="evenodd" />
                                </svg>
                             </button>
                         }.into_any()
                     } else {
                         view! {}.into_any()
                     }}
                </div>

                // 大纲侧边栏 (嵌入模式下不渲染)
                {if !embedded {
                    view! {
                        <div
                            class="bg-[#f9f9f9] border-l border-gray-200 transition-all duration-300 ease-in-out overflow-hidden"
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
