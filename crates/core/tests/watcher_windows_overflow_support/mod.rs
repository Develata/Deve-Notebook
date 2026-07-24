//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! One-process Windows overflow probe and immutable dependency-source checks.

use crate::watcher_test_support::Harness;
use anyhow::{Context, bail, ensure};
use deve_core::source_control::{ChangeStatus, pending_fs};
use deve_core::sync::watcher::{RepoWatcherHandle, RepoWatcherStart};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};
use windows_sys::Win32::Storage::FileSystem::{GetVolumeInformationW, GetVolumePathNameW};

pub(super) const BURST_FILE_COUNT: usize = 2048;
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(30);
const EVENT_TIMEOUT: Duration = Duration::from_secs(20);
const NOTIFY_PACKAGE_VERSION: &str = "8.2.0";
const REGISTRY_NOTIFY_TYPES_VERSION: &str = "2.1.0";
const GIT_NOTIFY_TYPES_VERSION: &str = "2.0.0";
const CRATES_IO_SOURCE: &str = "registry+https://github.com/rust-lang/crates.io-index";
const OVERRIDE_REGISTRY: &str = "docs/registry/dependency-source-overrides.json";
const PATCH_PATH: &str = "tools/patches/notify/8.2.0-windows-overflow-rescan.patch";

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct OverflowRun {
    pub(super) process_id: u32,
    pub(super) windows_build: String,
    pub(super) filesystem: String,
    pub(super) burst_file_count: usize,
    pub(super) rescan_seen: bool,
    pub(super) post_rearm_event_seen: bool,
    pub(super) pending_file_count: usize,
    pub(super) expected_hash: String,
    pub(super) actual_hash: String,
}

#[derive(Debug, Serialize)]
pub(super) struct DependencyBinding {
    notify_source: String,
    notify_revision: String,
    notify_types_version: String,
    notify_types_source: String,
    patch_sha256: String,
}

#[derive(Debug, Serialize)]
pub(super) struct OverflowClaims {
    pub(super) schema: u8,
    pub(super) producer: &'static str,
    pub(super) head: String,
    pub(super) dependency: DependencyBinding,
    pub(super) runs: Vec<OverflowRun>,
}

pub(super) fn run_one_overflow_probe() -> anyhow::Result<OverflowRun> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();
    let root = h.workspace_root(&repo_name)?;
    let burst_root = root.join("w9-overflow");
    fs::create_dir_all(&burst_root)?;

    let (barrier_entered_tx, barrier_entered_rx) = mpsc::sync_channel(1);
    let (barrier_release_tx, barrier_release_rx) = mpsc::channel();
    let (barrier_released_tx, barrier_released_rx) = mpsc::sync_channel(1);
    let release_rx = Arc::new(Mutex::new(barrier_release_rx));
    let barrier_armed = Arc::new(AtomicBool::new(true));
    let (event_tx, event_rx) = mpsc::channel::<notify::Result<Event>>();

    let callback_release = Arc::clone(&release_rx);
    let callback_barrier = Arc::clone(&barrier_armed);
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            if callback_barrier.swap(false, Ordering::SeqCst) {
                let _ = barrier_entered_tx.send(());
                let released = callback_release
                    .lock()
                    .map_err(|_| ())
                    .and_then(|receiver| {
                        receiver
                            .recv_timeout(CALLBACK_TIMEOUT)
                            .map(|_| ())
                            .map_err(|_| ())
                    });
                let _ = barrier_released_tx.send(released.is_ok());
            }
            let _ = event_tx.send(event);
        },
        Config::default(),
    )?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    let mut expected = BTreeMap::new();
    write_expected_file(&root, "w9-overflow/barrier.md", "barrier", &mut expected)?;
    barrier_entered_rx
        .recv_timeout(EVENT_TIMEOUT)
        .context("notify callback did not enter the W9 barrier")?;

    for index in 0..BURST_FILE_COUNT {
        let path = format!(
            "w9-overflow/{index:04}-{}-burst.md",
            "overflow-payload".repeat(4)
        );
        let content = format!("overflow-{index:04}");
        write_expected_file(&root, &path, &content, &mut expected)?;
    }
    barrier_release_tx
        .send(())
        .context("release W9 callback barrier")?;
    ensure!(
        barrier_released_rx
            .recv_timeout(EVENT_TIMEOUT)
            .context("notify callback did not report W9 barrier release")?,
        "notify callback barrier expired before the 2048-file burst completed"
    );

    wait_for_event(&event_rx, EVENT_TIMEOUT, Event::need_rescan)
        .context("notify did not surface the Windows overflow as Rescan")?;

    let sentinel = "w9-overflow/post-rearm.md";
    write_expected_file(&root, sentinel, "post-rearm", &mut expected)?;
    wait_for_event(&event_rx, EVENT_TIMEOUT, |event| {
        !event.need_rescan()
            && event
                .paths
                .iter()
                .any(|path| path.ends_with(Path::new(sentinel)))
    })
    .context("notify did not deliver a normal event after overflow rearm")?;
    drop(watcher);

    let handle =
        RepoWatcherHandle::start(RepoWatcherStart::resolve(h.sync.clone(), &repo_name, 1)?)?;
    handle.shutdown()?;
    let pending = h.repo.run_on_local_repo(&repo_name, pending_fs::list_all)?;
    let mut actual = BTreeMap::new();
    for entry in pending {
        ensure!(
            entry.change_type == ChangeStatus::Added,
            "overflow reconcile produced non-added candidate {}: {:?}",
            entry.path,
            entry.change_type
        );
        ensure!(
            entry.renamed_from.is_none() && entry.doc_id.is_none() && !entry.has_conflict,
            "overflow reconcile produced unexpected pending metadata for {}",
            entry.path
        );
        ensure!(
            actual
                .insert(entry.path.clone(), entry.content_hash)
                .is_none(),
            "overflow reconcile produced duplicate candidate {}",
            entry.path
        );
    }
    let expected_hash = pending_set_hash(&expected);
    let actual_hash = pending_set_hash(&actual);
    ensure!(
        actual == expected,
        "reconciled pending set differs from independent expected set: expected={} actual={}",
        expected.len(),
        actual.len()
    );

    Ok(OverflowRun {
        process_id: std::process::id(),
        windows_build: windows_build()?,
        filesystem: filesystem_name(&root)?,
        burst_file_count: BURST_FILE_COUNT,
        rescan_seen: true,
        post_rearm_event_seen: true,
        pending_file_count: actual.len(),
        expected_hash,
        actual_hash,
    })
}

