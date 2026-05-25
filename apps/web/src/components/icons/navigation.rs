//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! 导航与搜索相关图标
#![allow(dead_code)]

use leptos::prelude::*;

// ────── Navigation ──────

icon!(
    Home,
    view! {
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
        <polyline points="9 22 9 12 15 12 15 22"/>
    }
);

icon!(
    Terminal,
    view! {
        <polyline points="4 17 10 11 4 5"/>
        <line x1="12" x2="20" y1="19" y2="19"/>
    }
);

icon!(ChevronUp, view! { <path d="m18 15-6-6-6 6"/> });

icon!(
    ArrowRight,
    view! {
        <path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>
    }
);

// ────── Search ──────

icon!(
    Search,
    view! {
        <circle cx="11" cy="11" r="8"/>
        <line x1="21" x2="16.65" y1="21" y2="16.65"/>
    }
);
