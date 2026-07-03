use std::sync::{Arc, Mutex};

use deve_core::config::GitBridgeMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::manifest::{Capability, PluginManifest};
use deve_core::plugin::runtime::host;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, SourceControlApi};
use deve_core::sync::SyncManager;

#[derive(Default)]
struct RecordingSourceControlApi {
    commit_mode: Mutex<Option<GitBridgeMode>>,
}

impl RecordingSourceControlApi {
    fn commit_mode(&self) -> Option<GitBridgeMode> {
        *self.commit_mode.lock().expect("commit mode lock")
    }

    fn unused<T>(&self) -> anyhow::Result<T> {
        Err(anyhow::anyhow!("unexpected source-control API call"))
    }
}

impl SourceControlApi for RecordingSourceControlApi {
    fn list_pending_fs_in_repo(&self, _repo: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        self.unused()
    }

    fn list_staged_in_repo(&self, _repo: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        self.unused()
    }

    fn stage_pending_in_repo(
        &self,
        _repo: &RepoSelector,
        _target: &ScPathTarget,
    ) -> anyhow::Result<()> {
        self.unused()
    }

    fn discard_pending_in_repo(
        &self,
        _repo: &RepoSelector,
        _target: &ScPathTarget,
    ) -> anyhow::Result<()> {
        self.unused()
    }

    fn unstage_file_in_repo(
        &self,
        _repo: &RepoSelector,
        _target: &ScPathTarget,
    ) -> anyhow::Result<()> {
        self.unused()
    }

    fn list_changes_in_repo(&self, _repo: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        self.unused()
    }

    fn diff_doc_path_in_repo(
        &self,
        _repo: &RepoSelector,
        _target: &ScPathTarget,
    ) -> anyhow::Result<String> {
        self.unused()
    }

    fn list_commits_in_repo(
        &self,
        _repo: &RepoSelector,
        _limit: u32,
    ) -> anyhow::Result<Vec<CommitInfo>> {
        self.unused()
    }

    fn diff_commits_in_repo(
        &self,
        _repo: &RepoSelector,
        _commit_a_id: Option<&str>,
        _commit_b_id: &str,
    ) -> anyhow::Result<Vec<CommitFileDiff>> {
        self.unused()
    }

    fn commit_staged_in_repo_with_git_bridge(
        &self,
        _repo: &RepoSelector,
        message: &str,
        git_bridge: GitBridgeMode,
    ) -> anyhow::Result<CommitInfo> {
        *self.commit_mode.lock().expect("commit mode lock") = Some(git_bridge);
        Ok(commit_info(message))
    }

    fn apply_external_changes_in_repo(
        &self,
        _repo: &RepoSelector,
    ) -> anyhow::Result<Vec<ChangeEntry>> {
        self.unused()
    }
}

fn commit_info(message: &str) -> CommitInfo {
    CommitInfo {
        id: "plugin-commit".to_string(),
        parent_id: None,
        message: message.to_string(),
        timestamp: 0,
        doc_count: 0,
        ledger_seq: 0,
    }
}

#[test]
fn plugin_sc_commit_respects_git_bridge_off() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ledger = dir.path().join("ledger");
    let projection = dir.path().join("notes");
    let mut repo =
        RepoManager::init(&ledger, 10, Some("default"), Some("urn:default")).expect("repo init");
    repo.set_projection_base_for_all_local_repos_checked(&projection)
        .expect("projection base");
    repo.ensure_local_repo_workspace_identity("default")
        .expect("workspace identity");
    let repo = Arc::new(repo);
    let sync = Arc::new(SyncManager::new_checked(repo.clone()).expect("sync manager"));
    let api = Arc::new(RecordingSourceControlApi::default());

    host::set_source_control_api(api.clone(), GitBridgeMode::Off).expect("source control api");
    host::set_repo_manager(repo).expect("repo manager");
    host::set_sync_manager(sync).expect("sync manager");

    let manifest = PluginManifest {
        id: "test-source-control".to_string(),
        name: "Test Source Control".to_string(),
        version: "0.0.0".to_string(),
        entry: "main.rhai".to_string(),
        capabilities: Capability {
            allow_source_control: true,
            ..Capability::default()
        },
    };
    let mut engine = rhai::Engine::new();
    host::register_core_api(&mut engine, &manifest);

    let _ = engine
        .eval::<rhai::Dynamic>("sc_commit(\"plugin commit\")")
        .expect("plugin sc_commit");

    assert_eq!(api.commit_mode(), Some(GitBridgeMode::Off));
}
