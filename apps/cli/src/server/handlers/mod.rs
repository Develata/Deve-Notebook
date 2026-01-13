//! 消息处理器模块
//!
//! 包含各类 ClientMessage 的处理逻辑，按功能领域划分。
pub mod document;
pub mod system;
pub mod plugin;
pub mod search;
pub mod sync;
pub mod merge;
pub mod source_control;
