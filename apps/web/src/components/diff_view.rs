// apps\web\src\components
//! # Diff 视图组件 (Diff View)
//!
//! 并排显示文档的新旧版本差异。
//!
//! **功能**:
//! - 左侧: 已提交版本 (旧)
//! - 右侧: 当前版本 (新)
//! - 高亮显示差异行

use leptos::prelude::*;
// use leptos::task::spawn_local; // Removed
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Diff 视图组件
#[component]
pub fn DiffView<F>(
    /// 文件路径
    path: String,
    /// 已提交版本内容 (Old)
    old_content: String,
    /// 当前版本内容 (New, unused in editable mode as Editor fetches it)
    _new_content: String,
    /// 关闭回调
    on_close: F,
) -> impl IntoView
where
    F: Fn() + 'static + Clone,
{
    let old_lines: Vec<_> = old_content.lines().map(String::from).collect();
    // New content is managed by Editor

    // 提取文件名和目录
    let full_path = path.clone();
    let normalized_path = full_path.replace('\\', "/");
    let path_parts: Vec<&str> = normalized_path.split('/').collect();
    let filename = path_parts.last().unwrap_or(&"?").to_string();
    let directory = if path_parts.len() > 1 {
        path_parts[..path_parts.len() - 1].join("/")
    } else {
        String::new()
    };

    // 查找 DocId
    let core = expect_context::<crate::hooks::use_core::CoreState>();
    let doc_id = core
        .docs
        .get()
        .iter()
        .find(|(_, p)| p.replace('\\', "/") == normalized_path)
        .map(|(id, _)| *id);

    // Refs for scroll sync
    let left_container_ref = NodeRef::<leptos::html::Div>::new();
    let right_container_ref = NodeRef::<leptos::html::Div>::new();

    // Sync Scroll Logic: Right (Editor) -> Left (Static)
    // 注意: CodeMirror 的滚动容器是 .cm-scroller
    Effect::new(move |_| {
        if let Some(right_div) = right_container_ref.get() {
            // 需要延迟一点等待 CodeMirror 渲染? 或者轮询?
            // 简单的做法是监听 capture 下的 scroll 事件
            // 由于 leptos event delegation, 我们需要直接操作 DOM

            // 尝试查找 scroller
            let find_scroller = move || {
                use wasm_bindgen::JsCast;
                let el: web_sys::HtmlElement = right_div.into();
                // 查找 .cm-scroller
                let collection = el.get_elements_by_class_name("cm-scroller");
                if let Some(element) = collection.item(0) {
                    if let Ok(scroller) = element.dyn_into::<web_sys::HtmlElement>() {
                        return Some(scroller);
                    }
                }
                None
            };

            // 这是一个一次性尝试，实际可能需要 MutationObserver 或定时器
            // 这里我们使用 set_timeout 简单尝试绑定
            let left_ref = left_container_ref;

            request_animation_frame(move || {
                let handle_scroll = Closure::wrap(Box::new(move |ev: web_sys::Event| {
                    if let Some(target) = ev.target() {
                        if let Some(el) = target.dyn_ref::<web_sys::HtmlElement>() {
                            let top = el.scroll_top();
                            // Sync to Left
                            if let Some(left) = left_ref.get() {
                                left.set_scroll_top(top);
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);

                // 简单的轮询绑定 (最多尝试 5 次)
                leptos::task::spawn_local(async move {
                    for _ in 0..10 {
                        if let Some(scroller) = find_scroller() {
                            let _ = scroller.add_event_listener_with_callback(
                                "scroll",
                                handle_scroll.as_ref().unchecked_ref(),
                            );
                            handle_scroll.forget(); // Leak to keep listener
                            break;
                        }
                        gloo_timers::future::TimeoutFuture::new(200).await;
                    }
                });
            });
        }
    });

    view! {
        // bg-white for Light, bg-[#1e1e1e] for Dark
        <div class="diff-view h-full flex flex-col bg-white dark:bg-[#1e1e1e] text-gray-800 dark:text-[#cccccc] font-sans">
            // 标题栏
            <div class="diff-header flex justify-between items-center border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-[#2d2d2d] px-4 py-2 text-xs select-none">
                <div class="flex items-center gap-2">
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-blue-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>

                    <span class="font-bold text-gray-700 dark:text-[#cccccc]">{filename}</span>
                    <span class="text-gray-400 dark:text-gray-500 ml-1">{directory}</span>

                    <span class="ml-2 px-1.5 py-0.5 rounded text-[10px] bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400">
                        "Working Tree (Editable)"
                    </span>
                </div>

                <button
                    class="p-1 hover:bg-gray-200 dark:hover:bg-[#ffffff1a] rounded text-gray-500 dark:text-gray-400 transition-colors"
                    title="Close Diff View (Esc)"
                    on:click=move |_| on_close()
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </button>
            </div>

            // 内容区域
            <div class="diff-content flex-1 flex overflow-hidden font-mono text-sm leading-6">
                // 左侧 (已提交 - ReadOnly Static)
                // 注意: 这里我们仍然使用行渲染，但为了 sync scroll，我们需要它只是一个可滚动的 div
                // 使用一样的 line-height 和 padding
                <div
                    node_ref=left_container_ref
                    class="flex-1 overflow-auto bg-gray-50/30 dark:bg-[#1e1e1e] select-none"
                    style="scrollbar-width: thin;" // Hide scrollbar if possible or sync?
                >
                    {(0..old_lines.len()).map(|i| {
                        let old_line = old_lines.get(i).cloned().unwrap_or_default();
                        // 移除 diff 高亮逻辑，因为右侧是 Editor，无法简单对应
                        // 或者我们可以保留基本的红色用于 delete?
                        // 但由于 Editor 处理 wrap，静态行高可能会偏移。
                        // MVP: 纯文本显示
                        view! {
                            <div class="px-4 whitespace-pre text-gray-500 dark:text-gray-400 opacity-70">
                                <span class="inline-block w-8 text-right mr-4 text-xs opacity-30 select-none border-r border-gray-300 pr-2">{i + 1}</span>
                                {old_line}
                            </div>
                        }
                    }).collect_view()}
                    // 填充底部空白以匹配编辑器滚动
                    <div class="h-[50vh]"></div>
                </div>

                // 右侧 (当前 - Editable Editor)
                <div
                    node_ref=right_container_ref
                    class="flex-1 overflow-hidden border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-[#1e1e1e]"
                >
                    {move || match doc_id {
                        Some(id) => view! {
                            <crate::editor::Editor doc_id=id embedded=true />
                        }.into_any(),
                        None => view! {
                            <div class="flex items-center justify-center h-full text-red-500">
                                "Document ID not found"
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