fn write_expected_file(
    root: &Path,
    relative: &str,
    content: &str,
    expected: &mut BTreeMap<String, String>,
) -> anyhow::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    ensure!(
        expected
            .insert(relative.to_string(), pending_fs::content_hash(content))
            .is_none(),
        "duplicate expected W9 path {relative}"
    );
    Ok(())
}

fn wait_for_event(
    receiver: &mpsc::Receiver<notify::Result<Event>>,
    timeout: Duration,
    predicate: impl Fn(&Event) -> bool,
) -> anyhow::Result<Event> {
    let started = Instant::now();
    loop {
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            bail!("timed out after {timeout:?}");
        }
        match receiver.recv_timeout(remaining) {
            Ok(Ok(event)) if predicate(&event) => return Ok(event),
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(error.into()),
            Err(mpsc::RecvTimeoutError::Timeout) => bail!("timed out after {timeout:?}"),
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("notify event channel disconnected"),
        }
    }
}

fn pending_set_hash(entries: &BTreeMap<String, String>) -> String {
    let mut digest = Sha256::new();
    for (path, content_hash) in entries {
        digest.update(path.as_bytes());
        digest.update([0]);
        digest.update(content_hash.as_bytes());
        digest.update(b"\n");
    }
    hex::encode(digest.finalize())
}

pub(super) fn verify_notify_dependency_binding(root: &Path) -> anyhow::Result<DependencyBinding> {
    let lock: toml::Value = fs::read_to_string(root.join("Cargo.lock"))?
        .parse()
        .context("parse Cargo.lock")?;
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .context("Cargo.lock package array")?;
    let notify = packages
        .iter()
        .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some("notify"))
        .collect::<Vec<_>>();
    ensure!(
        notify.len() == 1,
        "Cargo.lock must contain exactly one notify package identity, found {}",
        notify.len()
    );
    let notify = notify[0];
    ensure!(
        notify.get("version").and_then(toml::Value::as_str) == Some(NOTIFY_PACKAGE_VERSION),
        "the single notify package identity must be version {NOTIFY_PACKAGE_VERSION}"
    );
    let notify_source = package_source(notify, "notify")?;
    ensure!(
        notify_source.starts_with("git+"),
        "notify dependency source is not git-bound: {notify_source}"
    );
    let notify_revision = git_revision(&notify_source)?;

    let notify_types = packages
        .iter()
        .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some("notify-types"))
        .collect::<Vec<_>>();
    ensure!(
        notify_types.len() == 1,
        "Cargo.lock must contain exactly one notify-types package identity, found {}",
        notify_types.len()
    );
    let notify_types = notify_types[0];
    let notify_types_version = notify_types
        .get("version")
        .and_then(toml::Value::as_str)
        .context("notify-types Cargo.lock version")?
        .to_string();
    let notify_types_source = package_source(notify_types, "notify-types")?;
    ensure!(
        notify_types_source.starts_with("git+") || notify_types_source.starts_with("registry+"),
        "notify-types dependency source is neither git nor registry bound: {notify_types_source}"
    );
    if notify_types_source.starts_with("git+") {
        ensure!(
            notify_types_version == GIT_NOTIFY_TYPES_VERSION,
            "git-bound notify-types must be version {GIT_NOTIFY_TYPES_VERSION}"
        );
        ensure!(
            git_source_without_revision(&notify_source)?
                == git_source_without_revision(&notify_types_source)?,
            "git-bound notify and notify-types must use the same source"
        );
        ensure!(
            notify_revision == git_revision(&notify_types_source)?,
            "git-bound notify and notify-types must use the same immutable revision"
        );
    } else {
        ensure!(
            notify_types_version == REGISTRY_NOTIFY_TYPES_VERSION
                && notify_types_source == CRATES_IO_SOURCE,
            "registry-bound notify-types must be crates.io version {REGISTRY_NOTIFY_TYPES_VERSION}"
        );
    }

    verify_override_registry(root, &notify_source, &notify_revision, &notify_types_source)?;
    let patch = fs::read(root.join(PATCH_PATH)).context("read reviewed W9 notify patch")?;
    Ok(DependencyBinding {
        notify_source,
        notify_revision,
        notify_types_version,
        notify_types_source,
        patch_sha256: hex::encode(Sha256::digest(patch)),
    })
}

