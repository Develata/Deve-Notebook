// crates/core/src/plugin/runtime/host/fs.rs
//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
//! # 文件系统宿主函数
//!
//! **功能**: 提供文件读写和项目树获取能力。
//! **安全**: 所有操作需通过 Capability 检查。

use crate::plugin::manifest::Capability;
use rhai::{Engine, EvalAltResult};
use std::path::Path;
use std::sync::Arc;

use super::path_guard::is_ledger_managed_write_target;

/// 注册文件系统 API
pub fn register_fs_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_read = caps.clone();
    let caps_write = caps.clone();

    // API: fs_read(path: &str) -> String
    engine.register_fn(
        "fs_read",
        move |path: &str| -> Result<String, Box<EvalAltResult>> {
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
            let p = Path::new(path);
            if !caps_write.check_write(p) {
                return Err(format!(
                    "Permission denied: write access to '{}' is not allowed by manifest.",
                    path
                )
                .into());
            }
            if is_ledger_managed_write_target(p).map_err(|e| -> Box<EvalAltResult> { e.into() })? {
                tracing::warn!("Plugin fs_write blocked on ledger-managed path: {}", path);
                return Err("Permission denied: ledger-managed write denied.".into());
            }
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| "IO Error: Failed to create parent dir")?;
            }
            std::fs::write(p, content).map_err(|_| "IO Error: Write failed".into())
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
mod tests {
    use super::*;
    use crate::plugin::runtime::host::path_guard::split_managed_note_target;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn classifies_ledger_managed_relative_paths() {
        assert_eq!(
            split_managed_note_target("vault/default/notes/a.md"),
            Some(("default".into(), "notes/a.md".into()))
        );
        assert!(is_ledger_managed_write_target(Path::new("ledger/local/wiki.redb")).unwrap());
        assert!(!is_ledger_managed_write_target(Path::new("tmp/report.md")).unwrap());
    }

    #[test]
    fn fs_write_denies_managed_markdown() {
        let _guard = CWD_LOCK.lock().expect("lock cwd");
        let dir = tempdir().expect("tempdir");
        let old_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");

        let mut engine = Engine::new();
        let caps = Arc::new(Capability {
            allow_fs_write: vec![dir.path().join("vault/default")],
            ..Default::default()
        });
        register_fs_api(&mut engine, caps);
        let script = format!(
            r#"fs_write("{}", "blocked")"#,
            dir.path()
                .join("vault/default/notes/a.md")
                .to_string_lossy()
                .replace('\\', "\\\\")
        );
        let err = engine
            .eval::<()>(&script)
            .expect_err("managed markdown must fail");

        std::env::set_current_dir(old_cwd).expect("restore cwd");
        assert!(err.to_string().contains("ledger-managed write denied"));
    }

    #[test]
    fn fs_write_allows_non_ledger_asset() {
        let _guard = CWD_LOCK.lock().expect("lock cwd");
        let dir = tempdir().expect("tempdir");
        let old_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");

        let output = dir.path().join("vault/default/exports/report.txt");
        let mut engine = Engine::new();
        let caps = Arc::new(Capability {
            allow_fs_write: vec![dir.path().join("vault/default/exports")],
            ..Default::default()
        });
        register_fs_api(&mut engine, caps);
        let script = format!(
            r#"fs_write("{}", "ok")"#,
            output.to_string_lossy().replace('\\', "\\\\")
        );
        engine
            .eval::<()>(&script)
            .expect("export write should work");

        std::env::set_current_dir(old_cwd).expect("restore cwd");
        assert_eq!(std::fs::read_to_string(output).expect("read output"), "ok");
    }
}
