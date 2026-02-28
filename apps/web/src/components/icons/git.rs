//! Git / 版本控制相关图标
#![allow(dead_code)]

use leptos::prelude::*;

/// SVG 公共属性宏
macro_rules! icon {
    ($name:ident, $body:expr) => {
        #[component]
        pub fn $name(#[prop(default = "w-4 h-4".to_string(), into)] class: String) -> impl IntoView {
            let cls = class;
            view! {
                <svg class=cls xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
                    fill="none" stroke="currentColor" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round">
                    {$body}
                </svg>
            }
        }
    };
}

icon!(GitBranch, view! {
    <line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/>
    <circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>
});

icon!(Sparkles, view! {
    <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/>
    <path d="M20 3v4"/><path d="M22 5h-4"/>
});
