//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 14_commands#repo-alias-command-contract

use serde_json::json;
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

fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn alias_import_process_reports_all_warnings_and_fails_malformed_documents() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let projection = root.join("projection");
    let repo_id = uuid::Uuid::new_v4();
    let init = deve(
        root,
        &[
            "init",
            "--path",
            root.to_str().expect("root UTF-8"),
            "--repo",
            "local",
            "--projection-base",
            projection.to_str().expect("projection UTF-8"),
            "--repo-id",
            &repo_id.to_string(),
        ],
    );
    assert!(
        init.status.success(),
        "init failed: {}",
        output_text(&init.stderr)
    );

    let unknown = uuid::Uuid::new_v4();
    let invalid_alias_id = uuid::Uuid::new_v4();
    let input = root.join("aliases.json");
    std::fs::write(
        &input,
        serde_json::to_vec(&json!({
            "format": "deve.host-repo-aliases",
            "version": 1,
            "aliases": [
                {"repo_id": repo_id, "alias": "math"},
                {"repo_id": unknown, "alias": "unknown"},
                {"repo_id": invalid_alias_id, "alias": "bad\nname"}
            ]
        }))
        .expect("serialize import"),
    )
    .expect("write import");

    let apply = deve(
        root,
        &[
            "repo",
            "alias",
            "import",
            "--input",
            input.to_str().expect("input UTF-8"),
            "--apply",
        ],
    );
    assert!(
        apply.status.success(),
        "apply failed: {}",
        output_text(&apply.stderr)
    );
    let stdout = output_text(&apply.stdout);
    let stderr = output_text(&apply.stderr);
    assert!(stdout.contains("accepted=1 changed=1 unchanged=0 skipped=2"));
    assert!(stderr.contains("repo_id is not an active local repository"));
    assert!(stderr.contains("alias contains a control character"));
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.contains("warning:"))
            .count(),
        2
    );

    let output = root.join("exported.json");
    let export = deve(
        root,
        &[
            "repo",
            "alias",
            "export",
            "--output",
            output.to_str().expect("output UTF-8"),
        ],
    );
    assert!(
        export.status.success(),
        "export failed: {}",
        output_text(&export.stderr)
    );
    let exported: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output).expect("read export")).expect("parse export");
    assert_eq!(exported["aliases"].as_array().expect("aliases").len(), 1);
    assert_eq!(exported["aliases"][0]["repo_id"], repo_id.to_string());
    assert_eq!(exported["aliases"][0]["alias"], "math");

    let malformed = root.join("malformed.json");
    std::fs::write(&malformed, b"{}").expect("write malformed input");
    let rejected = deve(
        root,
        &[
            "repo",
            "alias",
            "import",
            "--input",
            malformed.to_str().expect("malformed UTF-8"),
            "--apply",
        ],
    );
    assert!(!rejected.status.success());
    assert!(output_text(&rejected.stderr).contains("invalid alias import document"));
}
