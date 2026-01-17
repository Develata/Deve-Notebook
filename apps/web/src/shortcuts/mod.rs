// apps\web\src\shortcuts
//! # 快捷键模块 (Shortcuts Module)
//!
//! 统一管理全局键盘快捷键。
//!
//! ## 模块结构
//!
//! - `types`: 类型定义 (KeyCombo, Shortcut 等)
//! - `registry`: 快捷键注册表 (注册、查询、冲突检测)
//! - `config`: 用户自定义配置 (localStorage 持久化)
//! - `global`: 全局快捷键定义和处理

pub mod types;
pub mod registry;
pub mod config;
pub mod global;

// 重新导出常用类型
pub use global::create_global_shortcut_handler;
