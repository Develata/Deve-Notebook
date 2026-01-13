//! # PlaybackController 组件 (PlaybackController Component)
//!
//! 历史回放控制器，允许用户拖动滑块查看文档的历史版本。

use leptos::prelude::*;
use crate::i18n::{Locale, t};

#[component]
pub fn PlaybackController(
    max_version: ReadSignal<u64>,
    current_version: ReadSignal<u64>,
    on_change: Box<dyn Fn(u64) + Send + Sync>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = event_target_value(&ev).parse::<u64>().unwrap_or(0);
        on_change(value);
    };

    view! {
        <div class="absolute bottom-0 left-0 right-0 bg-white border-t border-gray-200 p-4 shadow-lg flex items-center gap-4 z-40">
            <span class="text-xs font-mono text-gray-500">{move || t::playback::label(locale.get())}</span>
            
            <input 
                type="range" 
                min="0" 
                max=move || max_version.get().to_string()
                value=move || current_version.get().to_string()
                on:input=on_input
                class="flex-1 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
            />
            
            <div class="font-mono text-sm min-w-[3rem] text-right">
                {move || current_version.get()} <span class="text-gray-400">/ {move || max_version.get()}</span>
            </div>
        </div>
    }
}
