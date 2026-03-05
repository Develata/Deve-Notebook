// apps\web\src\components\sidebar
//! # ExtensionsView 组件 (ExtensionsView Component)
//!
//! 扩展视图的占位符。计划在第三阶段实现插件系统。

use crate::components::icons::Puzzle;
use crate::i18n::{t, Locale};
use leptos::prelude::*;

#[component]
pub fn ExtensionsView() -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
         <div class="h-full w-full bg-sidebar flex flex-col">
            <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-default">
                 <span class="font-medium text-sm text-primary">{move || t::sidebar::extensions(locale.get())}</span>
            </div>
            <div class="flex-1 flex flex-col items-center justify-center text-muted p-4 text-center">
                <Puzzle class="w-12 h-12 mb-2" />
                <p class="text-sm">{move || t::sidebar::extensions_desc(locale.get())}</p>
            </div>
        </div>
    }
}
