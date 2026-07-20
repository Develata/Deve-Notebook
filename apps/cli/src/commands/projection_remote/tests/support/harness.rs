//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use std::path::PathBuf;

pub(in crate::commands::projection_remote::tests) struct ProjectionRemoteHarness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    pub(in crate::commands::projection_remote::tests) workspace: PathBuf,
}

impl ProjectionRemoteHarness {
    pub(in crate::commands::projection_remote::tests) fn ledger_dir(&self) -> PathBuf {
        self.root.join("ledger")
    }
}

pub(in crate::commands::projection_remote::tests) fn initialized_default_repo()
-> ProjectionRemoteHarness {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    crate::commands::init::run(
        &root.join("ledger"),
        "default",
        &root.join("notes"),
        root.clone(),
        8,
        None,
        None,
    )
    .expect("init");
    // Workspace segment is the bare canonical RepoId now (no "default--" alias
    // prefix); init creates exactly one workspace directory under `notes`.
    let workspace = std::fs::read_dir(root.join("notes"))
        .expect("notes dir")
        .map(|entry| entry.expect("workspace entry").path())
        .find(|path| path.is_dir())
        .expect("default workspace");

    ProjectionRemoteHarness {
        _dir: dir,
        root,
        workspace,
    }
}
