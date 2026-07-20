// apps\cli\src\server\handlers
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! 消息处理器模块
//!
//! 包含各类 ClientMessage 的处理逻辑，按功能领域划分。
pub mod admin;
pub mod docs;
pub mod document;
pub mod key_exchange;
pub mod listing;
pub(crate) mod local_writer;
pub mod merge;
pub mod plugin;
pub mod remote_import;
pub mod remote_projection;
pub mod repo;
pub mod repo_control;
pub mod repo_list;
pub mod search;
pub mod source_control;
pub mod switcher;
pub mod sync;
