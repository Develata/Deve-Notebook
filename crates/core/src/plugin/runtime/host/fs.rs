// crates/core/src/plugin/runtime/host/fs.rs
//! # 文件系统宿主函数
//!
//! **功能**: 提供文件读写和项目树获取能力。
//! **安全**: 所有操作需通过 Capability 检查。

use crate::plugin::manifest::Capability;
use rhai::{Engine, EvalAltResult};
use std::sync::Arc;

/// 注册文件系统 API
pub fn register_fs_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_read = caps.clone();
    let caps_write = caps.clone();

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
            // 确保父目录存在
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| "IO Error: Failed to create parent dir")?;
            }
            std::fs::write(p, content).map_err(|_| "IO Error: Write failed".into())
        },
    );

    // API: get_project_tree() -> String
    engine.register_fn(
        "get_project_tree",
        move || -> Result<String, Box<EvalAltResult>> {
            let root = std::env::current_dir().map_err(|e| e.to_string())?;
            let tree = crate::context::DirectoryTree::generate(&root);
            Ok(tree.structure)
        },
    );
}
