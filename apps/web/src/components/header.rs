// apps\web\src\components
//! plan_ref:
//!   - 11_ui_design_01_web#web-layout-persistence
//!   - 04_repository#repo-scope-runtime
//!
//! # Header 组件 (Header Component)
//!
//! 应用程序顶部导航栏，包含标题、状态指示器和常用操作（主页、打开、命令）。

use crate::components::icons::{Book, Home, Terminal};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn Header(
    #[prop(into)] status_text: Signal<String>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_logout: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <header class="w-full h-12 bg-panel border-b border-default flex items-center justify-between px-4 shadow-sm z-[var(--z-chrome)]">
            // 左侧: 标题与状态
            <div class="flex items-center gap-2">
                <span class="font-bold text-primary text-lg">{move || t::app_title(locale.get())}</span>
                <span class="text-xs text-muted border border-default rounded px-1">{move || status_text.get()}</span>
            </div>

            // 右侧: SB 风格操作 [Home] [Open] [Command]
            <div class="flex items-center gap-1">
                // 主页
                <button
                    class="p-2 text-secondary hover:bg-hover rounded transition-colors"
                    title=move || t::header::home(locale.get())
                    on:click=move |_| on_home.run(())
                >
                    <Home class="w-[18px] h-[18px]"/>
                </button>

                // 打开 (书籍)
                <button
                    class="p-2 text-secondary hover:bg-hover rounded transition-colors"
                    title=move || t::header::open(locale.get())
                    on:click=move |_| on_open.run(())
                >
                    <Book class="w-[18px] h-[18px]"/>
                </button>

                // 命令 (终端)
                <button
                    class="p-2 text-secondary hover:bg-hover rounded transition-colors"
                    title=move || t::header::command(locale.get())
                    on:click=move |_| on_command.run(())
                >
                    <Terminal class="w-[18px] h-[18px]"/>
                </button>

                <button
                    class="px-2 py-1 text-xs text-secondary hover:bg-hover rounded transition-colors"
                    title=move || t::header::logout(locale.get())
                    on:click=move |_| on_logout.run(())
                >
                    {move || t::header::logout(locale.get())}
                </button>
            </div>
        </header>
    }
}
