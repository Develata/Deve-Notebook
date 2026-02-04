// apps\web\src\components
//! # UI 组件模块 (UI Components Module)
//!
//! 包含 Web 应用程序的所有 Leptos UI 组件。
//! 结构遵循 "Activity Bar + Resizable Slot" 布局。
pub mod bottom_bar;
pub mod command_palette;
pub mod dropdown;
pub mod header;
pub mod layout_context;
pub mod outline;
pub mod outline_render;
pub mod playback;
pub mod settings;
pub mod sidebar;
pub mod sidebar_menu;

pub mod activity_bar;
pub mod branch_switcher;
pub mod chat; // [NEW] AI Chat
pub mod disconnect_overlay;
pub mod main_layout;
pub mod merge_modal;
pub mod merge_modal_slot;
pub mod merge_panel;
pub mod quick_open;
pub mod search_box;
pub mod spectator_overlay;

pub mod diff_view;
