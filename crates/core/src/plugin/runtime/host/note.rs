//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use crate::models::DocId;
use crate::plugin::manifest::Capability;
use crate::state;
use anyhow::Result;
use rhai::{Engine, EvalAltResult};
use std::path::Path;
use std::sync::Arc;

use super::path_guard::{
    is_ledger_managed_write_target, managed_note_target_parts, resolve_capability_read_target,
    resolve_capability_write_target,
};

pub fn register_note_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_read = caps.clone();
    let caps_write = caps;
    engine.register_fn(
        "note_read",
        move |path: &str| -> Result<String, Box<EvalAltResult>> {
            let target = Path::new(path);
            let target = resolve_capability_read_target(caps_read.as_ref(), target)
                .map_err(|e| -> Box<EvalAltResult> { e.into() })?
                .ok_or_else(|| -> Box<EvalAltResult> {
                    format!(
                        "Permission denied: read access to '{}' is not allowed by manifest.",
                        path
                    )
                    .into()
                })?;
            read_managed_note(&target).map_err(|e| e.to_string().into())
        },
    );
    engine.register_fn(
        "note_write",
        move |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
            let target = Path::new(path);
            let target = resolve_capability_write_target(caps_write.as_ref(), target)
                .map_err(|e| -> Box<EvalAltResult> { e.into() })?
                .ok_or_else(|| -> Box<EvalAltResult> {
                    format!(
                        "Permission denied: write access to '{}' is not allowed by manifest.",
                        path
                    )
                    .into()
                })?;
            write_managed_note(&target, content).map_err(super::host_error_to_eval)
        },
    );
}

fn read_managed_note(path: &Path) -> Result<String> {
    let doc_id = resolve_managed_note_target(path)?;
    let repo_name = managed_target_parts(path)?.0;
    let repo = super::repo_manager()?;
    let ops = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    Ok(state::reconstruct_content(&entries))
}

fn write_managed_note(path: &Path, content: &str) -> Result<()> {
    let (repo_name, repo_path) = managed_target_parts(path)?;
    super::managed_note::managed_note_mutation_host()?.write_managed_note(
        super::ManagedNoteWriteIntent {
            repo_name,
            repo_path,
            content: content.to_owned(),
        },
    )
}

fn resolve_managed_note_target(path: &Path) -> Result<DocId> {
    let (repo_name, repo_path) = managed_target_parts(path)?;
    super::repo_manager()?
        .get_tracked_docid_in_local_repo(&repo_name, &repo_path)?
        .ok_or_else(|| anyhow::anyhow!("managed note not found: {}", repo_path))
}

fn managed_target_parts(path: &Path) -> Result<(String, String)> {
    if !is_ledger_managed_write_target(path).map_err(anyhow::Error::msg)? {
        anyhow::bail!("note API only supports ledger-managed markdown targets");
    }
    managed_note_target_parts(path)
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("note API requires a managed markdown path"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::CWD_LOCK;
    use rhai::Engine;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::tempdir;

    struct CwdGuard {
        old: PathBuf,
    }

    impl CwdGuard {
        fn enter(path: &Path) -> Self {
            let old = std::env::current_dir().expect("cwd");
            std::env::set_current_dir(path).expect("set cwd");
            Self { old }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.old);
        }
    }

    #[test]
    fn rejects_non_managed_targets() {
        let err = write_managed_note(Path::new("tmp/out.txt"), "x").expect_err("must fail");
        assert!(err.to_string().contains("host context is unavailable"));
        let err = read_managed_note(Path::new("tmp/out.txt")).expect_err("must fail");
        assert!(err.to_string().contains("host context is unavailable"));
    }

    #[test]
    fn note_api_denies_parent_escape_before_managed_note_resolution() {
        let _guard = CWD_LOCK.lock().expect("lock cwd");
        let dir = tempdir().expect("tempdir");
        let app_root = dir.path().join("app");
        std::fs::create_dir_all(app_root.join("allowed")).expect("mkdir app allowed");
        std::fs::create_dir_all(dir.path().join("allowed")).expect("mkdir outside allowed");
        let _cwd = CwdGuard::enter(&app_root);

        let mut engine = Engine::new();
        let caps = Arc::new(Capability {
            allow_fs_read: vec![PathBuf::from("allowed")],
            allow_fs_write: vec![PathBuf::from("allowed")],
            ..Default::default()
        });
        register_note_api(&mut engine, caps);

        let err = engine
            .eval::<String>(r#"note_read("../allowed/secret.md")"#)
            .expect_err("parent escape read must fail at capability gate");
        assert!(err.to_string().contains("Permission denied"));

        let err = engine
            .eval::<()>(r#"note_write("../allowed/secret.md", "x")"#)
            .expect_err("parent escape write must fail at capability gate");
        assert!(err.to_string().contains("Permission denied"));
    }
}
