use super::{GitImportPlan, plan_import, validate_import_path};
use crate::source_control::ChangeStatus;
use std::path::Path;
use std::process::Command;

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git(path, &["init"]);
    git(path, &["config", "user.email", "deve@example.invalid"]);
    git(path, &["config", "user.name", "Deve Test"]);
    crate::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

fn commit_baseline(path: &Path) {
    std::fs::write(path.join("delete.md"), "delete\n").expect("delete seed");
    std::fs::write(path.join("mod.md"), "old\n").expect("mod seed");
    std::fs::write(path.join("rename_from.md"), "rename\n").expect("rename seed");
    git(path, &["add", "."]);
    git(path, &["commit", "--no-gpg-sign", "-m", "baseline"]);
}

fn entry<'a>(plan: &'a GitImportPlan, path: &str) -> &'a super::GitImportPlanEntry {
    plan.entries
        .iter()
        .find(|entry| entry.path == path)
        .unwrap_or_else(|| panic!("missing entry {path}: {:?}", plan.entries))
}

#[test]
fn plan_import_reports_tracked_and_untracked_changes() {
    let dir = tempfile::tempdir().expect("tempdir");
    init_git_repo(dir.path());
    commit_baseline(dir.path());

    std::fs::write(dir.path().join("mod.md"), "new\n").expect("modify");
    std::fs::remove_file(dir.path().join("delete.md")).expect("delete");
    git(dir.path(), &["mv", "rename_from.md", "rename_to.md"]);
    std::fs::write(dir.path().join("new.md"), "new file\n").expect("new");

    let plan = plan_import(dir.path()).expect("plan import");

    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    assert_eq!(plan.entries.len(), 4);
    assert_eq!(entry(&plan, "delete.md").status, ChangeStatus::Deleted);
    assert_eq!(entry(&plan, "mod.md").status, ChangeStatus::Modified);
    assert_eq!(entry(&plan, "new.md").status, ChangeStatus::Added);
    let renamed = entry(&plan, "rename_to.md");
    assert_eq!(renamed.status, ChangeStatus::Renamed);
    assert_eq!(renamed.previous_path.as_deref(), Some("rename_from.md"));
    assert!(renamed.git_status.starts_with('R'));
}

#[test]
fn plan_import_requires_git_head() {
    let dir = tempfile::tempdir().expect("tempdir");
    init_git_repo(dir.path());

    let err = plan_import(dir.path()).expect_err("missing HEAD must fail closed");

    assert!(err.to_string().contains("requires Git HEAD"));
}

#[test]
fn validate_import_path_rejects_internal_and_escape_paths() {
    assert_eq!(
        validate_import_path("notes/file.md").expect("valid"),
        "notes/file.md"
    );
    for path in [
        "",
        "/abs.md",
        "C:/abs.md",
        "notes//file.md",
        "notes/../file.md",
        ".git/config",
        "notes/.notegit/state",
    ] {
        let err = validate_import_path(path).expect_err("unsafe path");
        assert!(err.to_string().contains("unsafe path"), "{err}");
    }
}
