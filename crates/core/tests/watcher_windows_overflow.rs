#![cfg(windows)]
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! W9 Windows kernel-overflow receipt producer. Ordinary test runs ignore both
//! entrypoints. The parent entrypoint verifies the immutable notify git binding,
//! then launches the child entrypoint in three independent processes.

mod common;
mod watcher_test_support;
mod watcher_windows_overflow_support;

use anyhow::{Context, ensure};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use watcher_windows_overflow_support::{
    BURST_FILE_COUNT, OverflowClaims, OverflowRun, run_one_overflow_probe,
    verify_notify_dependency_binding, workspace_root,
};

const CHILD_PROCESS_TIMEOUT: Duration = Duration::from_secs(120);
const CHILD_TERMINATION_TIMEOUT: Duration = Duration::from_secs(5);
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[test]
#[ignore = "W9 receipt producer; requires an approved immutable notify git override"]
fn watcher_windows_overflow_three_process_producer() -> anyhow::Result<()> {
    let root = workspace_root()?;
    let dependency = verify_notify_dependency_binding(&root)?;
    let result_dir = tempfile::tempdir().context("create W9 child result directory")?;
    let current_test = std::env::current_exe().context("resolve W9 test executable")?;
    let mut runs = Vec::with_capacity(3);

    for run_index in 0..3 {
        let result_path = result_dir.path().join(format!("run-{run_index}.json"));
        let output = run_child(&current_test, &result_path, run_index)?;
        ensure!(
            output.status.success(),
            "W9 overflow child {run_index} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = fs::read(&result_path)
            .with_context(|| format!("read W9 child result {}", result_path.display()))?;
        runs.push(
            serde_json::from_slice::<OverflowRun>(&bytes)
                .with_context(|| format!("decode W9 child result {}", result_path.display()))?,
        );
    }

    validate_runs(&runs)?;
    let claims = OverflowClaims {
        schema: 1,
        producer: "storage.watcher-windows-overflow",
        head: git_output(&root, ["rev-parse", "HEAD"])?,
        dependency,
        runs,
    };
    let claims_json = serde_json::to_vec_pretty(&claims)?;
    if let Some(path) = std::env::var_os("DEVE_W9_OVERFLOW_CLAIMS") {
        write_json_atomically(&PathBuf::from(path), &claims_json)
            .context("write W9 overflow claims")?;
    } else {
        println!("{}", String::from_utf8_lossy(&claims_json));
    }
    Ok(())
}

#[test]
#[ignore = "spawned only by watcher_windows_overflow_three_process_producer"]
fn watcher_windows_overflow_single_process_child() -> anyhow::Result<()> {
    ensure!(
        std::env::var_os("DEVE_W9_OVERFLOW_CHILD").is_some(),
        "W9 child entrypoint must be launched by the parent producer"
    );
    let result_path =
        PathBuf::from(std::env::var_os("DEVE_W9_OVERFLOW_RESULT").context("child result path")?);
    let run = run_one_overflow_probe()?;
    write_json_atomically(&result_path, &serde_json::to_vec_pretty(&run)?)
        .with_context(|| format!("write W9 child result {}", result_path.display()))?;
    Ok(())
}

fn validate_runs(runs: &[OverflowRun]) -> anyhow::Result<()> {
    ensure!(runs.len() == 3, "W9 requires exactly three child runs");
    let process_ids = runs
        .iter()
        .map(|run| run.process_id)
        .collect::<BTreeSet<_>>();
    ensure!(
        process_ids.len() == runs.len(),
        "W9 child runs did not use independent processes"
    );
    let first = runs.first().context("first W9 child run")?;
    for run in runs {
        ensure!(
            run.burst_file_count == BURST_FILE_COUNT
                && run.rescan_seen
                && run.post_rearm_event_seen,
            "W9 child did not prove the complete overflow/rearm sequence: {run:?}"
        );
        ensure!(
            run.pending_file_count == BURST_FILE_COUNT + 2,
            "W9 child pending count does not include barrier, burst and sentinel: {run:?}"
        );
        ensure!(
            run.expected_hash == run.actual_hash,
            "W9 child pending hash mismatch: {run:?}"
        );
        ensure!(
            run.expected_hash == first.expected_hash
                && run.pending_file_count == first.pending_file_count
                && run.windows_build == first.windows_build
                && run.filesystem == first.filesystem,
            "W9 child runs did not converge to the same deterministic state"
        );
    }
    Ok(())
}

fn run_child(current_test: &Path, result_path: &Path, run_index: usize) -> anyhow::Result<Output> {
    let stdout_path = result_path.with_extension("stdout.log");
    let stderr_path = result_path.with_extension("stderr.log");
    let stdout = fs::File::create(&stdout_path)
        .with_context(|| format!("create W9 child stdout {}", stdout_path.display()))?;
    let stderr = fs::File::create(&stderr_path)
        .with_context(|| format!("create W9 child stderr {}", stderr_path.display()))?;
    let mut child = Command::new(current_test)
        .arg("--exact")
        .arg("watcher_windows_overflow_single_process_child")
        .arg("--ignored")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("DEVE_W9_OVERFLOW_CHILD", "1")
        .env("DEVE_W9_OVERFLOW_RESULT", result_path)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("launch W9 overflow child {run_index}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return collect_child_output(status, &stdout_path, &stderr_path),
            Ok(None) => {}
            Err(error) => {
                return Err(terminate_child(
                    &mut child,
                    &stdout_path,
                    &stderr_path,
                    format!("poll W9 overflow child {run_index} failed: {error}"),
                ));
            }
        }
        if started.elapsed() >= CHILD_PROCESS_TIMEOUT {
            return Err(terminate_child(
                &mut child,
                &stdout_path,
                &stderr_path,
                format!("W9 overflow child {run_index} timed out after {CHILD_PROCESS_TIMEOUT:?}"),
            ));
        }
        thread::sleep(CHILD_POLL_INTERVAL);
    }
}

