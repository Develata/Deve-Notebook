// apps\web\src\components
//! # InputModal 组件 (InputModal Component)
//!
//! 通用输入模态框，用于文件重命名、创建新文件等场景。

use leptos::prelude::*;

#[component]
pub fn InputModal(
    show: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
    #[prop(into)] title: Signal<String>,
    #[prop(into)] initial_value: Signal<Option<String>>,
    #[prop(into)] placeholder: Signal<String>,
    #[prop(into)] confirm_label: Signal<String>,
    #[prop(into)] on_confirm: Callback<String>,
) -> impl IntoView {
    let (value, set_value) = signal(String::new());
    
    // 显示时聚焦输入框并设置初始值
    let input_ref = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
         if show.get() {
             set_value.set(initial_value.get().unwrap_or_default());
             if let Some(el) = input_ref.get() {
                 let _ = el.focus();
                 // 如果是重命名，全选 (简单 hack: 超时或 set selection)
                 // 目前仅聚焦。
             }
         }
    });

    let submit = move || {
        let val = value.get();
        if !val.trim().is_empty() {
             on_confirm.run(val);
             set_show.set(false);
        }
    };
    
    view! {
        <div 
            class=move || if show.get() { 
                "fixed inset-0 z-[60]" // 无论何种情况都显示透明遮罩 (高 z-index)
            } else { 
                "hidden" 
            }
            on:click=move |_| set_show.set(false)
        >
            // 模态框主体 - 顶部居中浮动 (类似 CommandPalette)
            <div 
                class="absolute top-2 left-1/2 -translate-x-1/2 w-full max-w-xl bg-white rounded-lg shadow-xl border border-gray-200 overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-100"
                on:click=move |ev| ev.stop_propagation()
            >
                // 标题与输入容器
                <div class="flex flex-col">
                     // 可选标题头 (细微)
                     <div class="px-3 py-1.5 text-xs font-semibold text-gray-500 uppercase bg-gray-50/50 border-b border-gray-100">
                        {move || title.get()}
                     </div>

                     // 输入行
                     <div class="p-3 flex items-center gap-3">
                        // 图标 (通用编辑/输入)
                        <svg class="w-4 h-4 text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                        </svg>
                        
                        <input 
                            node_ref=input_ref
                            type="text"
                            class="flex-1 outline-none text-base bg-transparent text-gray-800 placeholder:text-gray-400"
                            placeholder=move || placeholder.get()
                            prop:value=value
                            on:input=move |ev| set_value.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    submit();
                                } else if ev.key() == "Escape" {
                                    set_show.set(false);
                                }
                            }
                        />
                     </div>
                </div>
                
                // 底部提示
                <div class="bg-gray-50 px-4 py-2 border-t border-gray-100 flex justify-end items-center text-xs text-gray-500 gap-4">
                     <span class="flex items-center gap-1">
                        <kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Enter</kbd> 
                        <span>{move || confirm_label.get()}</span>
                     </span>
                     <span class="flex items-center gap-1">
                        <kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Esc</kbd> 
                        <span>"Cancel"</span>
                     </span>
                </div>
            </div>
        </div>
    }
}
