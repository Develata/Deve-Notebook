// apps\web\src\components
//! # Header 组件 (Header Component)
//!
//! 应用程序顶部导航栏，包含标题、状态指示器和常用操作（主页、打开、命令）。

use leptos::prelude::*;
use crate::i18n::{Locale, t};

#[component]
pub fn Header(
    #[prop(into)] status_text: Signal<String>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <header class="w-full h-12 bg-white border-b border-gray-200 flex items-center justify-between px-4 shadow-sm z-50">
            // 左侧: 标题与状态
            <div class="flex items-center gap-2">
                <span class="font-bold text-gray-800 text-lg">{move || t::app_title(locale.get())}</span>
                <span class="text-xs text-gray-400 border border-gray-200 rounded px-1">{move || status_text.get()}</span>
            </div>
            
            // 右侧: SB 风格操作 [Home] [Open] [Command]
            <div class="flex items-center gap-1">
                // 主页
                <button 
                    class="p-2 text-gray-600 hover:bg-gray-100 rounded transition-colors"
                    title=move || t::header::home(locale.get())
                    on:click=move |_| on_home.run(())
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
                        <polyline points="9 22 9 12 15 12 15 22"></polyline>
                    </svg>
                </button>
                
                // 打开 (书籍)
                <button 
                    class="p-2 text-gray-600 hover:bg-gray-100 rounded transition-colors"
                    title=move || t::header::open(locale.get())
                    on:click=move |_| on_open.run(())
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
                        <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
                    </svg>
                </button>
                
                // 命令 (终端)
                <button 
                    class="p-2 text-gray-600 hover:bg-gray-100 rounded transition-colors"
                    title=move || t::header::command(locale.get())
                    on:click=move |_| on_command.run(())
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                         <polyline points="4 17 10 11 4 5"></polyline>
                         <line x1="12" y1="19" x2="20" y2="19"></line>
                    </svg>
                </button>
            </div>
        </header>
    }
}
