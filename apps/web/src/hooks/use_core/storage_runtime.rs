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

/// 验证并提取有效的 UUID 格式的 repo_id。
/// 如果不是有效的 UUID，返回 None 表示应等待服务端提供真实 ID。
fn repo_scope(repo_id: Option<String>) -> Option<String> {
    repo_id.filter(|value| {
        // 必须是有效的 UUID 格式才认为是真实 repo_id
        !value.is_empty() && uuid::Uuid::parse_str(value).is_ok()
    })
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
        // 必须等到有有效的 UUID 格式的 repo_id
        let Some(repo_id) = repo_scope(current_repo_id.get()) else {
            leptos::logging::log!(
                "Storage: waiting for valid repo UUID, current: {:?}",
                current_repo_id.get()
            );
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

            // 检查 repo 是否已变更
            if repo_scope(current_repo_id.get_untracked()).as_deref() != Some(repo_id.as_str()) {
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
