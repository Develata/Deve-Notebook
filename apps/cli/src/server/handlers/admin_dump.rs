use crate::admin_api::DumpResponse;
use crate::server::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

use super::DumpQuery;

pub async fn dump(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DumpQuery>,
) -> impl IntoResponse {
    let repo_name = match state
        .repo
        .resolve_local_repo_name(query.repo.repo_id, query.repo.repo_name.as_deref())
    {
        Ok(name) => name,
        Err(err) => return (StatusCode::BAD_REQUEST, err.to_string()).into_response(),
    };
    match build_dump(state.as_ref(), &repo_name, &query.path) {
        Ok(Some(dump)) => Json(dump).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Path not found in Ledger.").into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

fn build_dump(
    state: &AppState,
    repo_name: &str,
    path: &str,
) -> anyhow::Result<Option<DumpResponse>> {
    let path = deve_core::utils::path::to_forward_slash(path);
    let (node_id, node_meta, doc_id) = state.repo.run_on_local_repo(repo_name, |db| {
        let node_id = deve_core::ledger::node_meta::get_node_id(db, &path)?;
        let node_meta = match node_id {
            Some(id) => deve_core::ledger::node_meta::get_node_meta(db, id)?,
            None => None,
        };
        let doc_id = node_meta.as_ref().and_then(|meta| meta.doc_id).or_else(|| {
            deve_core::ledger::doc_lookup::resolve_doc_id(db, &path)
                .ok()
                .flatten()
        });
        Ok((node_id, node_meta, doc_id))
    })?;
    if node_id.is_none() && doc_id.is_none() {
        return Ok(None);
    }

    let structure_ops = match node_id {
        Some(node_id) => state
            .repo
            .get_local_structure_ops_in_local_repo(repo_name, node_id)?,
        None => Vec::new(),
    };
    let ops = match doc_id {
        Some(doc_id) => state.repo.get_local_ops_in_local_repo(repo_name, doc_id)?,
        None => Vec::new(),
    };
    let content = deve_core::state::reconstruct_content(
        &ops.iter()
            .map(|(_, entry)| entry.clone())
            .collect::<Vec<_>>(),
    );

    Ok(Some(DumpResponse {
        doc_id,
        node_id,
        node_meta,
        ops,
        structure_ops,
        content,
    }))
}
