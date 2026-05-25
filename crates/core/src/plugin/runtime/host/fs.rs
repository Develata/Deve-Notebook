// crates/core/src/plugin/runtime/host/fs.rs
//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
//! # 文件系统宿主函数
//!
//! **功能**: 提供文件读写和项目树获取能力。
//! **安全**: 所有操作需通过 Capability 检查。

use crate::plugin::manifest::Capability;
use rhai::{Engine, EvalAltResult};
use std::path::Path;
use std::sync::Arc;

use super::path_guard::{
    is_ledger_managed_write_target, resolve_capability_read_target, resolve_capability_write_target,
};

/// 注册文件系统 API
pub fn register_fs_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_read = caps.clone();
    let caps_write = caps.clone();

    // API: fs_read(path: &str) -> String
    engine.register_fn(
        "fs_read",
        move |path: &str| -> Result<String, Box<EvalAltResult>> {
            let p = Path::new(path);
            let target = resolve_capability_read_target(caps_read.as_ref(), p)
                .map_err(|e| -> Box<EvalAltResult> { e.into() })?
                .ok_or_else(|| -> Box<EvalAltResult> {
                    format!(
                        "Permission denied: read access to '{}' is not allowed by manifest.",
                        path
                    )
                    .into()
                })?;
            std::fs::read_to_string(target).map_err(|_| "IO Error: Read failed".into())
        },
    );

    // API: fs_write(path: &str, content: &str)
    engine.register_fn(
        "fs_write",
        move |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
            let p = Path::new(path);
            let target = resolve_capability_write_target(caps_write.as_ref(), p)
                .map_err(|e| -> Box<EvalAltResult> { e.into() })?
                .ok_or_else(|| -> Box<EvalAltResult> {
                    format!(
                        "Permission denied: write access to '{}' is not allowed by manifest.",
                        path
                    )
                    .into()
                })?;
            if is_ledger_managed_write_target(&target)
                .map_err(|e| -> Box<EvalAltResult> { e.into() })?
            {
                tracing::warn!("Plugin fs_write blocked on ledger-managed path: {}", path);
                return Err("Permission denied: ledger-managed write denied.".into());
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| "IO Error: Failed to create parent dir")?;
            }
            std::fs::write(target, content).map_err(|_| "IO Error: Write failed".into())
        },
    );

    // API: get_project_tree() -> String (需要 allow_project_tree 权限)
    let caps_tree = caps.clone();
    engine.register_fn(
        "get_project_tree",
        move || -> Result<String, Box<EvalAltResult>> {
            if !caps_tree.check_project_tree() {
                return Err(
                    "Permission denied: project tree access not allowed by manifest.".into(),
                );
            }
            let root = std::env::current_dir().map_err(|e| e.to_string())?;
            let tree = crate::context::DirectoryTree::generate(&root);
            Ok(tree.structure)
        },
    );
}

#[cfg(test)]
mod tests;
