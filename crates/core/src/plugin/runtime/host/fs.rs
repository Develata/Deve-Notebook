// crates/core/src/plugin/runtime/host/fs.rs
//! # 文件系统宿主函数
//!
//! **功能**: 提供文件读写和项目树获取能力。
//! **安全**: 所有操作需通过 Capability 检查。

use crate::plugin::manifest::Capability;
use crate::utils::path::path_to_forward_slash;
use rhai::{Engine, EvalAltResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

fn is_ledger_managed_write_target(path: &Path) -> Result<bool, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(project_relative_path(&cwd, path).is_some_and(|rel| is_ledger_managed_relative_path(&rel)))
}

fn project_relative_path(cwd: &Path, path: &Path) -> Option<String> {
    let cwd = normalize_host_path(cwd);
    let path = normalize_host_path(&resolve_host_path(cwd.as_path(), path));
    path.strip_prefix(&cwd).ok().map(path_to_forward_slash)
}

fn resolve_host_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn normalize_host_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(..) | std::path::Component::RootDir => {
                normalized.push(component.as_os_str())
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(segment) => normalized.push(segment),
        }
    }
    normalized
}

fn is_ledger_managed_relative_path(rel_path: &str) -> bool {
    let parts: Vec<_> = rel_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.first() == Some(&"ledger") {
        return true;
    }
    if parts.len() >= 3 && parts[0] == "vault" {
        if parts.iter().skip(2).any(|part| *part == ".notegit") {
            return true;
        }
        return rel_path.ends_with(".md");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn classifies_ledger_managed_relative_paths() {
        assert!(is_ledger_managed_relative_path("vault/default/notes/a.md"));
        assert!(is_ledger_managed_relative_path(
            "vault/default/.notegit/pending/db"
        ));
        assert!(is_ledger_managed_relative_path("ledger/local/wiki.redb"));
        assert!(!is_ledger_managed_relative_path(
            "vault/default/exports/report.txt"
        ));
        assert!(!is_ledger_managed_relative_path("tmp/report.md"));
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
