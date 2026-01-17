// crates\core\src\plugin
//! # Plugin System (插件系统)
//!
//! **架构作用**:
//! 定义插件系统的核心数据结构与运行时接口。
//!
//! **核心功能清单**:
//! - `manifest`: 定义插件清单与能力列表。
//!
//! **类型**: Core MUST (核心必选)

pub mod manifest;
pub mod runtime;
pub mod loader;
