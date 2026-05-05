// apps\web\src\components
//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!
//! # UI 组件模块 (UI Components Module)
//!
//! 包含 Web 应用程序的所有 Leptos UI 组件。
//! 结构遵循 "Activity Bar + Resizable Slot" 布局。
pub mod bottom_bar;
pub mod command_palette;
pub mod dropdown;
pub mod header;
pub mod icons;
pub mod layout_context;
pub mod outline;
pub mod outline_render;
pub mod playback;
pub mod settings;
pub mod sidebar;
pub mod sidebar_menu;
pub mod touch_feedback;

pub mod activity_bar;
pub mod ai_backend_guard;
pub mod branch_label;
pub mod branch_switcher;
pub mod chat; // [NEW] AI Chat
pub mod disconnect_overlay;
pub(crate) mod focus_scope;
pub mod login;
pub mod main_layout;
pub mod merge_modal;
pub mod merge_modal_slot;
pub mod merge_panel;
pub mod pending_navigation_modal;
pub mod quick_open;
pub mod search_box;
pub mod spectator_overlay;

pub mod dashboard;
pub mod desktop_chat_panel;
pub mod desktop_layout;
pub mod diff_view;
pub mod main_layout_runtime;
pub mod mobile_layout;
pub mod settings_sections;
mod settings_sections_policy;
