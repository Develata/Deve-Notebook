//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Server runtime assembly parts.
//!
//! This module keeps startup orchestration out of handlers and narrows
//! `start_server_with_bound_listener` toward a composition root.

pub(crate) mod app_runtime;
pub(crate) mod auth_runtime;
pub(crate) mod host_runtime;
pub(crate) mod node_role_runtime;
pub(crate) mod peripheral_runtime;
pub(crate) mod router_runtime;
pub(crate) mod sync_runtime;
pub(crate) mod watcher_runtime;

pub(crate) use app_runtime::{
    AppStateParts, build_app_state, build_tree_registry, new_server_broadcast_channel,
};
pub(crate) use auth_runtime::init_auth_runtime;
pub(crate) use host_runtime::{
    install_repo_host_apis, install_sync_host_api, prepare_host_layout, refresh_host_port_hint,
};
pub(crate) use node_role_runtime::{current_repo_health, init_node_role, update_repo_health};
#[cfg(feature = "search")]
pub(crate) use peripheral_runtime::search_available;
pub(crate) use peripheral_runtime::{
    BackgroundRuntimeTasks, init_observability_runtime, spawn_background_runtime_tasks,
};
pub(crate) use router_runtime::build_runtime_router;
pub(crate) use sync_runtime::{build_sync_engine, init_sync_manager, load_identity_key};
pub(crate) use watcher_runtime::start_file_watchers;
