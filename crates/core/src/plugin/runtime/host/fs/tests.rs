use super::*;
use crate::test_support::CWD_LOCK;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

struct CwdGuard {
    old: PathBuf,
}

impl CwdGuard {
    fn enter(path: &std::path::Path) -> Self {
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
fn classifies_ledger_managed_relative_paths() {
    assert!(is_ledger_managed_write_target(Path::new("ledger/local/wiki.redb")).unwrap());
    assert!(!is_ledger_managed_write_target(Path::new("tmp/report.md")).unwrap());
}

#[test]
fn fs_write_denies_project_ledger_path() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::create_dir_all(dir.path().join("ledger/local")).expect("mkdir ledger");

    let mut engine = Engine::new();
    let caps = Arc::new(Capability {
        allow_fs_write: vec![dir.path().join("ledger/local")],
        ..Default::default()
    });
    register_fs_api(&mut engine, caps);
    let script = format!(
        r#"fs_write("{}", "blocked")"#,
        dir.path()
            .join("ledger/local/wiki.redb")
            .to_string_lossy()
            .replace('\\', "\\\\")
    );
    let err = engine
        .eval::<()>(&script)
        .expect_err("project ledger write must fail");

    assert!(err.to_string().contains("ledger-managed write denied"));
}

#[test]
fn fs_write_allows_non_ledger_asset() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let output = dir.path().join("workspace/default/exports/report.txt");
    let mut engine = Engine::new();
    let caps = Arc::new(Capability {
        allow_fs_write: vec![dir.path().join("workspace/default/exports")],
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

    assert_eq!(std::fs::read_to_string(output).expect("read output"), "ok");
}

#[cfg(unix)]
#[test]
fn fs_read_denies_symlink_escape_from_allowed_dir() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let allowed = dir.path().join("allowed");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&allowed).expect("mkdir allowed");
    std::fs::create_dir_all(&outside).expect("mkdir outside");
    let secret = outside.join("secret.txt");
    std::fs::write(&secret, "secret").expect("write secret");
    let alias = allowed.join("secret-link.txt");
    symlink(&secret, &alias).expect("symlink");

    let mut engine = Engine::new();
    let caps = Arc::new(Capability {
        allow_fs_read: vec![allowed],
        ..Default::default()
    });
    register_fs_api(&mut engine, caps);
    let script = format!(
        r#"fs_read("{}")"#,
        alias.to_string_lossy().replace('\\', "\\\\")
    );
    let err = engine
        .eval::<String>(&script)
        .expect_err("symlink read escape must fail");

    assert!(err.to_string().contains("Permission denied"));
}

#[cfg(unix)]
#[test]
fn fs_read_denies_symlink_parent_escape_from_allowed_dir() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let allowed = dir.path().join("allowed");
    let outside_root = dir.path().join("outside");
    let outside_subdir = outside_root.join("subdir");
    std::fs::create_dir_all(&allowed).expect("mkdir allowed");
    std::fs::create_dir_all(&outside_subdir).expect("mkdir outside");
    let secret = outside_root.join("secret.txt");
    std::fs::write(&secret, "secret").expect("write secret");
    let alias = allowed.join("outside-link");
    symlink(&outside_subdir, &alias).expect("symlink");
    let escaped = alias.join("../secret.txt");

    let mut engine = Engine::new();
    let caps = Arc::new(Capability {
        allow_fs_read: vec![allowed],
        ..Default::default()
    });
    register_fs_api(&mut engine, caps);
    let script = format!(
        r#"fs_read("{}")"#,
        escaped.to_string_lossy().replace('\\', "\\\\")
    );
    let err = engine
        .eval::<String>(&script)
        .expect_err("symlink parent read escape must fail");

    assert!(err.to_string().contains("Permission denied"));
}

#[cfg(unix)]
#[test]
fn fs_write_denies_symlink_file_escape_from_allowed_dir() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let allowed = dir.path().join("allowed");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&allowed).expect("mkdir allowed");
    std::fs::create_dir_all(&outside).expect("mkdir outside");
    let target = outside.join("target.txt");
    std::fs::write(&target, "safe").expect("write target");
    let alias = allowed.join("target-link.txt");
    symlink(&target, &alias).expect("symlink");

    let mut engine = Engine::new();
    let caps = Arc::new(Capability {
        allow_fs_write: vec![allowed],
        ..Default::default()
    });
    register_fs_api(&mut engine, caps);
    let script = format!(
        r#"fs_write("{}", "pwned")"#,
        alias.to_string_lossy().replace('\\', "\\\\")
    );
    let err = engine
        .eval::<()>(&script)
        .expect_err("symlink write escape must fail");

    assert!(err.to_string().contains("Permission denied"));
    assert_eq!(
        std::fs::read_to_string(target).expect("read target"),
        "safe"
    );
}

#[cfg(unix)]
#[test]
fn fs_write_denies_dangling_symlink_escape_from_allowed_dir() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let allowed = dir.path().join("allowed");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&allowed).expect("mkdir allowed");
    std::fs::create_dir_all(&outside).expect("mkdir outside");
    let target = outside.join("created-through-link.txt");
    let alias = allowed.join("dangling-link.txt");
    symlink(&target, &alias).expect("symlink");

    let mut engine = Engine::new();
    let caps = Arc::new(Capability {
        allow_fs_write: vec![allowed],
        ..Default::default()
    });
    register_fs_api(&mut engine, caps);
    let script = format!(
        r#"fs_write("{}", "pwned")"#,
        alias.to_string_lossy().replace('\\', "\\\\")
    );
    let err = engine
        .eval::<()>(&script)
        .expect_err("dangling symlink write escape must fail");

    assert!(err.to_string().contains("Failed to canonicalize"));
    assert!(!target.exists());
}

#[cfg(unix)]
#[test]
fn fs_write_denies_symlink_parent_escape_from_allowed_dir() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let allowed = dir.path().join("allowed");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&allowed).expect("mkdir allowed");
    std::fs::create_dir_all(&outside).expect("mkdir outside");
    let alias_dir = allowed.join("outside-link");
    symlink(&outside, &alias_dir).expect("symlink");
    let escaped_target = alias_dir.join("created.txt");

    let mut engine = Engine::new();
    let caps = Arc::new(Capability {
        allow_fs_write: vec![allowed],
        ..Default::default()
    });
    register_fs_api(&mut engine, caps);
    let script = format!(
        r#"fs_write("{}", "pwned")"#,
        escaped_target.to_string_lossy().replace('\\', "\\\\")
    );
    let err = engine
        .eval::<()>(&script)
        .expect_err("symlink parent write escape must fail");

    assert!(err.to_string().contains("Permission denied"));
    assert!(!outside.join("created.txt").exists());
}
