// crates/core/src/tree/mod.rs
//! # 文件树模块 (File Tree Module)
//!
//! 本模块提供文件树的核心数据结构和增量更新逻辑。
//! 被后端（维护状态）和前端（应用 Delta）共同使用。
//!
//! ## 模块结构
//!
//! - `node`: 文件节点定义
//! - `delta`: 增量更新类型
//! - `manager`: 树状态管理器 (仅后端)

pub mod delta;
pub mod node;

#[cfg(not(target_arch = "wasm32"))]
mod ops;

#[cfg(not(target_arch = "wasm32"))]
pub mod manager;

// 重导出常用类型
pub use delta::TreeDelta;
pub use node::FileNode;

#[cfg(not(target_arch = "wasm32"))]
pub use manager::TreeManager;
