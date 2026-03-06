//! 浏览器存储初始化运行时。

use crate::storage::DegradedSyncMode;
use crate::storage::identity::{
    StoredPeerIdentity, load_or_create_identity, load_repo_metadata, probe_capabilities,
    touch_offline_cache,
};
use deve_core::models::VersionVector;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::rc::Rc;

use super::state::CoreSignals;

fn repo_scope(repo: Option<String>) -> String {
    repo.filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

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
pub fn init_storage_runtime(
    signals: &CoreSignals,
) -> (
    ReadSignal<Option<StoredPeerIdentity>>,
    ReadSignal<VersionVector>,
) {
    let (identity, set_identity) = signal(None::<StoredPeerIdentity>);
    let (repo_vector, set_repo_vector) = signal(VersionVector::new());
    let last_repo = Rc::new(RefCell::new(None::<String>));
    let current_repo = signals.current_repo;
    let set_degraded_sync_mode = signals.set_degraded_sync_mode;
    let set_sync_banner = signals.set_sync_banner;
    Effect::new(move |_| {
        let repo_id = repo_scope(current_repo.get());
        if last_repo.borrow().as_deref() == Some(repo_id.as_str()) {
            return;
        }
        *last_repo.borrow_mut() = Some(repo_id.clone());
        set_identity.set(None);
        set_repo_vector.set(VersionVector::new());

        let set_identity = set_identity;
        let set_repo_vector = set_repo_vector;
        spawn_local(async move {
            match probe_capabilities().await {
                Ok(capabilities) => {
                    if let Some(mode) = capabilities.degraded_mode() {
                        set_degraded(set_degraded_sync_mode, set_sync_banner, mode);
                        return;
                    }
                }
                Err(err) => {
                    set_degraded(
                        set_degraded_sync_mode,
                        set_sync_banner,
                        DegradedSyncMode {
                            reason: format!("能力探测失败: {}", err),
                        },
                    );
                    return;
                }
            }

            let identity = match load_or_create_identity(&repo_id).await {
                Ok(identity) => identity,
                Err(err) => {
                    set_degraded(
                        set_degraded_sync_mode,
                        set_sync_banner,
                        DegradedSyncMode {
                            reason: format!("无法恢复浏览器身份: {}", err),
                        },
                    );
                    return;
                }
            };

            let metadata = load_repo_metadata(&repo_id).await.unwrap_or_default();
            if repo_scope(current_repo.get_untracked()) != repo_id {
                return;
            }
            let vector = metadata
                .vector_json
                .as_deref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            clear_degraded(set_degraded_sync_mode, set_sync_banner);
            set_repo_vector.set(vector);
            set_identity.set(Some(identity.clone()));
            if let Err(err) = touch_offline_cache(&repo_id, "bootstrap").await {
                leptos::logging::warn!("离线缓存触点更新失败: {}", err);
            }
            leptos::logging::log!("Frontend PeerId: {}", identity.peer_id);
        });
    });
    (identity, repo_vector)
}
