use std::sync::{Arc, Mutex};

use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::manifest::{Capability, PluginManifest};
use deve_core::plugin::runtime::host;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{
    ChangeEntry, CommitFileDiff, CommitInfo, ExternalApplyReceipt, SourceControlApi,
};
use deve_core::sync::SyncManager;

#[derive(Default)]
struct RecordingSourceControlApi;

#[derive(Default)]
struct RecordingManagedSourceControlHost {
    commit_message: Mutex<Option<String>>,
}

impl host::ManagedSourceControlMutationHost for RecordingManagedSourceControlHost {
    fn commit_source_control(
        &self,
        intent: host::ManagedSourceControlCommitIntent,
    ) -> anyhow::Result<CommitInfo> {
        *self.commit_message.lock().expect("commit message lock") = Some(intent.message.clone());
        Ok(commit_info(&intent.message))
    }
}

impl RecordingManagedSourceControlHost {
    fn commit_message(&self) -> Option<String> {
        self.commit_message
            .lock()
            .expect("commit message lock")
            .clone()
    }
}

impl RecordingSourceControlApi {
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

    fn commit_source_control_changes_in_repo(
        &self,
        _repo: &RepoSelector,
        _message: &str,
    ) -> anyhow::Result<CommitInfo> {
        self.unused()
    }

    fn apply_external_changes_in_repo(
        &self,
        _repo: &RepoSelector,
    ) -> anyhow::Result<ExternalApplyReceipt> {
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
fn plugin_sc_commit_uses_ngit_authority_api() {
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
    let api = Arc::new(RecordingSourceControlApi);
    let managed = Arc::new(RecordingManagedSourceControlHost::default());

    host::set_source_control_api(api.clone()).expect("source control api");
    host::set_managed_source_control_mutation_host(managed.clone())
        .expect("managed source control host");
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

    assert_eq!(managed.commit_message().as_deref(), Some("plugin commit"));
}
