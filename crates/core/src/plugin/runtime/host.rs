// crates\core\src\plugin\runtime\host.rs
//! # Host Functions (宿主函数)
//!
//! **功能**:
//! 向 Rhai 引擎注册宿主环境提供的能力（如文件 IO、日志）。
//!
//! **安全**:
//! 所有敏感操作必须经过 `Capability` 检查。

use crate::plugin::manifest::PluginManifest;
use rhai::Engine;
#[cfg(not(target_arch = "wasm32"))]
use rhai::EvalAltResult;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

/// 注册核心 API 到 Rhai 引擎
#[allow(unused_variables)] // manifest is used only in non-WASM builds
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
                // Ensure parent dir exists
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|_| "IO Error: Failed to create parent dir")?;
                }
                std::fs::write(p, content).map_err(|_| "IO Error: Write failed".into())
            },
        );

        // API: to_json (Helper)
        engine.register_fn(
            "to_json",
            |val: rhai::Dynamic| -> Result<String, Box<EvalAltResult>> {
                serde_json::to_string(&val).map_err(|e| e.to_string().into())
            },
        );

        // API: parse_json (Helper)
        engine.register_fn(
            "parse_json",
            |json: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
                serde_json::from_str(json).map_err(|e| e.to_string().into())
            },
        );

        // API: env (Helper for capabilities)
        let caps_env = caps.clone();
        engine.register_fn(
            "env",
            move |key: &str| -> Result<rhai::Dynamic, Box<EvalAltResult>> {
                if !caps_env.check_env(key) {
                    return Ok(rhai::Dynamic::UNIT); // Not allowed = None
                }
                match std::env::var(key) {
                    Ok(v) => Ok(v.into()),
                    Err(_) => Ok(rhai::Dynamic::UNIT),
                }
            },
        );
    }

    // API: log_info(msg: &str)
    engine.register_fn("log_info", |msg: &str| {
        println!("[Plugin Log] {}", msg);
    });
}
