//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! UI 辅助图标 (状态、文件操作、布局)
#![allow(dead_code)]

use leptos::prelude::*;

// ────── Status ──────

icon!(Zap, view! { <path d="M13 2 3 14h9l-1 10 10-12h-9l1-10z"/> });
icon!(
    AlertTriangle,
    view! { <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/> }
);

icon!(
    Lock,
    view! {
        <rect width="18" height="11" x="3" y="11" rx="2" ry="2"/>
        <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
    }
);

icon!(
    Pin,
    view! {
        <line x1="12" x2="12" y1="17" y2="22"/>
        <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"/>
    }
);

// ────── File Operations ──────

icon!(
    Folder,
    view! {
        <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>
    }
);

icon!(
    File,
    view! {
        <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/>
        <path d="M14 2v4a2 2 0 0 0 2 2h4"/>
    }
);

icon!(
    Pencil,
    view! {
        <path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/>
        <path d="m15 5 4 4"/>
    }
);

icon!(
    Copy,
    view! {
        <rect width="14" height="14" x="8" y="8" rx="2" ry="2"/>
        <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>
    }
);

icon!(
    FolderInput,
    view! {
        <path d="M2 9V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-1"/>
        <path d="M2 13h10"/><path d="m9 16 3-3-3-3"/>
    }
);

icon!(
    Trash2,
    view! {
        <path d="M10 11v6"/><path d="M14 11v6"/>
        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/>
        <path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
    }
);

// ────── Layout ──────

icon!(
    ListTree,
    view! {
        <path d="M21 12h-8"/><path d="M21 6H8"/><path d="M21 18h-8"/>
        <path d="M3 6v4c0 1.1.9 2 2 2h3"/>
        <path d="M3 10v6c0 1.1.9 2 2 2h3"/>
    }
);

icon!(
    SourceControl,
    view! {
        <circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/>
        <path d="M6 21V9a9 9 0 0 0 9 9"/>
    }
);

icon!(
    LayoutGrid,
    view! {
        <rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/>
        <rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/>
    }
);

icon!(
    PanelLeft,
    view! {
        <rect width="18" height="18" x="3" y="3" rx="2"/>
        <path d="M9 3v18"/>
    }
);
