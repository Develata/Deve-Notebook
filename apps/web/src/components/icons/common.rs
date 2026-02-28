//! 通用 UI 图标 (导航 + 操作 + 文档)
#![allow(dead_code)]

use leptos::prelude::*;

/// SVG 公共属性宏 (减少重复)
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

// ────── Navigation ──────

icon!(ChevronRight, view! { <path d="m9 18 6-6-6-6"/> });
icon!(ChevronDown,  view! { <path d="m6 9 6 6 6-6"/> });
icon!(ExternalLink, view! {
    <path d="M15 3h6v6"/><path d="M10 14 21 3"/>
    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
});

// ────── Actions ──────

icon!(Plus,     view! { <path d="M5 12h14"/><path d="M12 5v14"/> });
icon!(Minus,    view! { <path d="M5 12h14"/> });
icon!(Check,    view! { <path d="M20 6 9 17l-5-5"/> });

icon!(EllipsisVertical, view! {
    <circle cx="12" cy="12" r="1"/><circle cx="12" cy="5" r="1"/><circle cx="12" cy="19" r="1"/>
});
icon!(MoreHorizontal, view! {
    <circle cx="12" cy="12" r="1"/><circle cx="5" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>
});

icon!(RefreshCw, view! {
    <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/>
    <path d="M21 3v5h-5"/>
    <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/>
    <path d="M3 21v-5h5"/>
});
icon!(RotateCcw, view! {
    <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>
});

icon!(Upload, view! {
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
    <polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>
});
icon!(Download, view! {
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
    <polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>
});

// ────── Documents ──────

icon!(FileText, view! {
    <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/>
    <path d="M14 2v4a2 2 0 0 0 2 2h4"/>
    <path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>
});
icon!(FolderOpen, view! {
    <path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>
});
icon!(Puzzle, view! {
    <path d="M15.39 4.39a1 1 0 0 0 0 1.68l.3.18a1 1 0 0 1 0 1.74H13V6a1 1 0 0 0-2 0v1.97H8.31a1 1 0 0 1 0-1.74l.3-.18a1 1 0 0 0 0-1.68C8.08 3.55 7 2.65 7 2a3 3 0 0 1 5 0c0 .65-1.08 1.55-1.61 2.39Z"/>
    <rect width="12" height="12" x="2" y="10" rx="2"/>
});
icon!(Book, view! {
    <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/>
    <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>
});

// ────── Communication ──────

icon!(Send, view! {
    <line x1="22" y1="2" x2="11" y2="13"/>
    <polygon points="22 2 15 22 11 13 2 9 22 2"/>
});
icon!(X, view! {
    <path d="M18 6 6 18"/><path d="m6 6 12 12"/>
});
