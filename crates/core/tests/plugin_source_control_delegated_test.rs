use std::sync::{Arc, Mutex};

use deve_core::config::GitBridgeMode;
use deve_core::ledger::traits::RepoSelector;
use deve_core::plugin::manifest::{Capability, PluginManifest};
use deve_core::plugin::runtime::host;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{
    ChangeEntry, CommitFileDiff, CommitInfo, DelegatedSourceControlApi, SourceControlApi,
};

#[derive(Default)]
struct RecordingDelegatedSourceControlApi {
    staged_target: Mutex<Option<ScPathTarget>>,
    diff_target: Mutex<Option<ScPathTarget>>,
}

impl RecordingDelegatedSourceControlApi {
    fn staged_target(&self) -> Option<ScPathTarget> {
        self.staged_target
            .lock()
            .expect("staged target lock")
            .clone()
    }

    fn diff_target(&self) -> Option<ScPathTarget> {
        self.diff_target.lock().expect("diff target lock").clone()
    }

    fn unused<T>(&self) -> anyhow::Result<T> {
        Err(anyhow::anyhow!("unexpected source-control API call"))
    }
}

impl SourceControlApi for RecordingDelegatedSourceControlApi {
    fn list_pending_fs_in_repo(&self, _repo: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        self.unused()
    }

    fn list_staged_in_repo(&self, _repo: &RepoSelector) -> anyhow::Result<Vec<ChangeEntry>> {
        self.unused()
    }

    fn stage_pending_in_repo(
        &self,
        _repo: &RepoSelector,
        target: &ScPathTarget,
    ) -> anyhow::Result<()> {
        *self.staged_target.lock().expect("staged target lock") = Some(target.clone());
        Ok(())
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
        target: &ScPathTarget,
    ) -> anyhow::Result<String> {
        *self.diff_target.lock().expect("diff target lock") = Some(target.clone());
        Ok("delegated diff".to_string())
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
        _message: &str,
        _git_bridge: GitBridgeMode,
    ) -> anyhow::Result<CommitInfo> {
        self.unused()
    }
}

impl DelegatedSourceControlApi for RecordingDelegatedSourceControlApi {}

#[test]
fn plugin_sc_stage_and_diff_use_delegated_path_target_without_local_repo_manager() {
    let api = Arc::new(RecordingDelegatedSourceControlApi::default());
    let delegated_api: Arc<dyn DelegatedSourceControlApi> = api.clone();
    host::set_delegated_source_control_api(delegated_api).expect("delegated source control api");

    let manifest = PluginManifest {
        id: "test-source-control-delegated".to_string(),
        name: "Test Source Control Delegated".to_string(),
        version: "0.0.0".to_string(),
        entry: "main.rhai".to_string(),
        capabilities: Capability {
            allow_source_control: true,
            ..Capability::default()
        },
    };
    let mut engine = rhai::Engine::new();
    host::register_core_api(&mut engine, &manifest);

    engine
        .eval::<()>(r#"sc_stage("notes\\a.md")"#)
        .expect("delegated sc_stage");
    let diff = engine
        .eval::<String>(r#"sc_diff("notes\\a.md")"#)
        .expect("delegated sc_diff");

    assert_eq!(diff, "delegated diff\n");
    assert_eq!(
        api.staged_target(),
        Some(ScPathTarget {
            path: "notes/a.md".to_string(),
            doc_id: None,
            domain: None,
        })
    );
    assert_eq!(
        api.diff_target(),
        Some(ScPathTarget {
            path: "notes/a.md".to_string(),
            doc_id: None,
            domain: None,
        })
    );
}
