use super::*;
use tempfile::TempDir;

fn new_repo() -> (TempDir, RepoManager, String) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, repo_id) = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
    )
    .expect("init cataloged repo");
    let name = repo_id.to_string();
    // These tests assert that workspace path *lookups* never materialize the
    // workspace root. Catalog membership and the projection locator are durable
    // and independent of the workspace directory, so clear the root that the
    // creation choreography pre-created and start from an absent root.
    let root = repo
        .local_repo_workspace_root(&name)
        .expect("resolve workspace root");
    std::fs::remove_dir_all(&root).expect("clear pre-created workspace root");
    (dir, repo, name)
}

#[test]
fn projection_workspace_child_path_accepts_canonical_relative_path_and_root() {
    let (_dir, repo, name) = new_repo();
    let root = repo
        .local_repo_workspace_path(&name, "")
        .expect("root lookup");
    assert!(!root.exists(), "path lookup must not create workspace root");

    assert_eq!(
        repo.local_repo_workspace_path(&name, "notes/a.md")
            .expect("safe child path"),
        root.join("notes").join("a.md")
    );
    assert!(
        !root.exists(),
        "safe lexical join must not create workspace root"
    );
}

#[test]
fn projection_workspace_child_path_rejects_unsafe_or_noncanonical_inputs() {
    let (_dir, repo, name) = new_repo();
    for path in [
        ".",
        "..",
        "../outside.md",
        "./a.md",
        "notes/../a.md",
        ".. /outside.md",
        "a//b.md",
        "/leading.md",
        "trailing.md/",
        "C:/outside.md",
        "notes/C:/outside.md",
        r"C:\outside.md",
        "//server/share/a.md",
        r"\\server\share\a.md",
        "nul\0path.md",
        ".git/config",
        ".git./config",
        ".git /config",
        "notes/.GiT/config",
        ".notegit/state",
        "notes/.NOTEGIT/state",
        "notes./a.md",
        "notes /a.md",
        r"notes\a.md",
    ] {
        let err = repo
            .local_repo_workspace_path(&name, path)
            .expect_err("unsafe child path must fail closed");
        assert!(
            err.to_string()
                .contains("Invalid Projection Workspace child path"),
            "unexpected diagnostic for {path:?}: {err:#}"
        );
    }
}

#[test]
fn projection_workspace_root_symlink_cannot_escape_projection_base() {
    let (dir, repo, name) = new_repo();
    let root = repo
        .local_repo_workspace_path(&name, "")
        .expect("resolve absent workspace root");
    let outside = dir.path().join("outside-root");
    std::fs::create_dir_all(&outside).expect("create outside root");
    if !create_dir_symlink(&outside, &root) {
        return;
    }

    let root_err = repo
        .local_repo_workspace_path(&name, "")
        .expect_err("linked workspace root lookup must fail closed");
    assert!(
        root_err
            .to_string()
            .contains("must not be a symlink or junction")
            || root_err
                .to_string()
                .contains("escapes canonical projection base"),
        "unexpected root diagnostic: {root_err:#}"
    );
    repo.local_repo_workspace_path(&name, "notes/a.md")
        .expect_err("child below linked workspace root must fail closed");
}

#[test]
fn projection_workspace_existing_ancestor_symlink_cannot_escape_root() {
    let (dir, repo, name) = new_repo();
    let root = repo
        .ensure_local_repo_workspace_identity(&name)
        .expect("create workspace root");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&outside).expect("create outside dir");
    let link = root.join("escape");
    if !create_dir_symlink(&outside, &link) {
        return;
    }

    let err = repo
        .local_repo_workspace_path(&name, "escape/a.md")
        .expect_err("external symlink must fail closed");
    assert!(
        err.to_string().contains("escapes canonical root"),
        "unexpected diagnostic: {err:#}"
    );
}

#[test]
fn projection_workspace_existing_ancestor_dangling_symlink_fails_closed() {
    let (dir, repo, name) = new_repo();
    let root = repo
        .ensure_local_repo_workspace_identity(&name)
        .expect("create workspace root");
    let missing = dir.path().join("missing");
    let link = root.join("dangling");
    if !create_dir_symlink(&missing, &link) {
        return;
    }

    let err = repo
        .local_repo_workspace_path(&name, "dangling/a.md")
        .expect_err("dangling symlink must fail closed");
    assert!(
        err.to_string()
            .contains("Failed to canonicalize existing Projection Workspace ancestor"),
        "unexpected diagnostic: {err:#}"
    );
}

#[cfg(unix)]
fn create_dir_symlink(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).expect("create directory symlink");
    true
}

#[cfg(windows)]
fn create_dir_symlink(target: &Path, link: &Path) -> bool {
    match std::os::windows::fs::symlink_dir(target, link) {
        Ok(()) => true,
        Err(err) if err.kind() == ErrorKind::PermissionDenied => {
            eprintln!("skipping symlink branch: insufficient Windows symlink privilege: {err}");
            false
        }
        Err(err) => panic!("create directory symlink: {err}"),
    }
}