fn collect_child_output(
    status: ExitStatus,
    stdout_path: &Path,
    stderr_path: &Path,
) -> anyhow::Result<Output> {
    Ok(Output {
        status,
        stdout: fs::read(stdout_path)
            .with_context(|| format!("read W9 child stdout {}", stdout_path.display()))?,
        stderr: fs::read(stderr_path)
            .with_context(|| format!("read W9 child stderr {}", stderr_path.display()))?,
    })
}

fn terminate_child(
    child: &mut Child,
    stdout_path: &Path,
    stderr_path: &Path,
    primary: String,
) -> anyhow::Error {
    let child_id = child.id();
    let kill_diagnostic = child.kill().map_or_else(
        |error| format!("kill failed: {error}"),
        |()| "kill requested".into(),
    );
    let started = Instant::now();
    let reap_diagnostic = loop {
        match child.try_wait() {
            Ok(Some(status)) => break format!("reaped with {status}"),
            Ok(None) if started.elapsed() < CHILD_TERMINATION_TIMEOUT => {
                thread::sleep(CHILD_POLL_INTERVAL);
            }
            Ok(None) => break format!("not reaped after {CHILD_TERMINATION_TIMEOUT:?}"),
            Err(error) => break format!("reap poll failed: {error}"),
        }
    };
    anyhow::anyhow!(
        "{primary}; pid={child_id}; termination={kill_diagnostic}; reap={reap_diagnostic}\n\
         stdout:\n{}\nstderr:\n{}",
        read_child_log(stdout_path),
        read_child_log(stderr_path)
    )
}

fn read_child_log(path: &Path) -> String {
    fs::read(path).map_or_else(
        |error| format!("<failed to read {}: {error}>", path.display()),
        |bytes| String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn write_json_atomically(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    ensure!(
        parent.is_dir(),
        "W9 result parent is not a directory: {}",
        parent.display()
    );
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).context("create W9 temporary result file")?;
    temporary
        .write_all(bytes)
        .context("write W9 temporary result file")?;
    temporary
        .as_file()
        .sync_all()
        .context("sync W9 temporary result file")?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| error.error)
        .with_context(|| format!("atomically publish W9 result {}", path.display()))?;
    Ok(())
}

fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("run git for W9 producer")?;
    ensure!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[cfg(test)]
mod dependency_binding_tests {
    use super::*;
    use serde_json::{Value, json};

    const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
    const OTHER_REVISION: &str = "89abcdef0123456789abcdef0123456789abcdef";
    const CRATES_IO_SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";

    #[test]
    fn dependency_binding_accepts_both_approved_identity_shapes() {
        let notify_source = notify_source("https://example.invalid/notify", REVISION);
        let registry_lock = lockfile("2.1.0", CRATES_IO_SOURCE, "");
        let registry_fixture = fixture(
            &registry_lock,
            override_registry(&notify_source, None, true),
        );
        assert!(
            verify_notify_dependency_binding(registry_fixture.path()).is_ok(),
            "registry notify-types identity should be accepted"
        );

        let git_lock = lockfile("2.0.0", &notify_source, "");
        let git_fixture = fixture(
            &git_lock,
            override_registry(&notify_source, Some(&notify_source), true),
        );
        assert!(
            verify_notify_dependency_binding(git_fixture.path()).is_ok(),
            "coherent git notify-types identity should be accepted"
        );
    }

