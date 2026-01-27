// crates\core\src\plugin\runtime\mod.rs
//! # Plugin Runtime (插件运行时)
//!
//! **功能**:
//! 定义插件运行时的抽象接口与模块导出。
//!
//! **模块结构**:
//! - `mod`: 接口定义。
//! - `rhai_v1`: Rhai 引擎实现。
//! - `host`: 宿主函数注入。

use crate::plugin::manifest::PluginManifest;
use anyhow::Result;
use rhai::Dynamic;

pub mod chat_stream;
pub mod host;
pub mod rhai_v1;
pub mod tools;

pub use rhai_v1::RhaiRuntime;

/// 插件运行时抽象接口
///
/// 允许未来扩展其他脚本引擎 (e.g., Lua, Wasm)。
pub trait PluginRuntime: Send + Sync {
    /// 加载插件
    ///
    /// **参数**:
    /// - `manifest`: 插件清单
    /// - `script`: 源代码
    fn load(&mut self, manifest: PluginManifest, script: &str) -> Result<()>;

    /// 调用函数
    ///
    /// **参数**:
    /// - `fn_name`: 函数名
    /// - `args`: 参数列表
    fn call(&self, fn_name: &str, args: Vec<Dynamic>) -> Result<Dynamic>;

    /// 获取清单
    fn manifest(&self) -> &PluginManifest;
}