fn package_source(package: &toml::Value, name: &str) -> anyhow::Result<String> {
    package
        .get("source")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("{name} dependency has no immutable registry or git source"))
}

fn git_revision(source: &str) -> anyhow::Result<String> {
    git_source_without_revision(source).and_then(|_| {
        source
            .rsplit_once('#')
            .map(|(_, revision)| revision)
            .filter(|revision| {
                revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            .map(str::to_ascii_lowercase)
            .context("notify git source lacks an immutable 40-character revision")
    })
}

fn git_source_without_revision(source: &str) -> anyhow::Result<&str> {
    source
        .strip_prefix("git+")
        .and_then(|_| source.rsplit_once('#').map(|(source, _)| source))
        .context("dependency source is not an immutable git source")
}

fn verify_override_registry(
    root: &Path,
    notify_source: &str,
    notify_revision: &str,
    notify_types_source: &str,
) -> anyhow::Result<()> {
    let path = root.join(OVERRIDE_REGISTRY);
    let registry: serde_json::Value = serde_json::from_slice(
        &fs::read(&path)
            .with_context(|| format!("read dependency override registry {}", path.display()))?,
    )
    .context("parse dependency override registry")?;
    let overrides = registry
        .get("overrides")
        .and_then(serde_json::Value::as_array)
        .context("dependency override registry entries")?;
    verify_registry_entry(overrides, "notify", notify_source, notify_revision, true)?;
    if notify_types_source.starts_with("git+") {
        verify_registry_entry(
            overrides,
            "notify-types",
            notify_types_source,
            &git_revision(notify_types_source)?,
            false,
        )?;
    } else {
        ensure!(
            !overrides.iter().any(|entry| {
                entry.get("package").and_then(serde_json::Value::as_str) == Some("notify-types")
            }),
            "registry-bound notify-types must not have a dependency source override entry"
        );
    }
    Ok(())
}

fn verify_registry_entry(
    overrides: &[serde_json::Value],
    package: &str,
    source: &str,
    revision: &str,
    requires_patch: bool,
) -> anyhow::Result<()> {
    let entries = overrides
        .iter()
        .filter(|entry| entry.get("package").and_then(serde_json::Value::as_str) == Some(package))
        .collect::<Vec<_>>();
    ensure!(
        entries.len() == 1,
        "dependency override registry must contain exactly one {package} entry, found {}",
        entries.len()
    );
    let entry = entries[0];
    ensure!(
        entry.get("source").and_then(serde_json::Value::as_str) == Some(source),
        "dependency override registry source differs from Cargo.lock for {package}"
    );
    ensure!(
        entry.get("revision").and_then(serde_json::Value::as_str) == Some(revision),
        "dependency override registry revision differs from Cargo.lock for {package}"
    );
    if requires_patch {
        ensure!(
            entry.get("patch").and_then(serde_json::Value::as_str) == Some(PATCH_PATH),
            "dependency override registry patch path is not the reviewed W9 patch"
        );
    }
    Ok(())
}

pub(super) fn workspace_root() -> anyhow::Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("resolve workspace root from deve_core manifest")
}

fn windows_build() -> anyhow::Result<String> {
    let output = Command::new("cmd")
        .args(["/D", "/C", "ver"])
        .output()
        .context("query Windows build")?;
    ensure!(output.status.success(), "cmd /C ver failed");
    let build = String::from_utf8_lossy(&output.stdout).trim().to_string();
    ensure!(!build.is_empty(), "Windows build string is empty");
    Ok(build)
}

fn filesystem_name(path: &Path) -> anyhow::Result<String> {
    let path_wide = wide_null(path.as_os_str());
    let mut volume_path = [0u16; 261];
    let volume_ok = unsafe {
        GetVolumePathNameW(
            path_wide.as_ptr(),
            volume_path.as_mut_ptr(),
            volume_path.len() as u32,
        )
    };
    ensure!(
        volume_ok != 0,
        "GetVolumePathNameW failed: {}",
        std::io::Error::last_os_error()
    );
    let mut filesystem = [0u16; 64];
    let info_ok = unsafe {
        GetVolumeInformationW(
            volume_path.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    };
    ensure!(
        info_ok != 0,
        "GetVolumeInformationW failed: {}",
        std::io::Error::last_os_error()
    );
    let end = filesystem
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(filesystem.len());
    let name = std::ffi::OsString::from_wide(&filesystem[..end])
        .to_string_lossy()
        .into_owned();
    ensure!(!name.is_empty(), "filesystem name is empty");
    Ok(name)
}

fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
