use deve_core::models::PeerId;
use leptos::prelude::*;

use super::types::{PendingBranchTarget, RepoSwitchSignals};

pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    active_branch: ReadSignal<Option<PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    set_active_branch: WriteSignal<Option<PeerId>>,
) -> bool {
    let Some(pending) = pending_branch_switch.get_untracked() else {
        leptos::logging::warn!("忽略无 pending 的 BranchSwitched: {:?}", peer_id);
        return false;
    };
    let next_target = peer_id
        .clone()
        .map(PendingBranchTarget::Shadow)
        .unwrap_or(PendingBranchTarget::Local);
    if pending != next_target {
        leptos::logging::warn!("忽略过期 BranchSwitched: {:?}", peer_id);
        return false;
    }
    set_pending_branch_switch.set(None);
    if !success {
        leptos::logging::warn!("分支切换失败");
        return false;
    }

    let next_branch = peer_id.map(PeerId::new);
    let changed = active_branch.get_untracked() != next_branch;
    set_active_branch.set(next_branch);
    changed
}

pub fn handle_repo_switched(name: String, uuid: String, signals: RepoSwitchSignals) -> bool {
    let current_repo = signals.current_repo.get_untracked();
    let current_repo_id = signals.current_repo_id.get_untracked();
    match signals.pending_repo_switch.get_untracked() {
        Some(pending) if pending == name => {
            signals.set_pending_repo_switch.set(None);
        }
        Some(_) => {
            leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
            return false;
        }
        None => {
            let same_repo = current_repo.as_deref() == Some(name.as_str())
                && current_repo_id.as_deref() == Some(uuid.as_str());
            if !same_repo {
                leptos::logging::warn!("忽略无 pending 的 RepoSwitched: {}", name);
                return false;
            }
        }
    }

    let same_repo = !uuid.is_empty() && current_repo_id.as_deref() == Some(uuid.as_str());
    signals.set_current_repo.set(Some(name));
    signals
        .set_current_repo_id
        .set((!uuid.is_empty()).then_some(uuid));
    if !same_repo {
        signals.set_current_doc.set(None);
    }
    !same_repo
}

#[cfg(test)]
mod tests {
    use super::{handle_branch_switched, handle_repo_switched};
    use crate::hooks::use_core::{PendingBranchTarget, RepoSwitchSignals};
    use deve_core::models::{DocId, PeerId};
    use leptos::prelude::*;
    use uuid::Uuid;

    #[test]
    fn clears_doc_when_repo_uuid_changes_even_if_name_matches() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (current_repo, set_current_repo) = signal(Some("default".to_string()));
        let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
        let (pending_repo_switch, set_pending_repo_switch) = signal(Some("default".to_string()));
        let (current_doc, set_current_doc) = signal(Some(DocId::new()));
        let next_repo_id = Uuid::new_v4().to_string();

        let changed = handle_repo_switched(
            "default".to_string(),
            next_repo_id.clone(),
            RepoSwitchSignals {
                current_repo,
                current_repo_id,
                pending_repo_switch,
                set_pending_repo_switch,
                set_current_repo,
                set_current_repo_id,
                set_current_doc,
            },
        );

        assert!(changed);
        assert_eq!(current_repo_id.get_untracked(), Some(next_repo_id));
        assert_eq!(current_doc.get_untracked(), None);
        assert_eq!(pending_repo_switch.get_untracked(), None);
    }

    #[test]
    fn branch_switch_reports_when_scope_changed() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (active_branch, set_active_branch) = signal(Some(PeerId::new("peer-a")));
        let (pending_branch_switch, set_pending_branch_switch) =
            signal(Some(PendingBranchTarget::Shadow("peer-b".into())));
        let changed = handle_branch_switched(
            Some("peer-b".into()),
            true,
            active_branch,
            pending_branch_switch,
            set_pending_branch_switch,
            set_active_branch,
        );

        assert!(changed);
        assert_eq!(active_branch.get_untracked(), Some(PeerId::new("peer-b")));
        assert_eq!(pending_branch_switch.get_untracked(), None);
    }

    #[test]
    fn ignores_branch_switched_without_pending_target() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (active_branch, set_active_branch) = signal(Some(PeerId::new("peer-a")));
        let (pending_branch_switch, set_pending_branch_switch) = signal(None);

        assert!(!handle_branch_switched(
            Some("peer-b".into()),
            true,
            active_branch,
            pending_branch_switch,
            set_pending_branch_switch,
            set_active_branch,
        ));
        assert_eq!(active_branch.get_untracked(), Some(PeerId::new("peer-a")));
    }

    #[test]
    fn ignores_stale_repo_switched_while_newer_target_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (current_repo, set_current_repo) = signal(Some("test".to_string()));
        let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
        let (pending_repo_switch, set_pending_repo_switch) = signal(Some("default".to_string()));
        let (_current_doc, set_current_doc) = signal(Some(DocId::new()));

        let changed = handle_repo_switched(
            "stale".to_string(),
            Uuid::new_v4().to_string(),
            RepoSwitchSignals {
                current_repo,
                current_repo_id,
                pending_repo_switch,
                set_pending_repo_switch,
                set_current_repo,
                set_current_repo_id,
                set_current_doc,
            },
        );

        assert!(!changed);
        assert_eq!(
            pending_repo_switch.get_untracked(),
            Some("default".to_string())
        );
    }

    #[test]
    fn ignores_repo_switched_without_pending_when_repo_differs() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();

        let (current_repo, set_current_repo) = signal(Some("default".to_string()));
        let (current_repo_id, set_current_repo_id) = signal(Some(Uuid::new_v4().to_string()));
        let (pending_repo_switch, set_pending_repo_switch) = signal(None);
        let doc_id = DocId::new();
        let (current_doc, set_current_doc) = signal(Some(doc_id));

        assert!(!handle_repo_switched(
            "test".to_string(),
            Uuid::new_v4().to_string(),
            RepoSwitchSignals {
                current_repo,
                current_repo_id,
                pending_repo_switch,
                set_pending_repo_switch,
                set_current_repo,
                set_current_repo_id,
                set_current_doc,
            },
        ));
        assert_eq!(current_doc.get_untracked(), Some(doc_id));
    }
}
