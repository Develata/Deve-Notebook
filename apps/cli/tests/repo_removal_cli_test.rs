//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 14_commands#repo-removal-command-contract
//!
//! Separate-process proof for preview/apply identity handoff and preservation.

use std::path::Path;
use std::process::{Command, Output};

fn deve(root: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_deve"));
    command.current_dir(root).args(args);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("DEVE_") {
            command.env_remove(key);
        }
    }
    command.output().expect("run deve")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn field(output: &str, key: &str) -> String {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing {key} in output:\n{output}"))
        .to_owned()
}

#[test]
fn removal_preview_and_apply_cross_process_preserve_workspace_and_git() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let projection = root.join("projection");
    let repo_id = uuid::Uuid::new_v4();
    let repo_id_text = repo_id.to_string();
    let root_text = root.to_str().expect("root UTF-8");
    let projection_text = projection.to_str().expect("projection UTF-8");
    let init = deve(
        root,
        &[
            "init",
            "--path",
            root_text,
            "--repo",
            "local",
            "--projection-base",
            projection_text,
            "--repo-id",
            &repo_id_text,
        ],
    );
    assert!(init.status.success(), "init failed: {}", text(&init.stderr));

    let workspace = projection.join(&repo_id_text);
    std::fs::write(workspace.join("kept.md"), b"# keep").expect("write markdown");
    std::fs::create_dir_all(workspace.join(".git")).expect("create git");
    std::fs::write(workspace.join(".git/config"), b"[core]\n").expect("write git config");
    std::fs::write(workspace.join("unknown.bin"), b"keep").expect("write unknown");

    let preview = deve(root, &["repo", "remove", "--repo-id", &repo_id_text]);
    assert!(
        preview.status.success(),
        "preview failed: {}",
        text(&preview.stderr)
    );
    let preview_stdout = text(&preview.stdout);
    assert!(preview_stdout.contains("repo_removal=preview"));
    assert!(preview_stdout.contains("blockers="));
    let token = field(&preview_stdout, "confirmation_token");
    assert_ne!(token, "unavailable");

    let apply = deve(
        root,
        &[
            "repo",
            "remove",
            "--repo-id",
            &repo_id_text,
            "--apply",
            "--token",
            &token,
        ],
    );
    assert!(
        apply.status.success(),
        "apply failed: {}",
        text(&apply.stderr)
    );
    let apply_stdout = text(&apply.stdout);
    assert!(apply_stdout.contains("repo_removal=accepted"));
    assert!(apply_stdout.contains("repo_removal=terminal"));
    assert!(apply_stdout.contains("outcome=succeeded"));
    let execute_request_id = field(&apply_stdout, "request_id");

    assert!(workspace.join("kept.md").is_file());
    assert_eq!(
        std::fs::read(workspace.join("unknown.bin")).expect("unknown file"),
        b"keep"
    );
    assert!(workspace.join(".git/config").is_file());
    assert!(workspace.join(".gitignore").is_file());
    assert!(!workspace.join(".notegit").exists());
    assert!(
        !root
            .join("ledger/local")
            .join(format!("{repo_id}.redb"))
            .exists()
    );

    std::fs::write(
        root.join("ledger/.host/main_port"),
        format!(
            r#"{{"format":"deve.local-cli-owner","version":1,"main_port":65534,"host_peer_id":"stale-owner","runtime_incarnation":"{}"}}"#,
            uuid::Uuid::new_v4()
        ),
    )
    .expect("write stale owner hint");
    let repair_status = deve(
        root,
        &[
            "repo",
            "removal-repair",
            "--request-id",
            &execute_request_id,
        ],
    );
    assert!(
        repair_status.status.success(),
        "terminal repair replay failed (code {:?}):\nstdout={}\nstderr={}",
        repair_status.status.code(),
        text(&repair_status.stdout),
        text(&repair_status.stderr),
    );
    let repair_stdout = text(&repair_status.stdout);
    assert!(repair_stdout.contains("repo_removal=terminal"));
    assert!(repair_stdout.contains("outcome=succeeded"));
    assert!(
        !text(&repair_status.stderr).contains(root_text),
        "repo removal stderr leaked a host path"
    );

    let host = deve_core::ledger::RepoManager::init_empty_host(root.join("ledger"), 100)
        .expect("open empty host");
    assert!(
        host.list_cataloged_local_repo_summaries()
            .expect("catalog summaries")
            .is_empty()
    );
}
