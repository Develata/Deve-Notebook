use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::source_control::errors;
use crate::server::handlers::source_control::repo_scope;
use crate::server::repo_scope::resolve_local_counterpart_repo;
use crate::server::session::WsSession;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::DOCID_TO_PATH;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub(super) async fn handle_remote_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    target: ScPathTarget,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(e) => return errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let path = deve_core::utils::path::to_forward_slash(&target.path);
    let (doc_id, new_content) =
        match resolve_remote_content(state, scope.branch.as_ref(), scope.repo_id, &target) {
            Ok(Some(content)) => content,
            Ok(None) => {
                return errors::send_ws_code_scoped(
                    ch,
                    ServerErrorCode::ScDocNotFound,
                    format!("Remote document not found: {}", path),
                    scope_nonce,
                );
            }
            Err(err) => {
                return errors::send_ws_scoped(
                    ch,
                    errors::map_repo_error(errors::ScOp::DiffDoc(path.clone()), err),
                    scope_nonce,
                );
            }
        };

    let local_repo_name = match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => Some(local_scope.repo_name),
        Ok(None) => None,
        Err(err) => {
            return errors::send_ws_scoped(ch, errors::map_repo_scope_error(err), scope_nonce);
        }
    };
    let old_content =
        match local_counterpart_content(state.repo.as_ref(), doc_id, local_repo_name.as_deref()) {
            Ok(Some(content)) => content,
            Ok(None) => String::new(),
            Err(err) => {
                return errors::send_ws_scoped(
                    ch,
                    errors::map_repo_error(errors::ScOp::DiffDoc(path.clone()), err),
                    scope_nonce,
                );
            }
        };

    ch.unicast(ServerMessage::DocDiff {
        request_id: Some(request_id),
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        path,
        old_content,
        new_content,
    });
}

pub(super) fn resolve_remote_content(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_id: RepoId,
    target: &ScPathTarget,
) -> anyhow::Result<Option<(DocId, String)>> {
    let Some(peer_id) = branch else {
        return Ok(None);
    };
    state
        .repo
        .run_on_shadow_repo_by_id(peer_id, &repo_id, |db| {
            let doc_id = resolve_tracked_doc_id(db, target)?;
            let Some(doc_id) = doc_id else {
                return Ok(None);
            };
            let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            Ok(Some((
                doc_id,
                deve_core::state::reconstruct_content(&entries),
            )))
        })
}

pub(crate) fn local_counterpart_content(
    repo: &RepoManager,
    doc_id: DocId,
    repo_name: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let Some(name) = repo_name else {
        return Ok(None);
    };
    repo.run_on_local_repo(name, |db| {
        if deve_core::ledger::node_meta::file_meta_for_doc(db, doc_id)?.is_none() {
            if let Some(path) = legacy_doc_path(db, doc_id)? {
                anyhow::bail!(
                    "Tracked document projection missing for legacy-mapped doc: {}",
                    path
                );
            }
            return Ok(None);
        }
        let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
        let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
        Ok(Some(deve_core::state::reconstruct_content(&entries)))
    })
}

fn legacy_doc_path(db: &redb::Database, doc_id: DocId) -> anyhow::Result<Option<String>> {
    let read = db.begin_read()?;
    let table = read.open_table(DOCID_TO_PATH)?;
    Ok(table
        .get(doc_id.as_u128())?
        .map(|path| path.value().to_string()))
}

pub(super) fn resolve_tracked_doc_id(
    db: &redb::Database,
    target: &ScPathTarget,
) -> anyhow::Result<Option<deve_core::models::DocId>> {
    if let Some(doc_id) = target.doc_id {
        return Ok(deve_core::ledger::node_meta::file_meta_for_doc(db, doc_id)?.map(|_| doc_id));
    }
    if let Some(node_id) = deve_core::ledger::node_meta::get_node_id(db, &target.path)? {
        return Ok(
            deve_core::ledger::node_meta::get_node_meta(db, node_id)?.and_then(|meta| meta.doc_id)
        );
    }
    if deve_core::ledger::metadata::get_docid(db, &target.path)?.is_some() {
        anyhow::bail!(
            "Tracked document projection missing for legacy-mapped path: {}",
            target.path
        );
    }
    Ok(None)
}
