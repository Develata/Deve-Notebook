//! plan_ref:
//!   - 03_storage#browser-storage-layering
//!   - 04_repository#repo-scope-runtime
//!
use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::VersionVector;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::rc::Rc;

use super::bootstrap;
use super::repo;

fn set_degraded(
    set_mode: WriteSignal<Option<DegradedSyncMode>>,
    set_banner: WriteSignal<Option<String>>,
    mode: DegradedSyncMode,
) {
    set_banner.set(Some(mode.banner_text()));
    set_mode.set(Some(mode));
}

fn clear_degraded(
    set_mode: WriteSignal<Option<DegradedSyncMode>>,
    set_banner: WriteSignal<Option<String>>,
) {
    set_mode.set(None);
    set_banner.set(None);
}

pub(super) fn run_storage_runtime_cycle(
    last_repo: &Rc<RefCell<Option<String>>>,
    current_repo_id: ReadSignal<Option<String>>,
    set_degraded_sync_mode: WriteSignal<Option<DegradedSyncMode>>,
    set_sync_banner: WriteSignal<Option<String>>,
    set_identity: WriteSignal<Option<StoredPeerIdentity>>,
    set_repo_vector: WriteSignal<VersionVector>,
) {
    let current_scope = current_repo_id.get();
    let Some(repo_id) = repo::repo_scope(current_scope) else {
        repo::reset_repo_runtime(last_repo, set_identity, set_repo_vector);
        return;
    };

    let should_skip = last_repo.borrow().as_deref() == Some(repo_id.as_str());
    if should_skip {
        return;
    }
    *last_repo.borrow_mut() = Some(repo_id.clone());

    set_identity.set(None);
    set_repo_vector.set(VersionVector::new());

    spawn_local(async move {
        let bootstrap = match bootstrap::bootstrap_repo_storage(&repo_id).await {
            Ok(bootstrap) => bootstrap,
            Err(mode) => {
                set_degraded(set_degraded_sync_mode, set_sync_banner, mode);
                return;
            }
        };

        if repo::repo_scope(current_repo_id.get_untracked()).as_deref() != Some(repo_id.as_str()) {
            return;
        }

        clear_degraded(set_degraded_sync_mode, set_sync_banner);
        set_repo_vector.set(bootstrap.vector);
        set_identity.set(Some(bootstrap.identity));
    });
}
