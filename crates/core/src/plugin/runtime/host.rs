// crates\core\src\plugin\runtime\host.rs
//! # Host Functions (宿主函数)
//!
//! **功能**:
//! 向 Rhai 引擎注册宿主环境提供的能力（如文件 IO、日志）。
//!
//! **安全**:
//! 所有敏感操作必须经过 `Capability` 检查。

use crate::plugin::manifest::PluginManifest;
use rhai::{Engine, EvalAltResult};
use std::sync::Arc;

/// 注册核心 API 到 Rhai 引擎
pub fn register_core_api(engine: &mut Engine, manifest: &PluginManifest) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 捕获 Capability 的 Arc 副本以用于闭包
        let caps = Arc::new(manifest.capabilities.clone());
        let caps_read = caps.clone();

        // API: fs_read(path: &str) -> String
        engine.register_fn(
            "fs_read",
            move |path: &str| -> Result<String, Box<EvalAltResult>> {
                use std::path::Path;
                let p = Path::new(path);
                if !caps_read.check_read(p) {
                    return Err(format!(
                        "Permission denied: read access to '{}' is not allowed by manifest.",
                        path
                    )
                    .into());
                }
                std::fs::read_to_string(p).map_err(|_| "IO Error: Read failed".into())
            },
        );

        let caps_write = caps.clone();
        // API: fs_write(path: &str, content: &str)
        engine.register_fn(
            "fs_write",
            move |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
                use std::path::Path;
                let p = Path::new(path);
                if !caps_write.check_write(p) {
                    return Err(format!(
                        "Permission denied: write access to '{}' is not allowed by manifest.",
                        path
                    )
                    .into());
                }
                std::fs::write(p, content).map_err(|_| "IO Error: Write failed".into())
            },
        );
    }

    // API: log_info(msg: &str)
    engine.register_fn("log_info", |msg: &str| {
        println!("[Plugin Log] {}", msg);
        // TODO: Integrate with tracing::info!
    });
}
