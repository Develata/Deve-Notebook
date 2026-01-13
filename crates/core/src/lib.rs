//! # Deve-Note 核心库
//!
//! 本 crate 包含 Deve-Note 的核心业务逻辑，是一个实时协作文档编辑器。
//! 它是平台无关的，可被 CLI 后端和 WebAssembly 前端共同使用。
//!
//! ## 模块说明
//!
//! - `models`: 核心数据类型（DocId, Op, LedgerEntry）
//! - `protocol`: WebSocket 消息定义（ClientMessage, ServerMessage）
//! - `state`: 文档状态管理（内容重建、差异计算）
//! - `error`: 统一错误处理
//! - `utils`: 跨平台工具函数
//!
//! ### 仅后端模块（wasm32 上不可用）：
//! - `ledger`: 使用 redb 的持久化文档存储
//! - `vfs`: 虚拟文件系统操作
//! - `watcher`: 文件系统变更检测
//! - `sync`: 文档同步与调和

#[cfg(not(target_arch = "wasm32"))]
pub mod ledger;
pub mod models;
#[cfg(not(target_arch = "wasm32"))]
pub mod vfs;
#[cfg(not(target_arch = "wasm32"))]
pub mod watcher;
pub mod sync;
pub mod state;
pub mod protocol;
pub mod utils;
pub mod config;
pub mod plugin;
pub mod error;
pub mod source_control;

#[cfg(all(not(target_arch = "wasm32"), feature = "search"))]
pub mod search;
