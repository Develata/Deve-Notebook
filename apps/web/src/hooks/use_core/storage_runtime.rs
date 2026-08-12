//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!   - 04_repository#repo-scope-runtime
//!
//! 浏览器存储初始化运行时。

use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::VersionVector;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use super::state::CoreSignals;
use crate::runtime::browser_runtime_lifetime::BrowserRuntimeLifetime;

mod bootstrap;
mod effect;
mod repo;

/// 初始化浏览器身份与 repo 级向量缓存。
/// 仅当 repo_id 是有效 UUID 时才执行，避免使用 "default" 等字符串。
pub fn init_storage_runtime(
    signals: &CoreSignals,
    runtime_lifetime: BrowserRuntimeLifetime,
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
        effect::run_storage_runtime_cycle(
            &last_repo,
            current_repo_id,
            set_degraded_sync_mode,
            set_sync_banner,
            set_identity,
            set_repo_vector,
            runtime_lifetime.clone(),
        );
    });

    (identity, repo_vector)
}
