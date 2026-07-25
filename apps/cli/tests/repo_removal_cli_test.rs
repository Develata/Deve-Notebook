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
        if is_deve_env_key(&key.to_string_lossy()) {
            command.env_remove(key);
        }
    }
    command.output().expect("run deve")
}

fn is_deve_env_key(key: &str) -> bool {
    key.as_bytes()
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"DEVE_"))
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

fn init_local_repo(root: &Path, projection: &Path, repo_name: &str, repo_id: uuid::Uuid) {
    let root_text = root.to_str().expect("root UTF-8");
    let projection_text = projection.to_str().expect("projection UTF-8");
    let repo_id_text = repo_id.to_string();
    let init = deve(
        root,
        &[
            "init",
            "--path",
            root_text,
            "--repo",
            repo_name,
            "--projection-base",
            projection_text,
            "--repo-id",
            &repo_id_text,
        ],
    );
    assert!(
        init.status.success(),
        "init {repo_name} failed:\nstdout={}\nstderr={}",
        text(&init.stdout),
        text(&init.stderr)
    );
}

fn preview_and_apply_removal(root: &Path, repo_id: uuid::Uuid) -> String {
    let repo_id_text = repo_id.to_string();
    let preview = deve(root, &["repo", "remove", "--repo-id", &repo_id_text]);
    assert!(
        preview.status.success(),
        "preview failed:\nstdout={}\nstderr={}",
        text(&preview.stdout),
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
        "apply failed:\nstdout={}\nstderr={}",
        text(&apply.stdout),
        text(&apply.stderr)
    );
    let apply_stdout = text(&apply.stdout);
    assert!(apply_stdout.contains("repo_removal=accepted"));
    assert!(apply_stdout.contains("repo_removal=terminal"));
    assert!(apply_stdout.contains("outcome=succeeded"));
    apply_stdout
}

#[test]
fn removal_preview_and_apply_cross_process_preserve_workspace_and_git() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let projection = root.join("projection");
    let repo_id = uuid::Uuid::new_v4();
    let repo_id_text = repo_id.to_string();
    let root_text = root.to_str().expect("root UTF-8");
    init_local_repo(root, &projection, "local", repo_id);

    let workspace = projection.join(&repo_id_text);
    std::fs::write(workspace.join("kept.md"), b"# keep").expect("write markdown");
    std::fs::create_dir_all(workspace.join(".git")).expect("create git");
    std::fs::write(workspace.join(".git/config"), b"[core]\n").expect("write git config");
    std::fs::write(workspace.join("unknown.bin"), b"keep").expect("write unknown");

    let apply_stdout = preview_and_apply_removal(root, repo_id);
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

#[test]
fn removal_repair_real_deve_child_fails_closed_with_typed_22() {
    assert!(is_deve_env_key("DEVE_RELEASE_CANDIDATE_IMAGE"));
    assert!(is_deve_env_key("deve_acceptance_removal_repair_outcome"));
    assert!(is_deve_env_key("DeVe_MiXeD"));
    assert!(!is_deve_env_key("NOT_DEVE_VALUE"));

    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let root_text = root.to_str().expect("root UTF-8");
    let ledger = root.join("ledger");
    let removal_dir = ledger.join(".host/repo-lifecycle-jobs/removals");
    std::fs::create_dir_all(&removal_dir).expect("create corrupt removal store");
    let request_id = uuid::Uuid::new_v4().to_string();
    std::fs::write(
        removal_dir.join(format!("{}.json", uuid::Uuid::new_v4())),
        b"{\"format\":\"corrupt\"}\n",
    )
    .expect("write corrupt removal receipt");

    let output = deve(
        root,
        &["repo", "removal-repair", "--request-id", &request_id],
    );
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(22),
        "unexpected repair-required exit:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(stderr.contains("REPO_LIFECYCLE_REPAIR_REQUIRED"));
    assert!(!stderr.contains(root_text), "stderr leaked a host path");
}