    #[test]
    fn dependency_binding_rejects_identity_and_registry_drift() {
        let primary_source = notify_source("https://example.invalid/notify", REVISION);
        assert_binding_error(
            lockfile(
                "2.1.0",
                CRATES_IO_SOURCE,
                "\n[[package]]\nname = \"notify\"\nversion = \"9.0.0\"\nsource = \
                 \"registry+https://github.com/rust-lang/crates.io-index\"\n",
            ),
            override_registry(&primary_source, None, true),
            "exactly one notify package identity",
        );
        assert_binding_error(
            lockfile(
                "2.1.0",
                CRATES_IO_SOURCE,
                &format!(
                    "\n[[package]]\nname = \"notify-types\"\nversion = \"2.0.0\"\nsource = \
                    \"{primary_source}\"\n"
                ),
            ),
            override_registry(&primary_source, Some(&primary_source), true),
            "exactly one notify-types package identity",
        );
        assert_binding_error(
            lockfile("2.1.0", "registry+https://example.invalid/crates-index", ""),
            override_registry(&primary_source, None, true),
            "must be crates.io version 2.1.0",
        );
        assert_binding_error(
            lockfile("2.0.0", CRATES_IO_SOURCE, ""),
            override_registry(&primary_source, None, true),
            "must be crates.io version 2.1.0",
        );

        let other_source = notify_source("https://example.invalid/other-notify", REVISION);
        assert_binding_error(
            lockfile("2.0.0", &other_source, ""),
            override_registry(&primary_source, Some(&other_source), true),
            "must use the same source",
        );
        let other_revision_source = notify_source("https://example.invalid/notify", OTHER_REVISION);
        assert_binding_error(
            lockfile("2.0.0", &other_revision_source, ""),
            override_registry(&primary_source, Some(&other_revision_source), true),
            "must use the same immutable revision",
        );
        assert_binding_error(
            lockfile("2.1.0", CRATES_IO_SOURCE, ""),
            override_registry(&primary_source, None, false),
            "exactly one notify entry",
        );
    }

    fn notify_source(url: &str, revision: &str) -> String {
        format!("git+{url}?branch=w9#{revision}")
    }

    fn lockfile(notify_types_version: &str, notify_types_source: &str, extra: &str) -> String {
        format!(
            "version = 4\n\
             \n\
             [[package]]\n\
             name = \"notify\"\n\
             version = \"8.2.0\"\n\
             source = \"{}\"\n\
             \n\
             [[package]]\n\
             name = \"notify-types\"\n\
             version = \"{notify_types_version}\"\n\
             source = \"{notify_types_source}\"\n\
             {extra}",
            notify_source("https://example.invalid/notify", REVISION)
        )
    }

    fn override_registry(
        notify_source: &str,
        notify_types_source: Option<&str>,
        include_notify: bool,
    ) -> Value {
        let mut overrides = Vec::new();
        if include_notify {
            overrides.push(json!({
                "package": "notify",
                "source": notify_source,
                "revision": REVISION,
                "patch": "tools/patches/notify/8.2.0-windows-overflow-rescan.patch"
            }));
        }
        if let Some(source) = notify_types_source {
            overrides.push(json!({
                "package": "notify-types",
                "source": source,
                "revision": source.rsplit_once('#').unwrap().1
            }));
        }
        json!({ "overrides": overrides })
    }

    fn fixture(lockfile: &str, registry: Value) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("docs/registry")).unwrap();
        fs::create_dir_all(root.path().join("tools/patches/notify")).unwrap();
        fs::write(root.path().join("Cargo.lock"), lockfile).unwrap();
        fs::write(
            root.path()
                .join("docs/registry/dependency-source-overrides.json"),
            serde_json::to_vec(&registry).unwrap(),
        )
        .unwrap();
        fs::write(
            root.path()
                .join("tools/patches/notify/8.2.0-windows-overflow-rescan.patch"),
            b"reviewed W9 patch fixture",
        )
        .unwrap();
        root
    }

    fn assert_binding_error(lockfile: String, registry: Value, expected: &str) {
        let root = fixture(&lockfile, registry);
        let error = verify_notify_dependency_binding(root.path()).unwrap_err();
        let diagnostic = format!("{error:#}");
        assert!(
            diagnostic.contains(expected),
            "expected {expected:?} in dependency diagnostic: {diagnostic}"
        );
    }
}
