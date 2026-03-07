// apps\cli\src\server\handlers
//! 消息处理器模块
//!
//! 包含各类 ClientMessage 的处理逻辑，按功能领域划分。
pub mod docs;
pub mod document;
pub mod key_exchange;
pub mod listing;
pub mod merge;
pub mod plugin;
pub mod repo;
pub mod search;
pub mod source_control;
pub mod switcher;
pub mod sync;
