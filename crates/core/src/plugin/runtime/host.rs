// crates\core\src\plugin\runtime\host.rs
//! # Host Functions (宿主函数)
//!
//! **功能**:
//! 向 Rhai 引擎注册宿主环境提供的能力（如文件 IO、日志）。
//!
//! **安全**:
//! 所有敏感操作必须经过 `Capability` 检查。

use crate::plugin::manifest::PluginManifest;
use crate::plugin::runtime::chat_stream;
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

        // API: get_project_tree
        // Returns a compact directory tree string for LLM context
        engine.register_fn(
            "get_project_tree",
            move || -> Result<String, Box<EvalAltResult>> {
                // Assume current working directory is project root
                let root = std::env::current_dir().map_err(|e| e.to_string())?;
                let tree = crate::context::DirectoryTree::generate(&root);
                Ok(tree.structure)
            },
        );

        let caps_chat = caps.clone();
        engine.register_fn(
            "ai_chat_stream",
            move |req_id: &str,
                  config: rhai::Dynamic,
                  history: rhai::Dynamic|
                  -> Result<rhai::Dynamic, Box<EvalAltResult>> {
                let config_json: serde_json::Value =
                    rhai::serde::from_dynamic(&config).map_err(|e| e.to_string())?;
                let history_json: serde_json::Value =
                    rhai::serde::from_dynamic(&history).map_err(|e| e.to_string())?;

                let base_url = config_json
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing base_url".to_string())?;
                let domain =
                    extract_domain(base_url).ok_or_else(|| "Invalid base_url".to_string())?;
                if !caps_chat.check_net(domain) {
                    return Err(format!(
                        "Permission denied: net access to '{}' is not allowed by manifest.",
                        domain
                    )
                    .into());
                }

                let handler = chat_stream::chat_stream_handler()
                    .ok_or_else(|| "Chat stream handler not configured".to_string())?;
                let sink = chat_stream::current_chat_stream_sink()
                    .ok_or_else(|| "Chat stream sink not configured".to_string())?;

                let request = chat_stream::ChatStreamRequest {
                    req_id: req_id.to_string(),
                    config: config_json,
                    history: history_json,
                    tools: None, // No tools for basic chat
                };
                let response = handler.stream(request, sink).map_err(|e| e.to_string())?;

                // Convert response to Dynamic
                let result_json = serde_json::to_value(&response).map_err(|e| e.to_string())?;
                rhai::serde::to_dynamic(&result_json).map_err(|e| e.to_string().into())
            },
        );

        // API: ai_chat_stream_with_tools - supports function calling
        let caps_chat_tools = caps.clone();
        engine.register_fn(
            "ai_chat_stream_with_tools",
            move |req_id: &str,
                  config: rhai::Dynamic,
                  history: rhai::Dynamic,
                  tools: rhai::Dynamic|
                  -> Result<rhai::Dynamic, Box<EvalAltResult>> {
                let config_json: serde_json::Value =
                    rhai::serde::from_dynamic(&config).map_err(|e| e.to_string())?;
                let history_json: serde_json::Value =
                    rhai::serde::from_dynamic(&history).map_err(|e| e.to_string())?;
                let tools_json: serde_json::Value =
                    rhai::serde::from_dynamic(&tools).map_err(|e| e.to_string())?;

                let base_url = config_json
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing base_url".to_string())?;
                let domain =
                    extract_domain(base_url).ok_or_else(|| "Invalid base_url".to_string())?;
                if !caps_chat_tools.check_net(domain) {
                    return Err(format!(
                        "Permission denied: net access to '{}' is not allowed by manifest.",
                        domain
                    )
                    .into());
                }

                let handler = chat_stream::chat_stream_handler()
                    .ok_or_else(|| "Chat stream handler not configured".to_string())?;
                let sink = chat_stream::current_chat_stream_sink()
                    .ok_or_else(|| "Chat stream sink not configured".to_string())?;

                let request = chat_stream::ChatStreamRequest {
                    req_id: req_id.to_string(),
                    config: config_json,
                    history: history_json,
                    tools: Some(tools_json),
                };
                let response = handler.stream(request, sink).map_err(|e| e.to_string())?;

                // Convert response to Dynamic
                let result_json = serde_json::to_value(&response).map_err(|e| e.to_string())?;
                rhai::serde::to_dynamic(&result_json).map_err(|e| e.to_string().into())
            },
        );
    }

    // API: log_info(msg: &str)
    engine.register_fn("log_info", |msg: &str| {
        println!("[Plugin Log] {}", msg);
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_domain(url: &str) -> Option<&str> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let host = without_scheme.split('/').next()?;
    host.split(':').next()
}
