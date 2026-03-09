use super::PluginHostState;
use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

use crate::server::handlers::{repo, source_control};
use crate::server::node_role_http;

pub(super) fn build_router(state: Arc<PluginHostState>) -> Router {
    Router::new()
        .route("/ws", get(super::ws_handler))
        .route("/api/repo/docs", get(repo::http::list_docs_plugin_host))
        .route("/api/repo/doc", get(repo::http::doc_content_plugin_host))
        .route(
            "/api/sc/pending",
            get(source_control::http::pending_plugin_host),
        )
        .route(
            "/api/sc/status",
            get(source_control::http::status_plugin_host),
        )
        .route("/api/sc/diff", get(source_control::http::diff_plugin_host))
        .route(
            "/api/sc/commits",
            get(source_control::http_commits::commit_history_plugin_host),
        )
        .route(
            "/api/sc/commit-diff",
            get(source_control::http_commits::commit_diff_plugin_host),
        )
        .route(
            "/api/sc/stage",
            post(source_control::http_mutations::stage_plugin_host),
        )
        .route(
            "/api/sc/stage-pending",
            post(source_control::http_mutations::stage_plugin_host),
        )
        .route(
            "/api/sc/unstage",
            post(source_control::http_mutations::unstage_plugin_host),
        )
        .route(
            "/api/sc/discard-pending",
            post(source_control::http_mutations::discard_pending_plugin_host),
        )
        .route(
            "/api/sc/commit",
            post(source_control::http_mutations::commit_plugin_host),
        )
        .route("/api/node/role", get(node_role_http::role))
        .with_state(state)
}
