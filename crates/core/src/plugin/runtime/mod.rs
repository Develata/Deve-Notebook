// crates\core\src\plugin\runtime\mod.rs
//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
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
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

pub mod chat_stream;
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
mod module_resolver;
pub mod provider;
pub mod rhai_v1;
pub mod tools;

pub use rhai_v1::RhaiRuntime;

/// 插件运行时抽象接口
///
/// 允许未来扩展其他脚本引擎 (e.g., Lua, Wasm)。
pub trait PluginRuntime: Send + Sync {
    /// Parse/compile a plugin without executing its top-level initializer.
    ///
    /// **参数**:
    /// - `manifest`: 插件清单
    /// - `script`: 源代码
    fn prepare(&mut self, manifest: PluginManifest, script: &str) -> Result<()>;

    /// Execute the prepared top-level initializer after host authority is installed.
    fn activate(&self) -> Result<()>;

    /// Bind the host-owned mutation authority for this runtime generation.
    #[cfg(not(target_arch = "wasm32"))]
    fn install_host_context(&self, _context: Arc<host::PluginHostContext>) -> Result<()> {
        anyhow::bail!("Plugin runtime does not support a managed host context")
    }

    /// Compatibility helper for isolated runtimes that deliberately own both phases.
    fn load(&mut self, manifest: PluginManifest, script: &str) -> Result<()> {
        self.prepare(manifest, script)?;
        self.activate()
    }

    /// 调用函数
    ///
    /// **参数**:
    /// - `fn_name`: 函数名
    /// - `args`: 参数列表
    fn call(&self, fn_name: &str, args: Vec<Dynamic>) -> Result<Dynamic>;

    /// 获取清单
    fn manifest(&self) -> &PluginManifest;
}
