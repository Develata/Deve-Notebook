//! 浏览器存储初始化运行时。

use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::VersionVector;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::rc::Rc;

use super::state::CoreSignals;

#[path = "storage_runtime_bootstrap.rs"]
mod bootstrap;
#[path = "storage_runtime_repo.rs"]
mod repo;

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

/// 初始化浏览器身份与 repo 级向量缓存。
/// 仅当 repo_id 是有效 UUID 时才执行，避免使用 "default" 等字符串。
pub fn init_storage_runtime(
    signals: &CoreSignals,
) -> (
    ReadSignal<Option<StoredPeerIdentity>>,
    ReadSignal<VersionVector>,
) {
    let (identity, set_identity) = signal(None::<StoredPeerIdentity>);
    let (repo_vector, set_repo_vector) = signal(VersionVector::new());
    let last_repo = Rc::new(RefCell::new(None::<String>));
    let current_repo_id = signals.current_repo_id;
    let set_degraded_sync_mode = signals.set_degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;

    Effect::new(move |_| {
        let current_scope = current_repo_id.get();
        // 必须等到有有效的 UUID 格式的 repo_id
        let Some(repo_id) = repo::repo_scope(current_scope.clone()) else {
            repo::reset_repo_runtime(&last_repo, set_identity, set_repo_vector);
            return;
        };

        if last_repo.borrow().as_deref() == Some(repo_id.as_str()) {
            return;
        }

        *last_repo.borrow_mut() = Some(repo_id.clone());
        set_identity.set(None);
        set_repo_vector.set(VersionVector::new());

        let set_identity = set_identity;
        let set_repo_vector = set_repo_vector;
        let current_repo_id = current_repo_id;

        spawn_local(async move {
            let bootstrap = match bootstrap::bootstrap_repo_storage(&repo_id).await {
                Ok(bootstrap) => bootstrap,
                Err(mode) => {
                    set_degraded(set_degraded_sync_mode, set_sync_banner, mode);
                    return;
                }
            };

            if repo::repo_scope(current_repo_id.get_untracked()).as_deref()
                != Some(repo_id.as_str())
            {
                return;
            }

            clear_degraded(set_degraded_sync_mode, set_sync_banner);
            set_repo_vector.set(bootstrap.vector);
            set_identity.set(Some(bootstrap.identity));
        });
    });

    (identity, repo_vector)
}
