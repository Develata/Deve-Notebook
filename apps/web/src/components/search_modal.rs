//! # SearchModal 组件 (SearchModal Component)
//!
//! **架构作用**:
//! 提供 "Go to file..." (Ctrl+P 风格) 的全文搜索弹窗。
//!
//! **核心功能清单**:
//! - `SearchModal`: 渲染搜索输入框与结果列表。
//! - 监听 Ctrl+P 快捷键触发显示。
//! - 将选中的结果传递给 `on_doc_select`。
//!
//! **类型**: Plugin MAY (插件可选) - 仅 Standard Profile 启用

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use deve_core::models::DocId;
use std::str::FromStr;

#[component]
pub fn SearchModal(
    /// 模态框是否可见
    #[prop(into)]
    show: Signal<bool>,
    /// 切换可见性的回调
    on_close: Callback<()>,
    /// 搜索文本输入回调
    on_search: Callback<String>,
    /// 搜索结果信号
    #[prop(into)]
    search_results: Signal<Vec<(String, String, f32)>>,
    /// 选中结果时的回调
    on_select: Callback<DocId>,
) -> impl IntoView {
    let (query, set_query) = signal(String::new());
    
    // 防抖搜索
    Effect::new(move |_| {
        let q = query.get();
        if !q.is_empty() {
            on_search.run(q);
        }
    });

    let handle_input = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.unchecked_into();
        set_query.set(input.value());
    };

    let handle_select = move |doc_id_str: String| {
        if let Ok(uuid) = uuid::Uuid::from_str(&doc_id_str) {
            on_select.run(DocId(uuid));
            on_close.run(());
        }
    };

    view! {
        <div
            class="search-modal-overlay"
            style:display=move || if show.get() { "flex" } else { "none" }
            on:click=move |_| on_close.run(())
        >
            <div
                class="search-modal"
                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
            >
                <input
                    type="text"
                    class="search-input"
                    placeholder="Go to file... (type to search)"
                    prop:value=move || query.get()
                    on:input=handle_input
                    autofocus=true
                />
                <div class="search-results">
                    <For
                        each=move || search_results.get()
                        key=|(id, _, _)| id.clone()
                        children=move |(doc_id, path, score)| {
                            let doc_id_clone = doc_id.clone();
                            view! {
                                <div
                                    class="search-result-item"
                                    on:click=move |_| handle_select(doc_id_clone.clone())
                                >
                                    <span class="search-result-path">{path.clone()}</span>
                                    <span class="search-result-score">{format!("{:.2}", score)}</span>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}
