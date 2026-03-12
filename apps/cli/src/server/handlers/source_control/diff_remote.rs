use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::source_control::errors;
use crate::server::repo_scope::{resolve_local_counterpart_repo, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
use deve_core::ledger::RepoManager;
use deve_core::models::RepoId;
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub(super) async fn handle_remote_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    target: ScPathTarget,
) {
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(e) => return errors::send_ws(ch, errors::map_repo_scope_error(e)),
    };
    let path = deve_core::utils::path::to_forward_slash(&target.path);
    let new_content =
        match get_remote_doc_content(session, &scope.repo_name, scope.repo_id, &target) {
            Some(content) => content,
            None => {
                return errors::send_ws_code(
                    ch,
                    ServerErrorCode::ScDocNotFound,
                    format!("Remote document not found: {}", path),
                );
            }
        };

    let local_repo_name = match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => Some(local_scope.repo_name),
        Ok(None) => None,
        Err(err) => return errors::send_ws(ch, errors::map_repo_scope_error(err)),
    };
    let old_content =
        match local_counterpart_content(state.repo.as_ref(), &target, local_repo_name.as_deref()) {
            Ok(Some(content)) => content,
            Ok(None) => String::new(),
            Err(err) => {
                return errors::send_ws(
                    ch,
                    errors::map_repo_error(errors::ScOp::DiffDoc(path.clone()), err),
                );
            }
        };

    ch.unicast(ServerMessage::DocDiff {
        request_id: Some(request_id),
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        path,
        old_content,
        new_content,
    });
}

fn get_remote_doc_content(
    session: &WsSession,
    repo_name: &str,
    repo_id: RepoId,
    target: &ScPathTarget,
) -> Option<String> {
    let db = session.active_db_for(session.active_branch.as_ref(), repo_name, Some(repo_id))?;
    let doc_id = target
        .doc_id
        .or_else(|| resolve_doc_id(&db.db, &target.path))?;
    let ops = deve_core::ledger::ops::get_ops_from_db(&db.db, doc_id).ok()?;
    let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
    Some(deve_core::state::reconstruct_content(&entries))
}

pub(crate) fn local_counterpart_content(
    repo: &RepoManager,
    target: &ScPathTarget,
    repo_name: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let Some(name) = repo_name else {
        return Ok(None);
    };
    repo.run_on_local_repo(name, |db| {
        let Some(doc_id) = target.doc_id.or_else(|| resolve_doc_id(db, &target.path)) else {
            return Ok(None);
        };
        let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
        let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
        Ok(Some(deve_core::state::reconstruct_content(&entries)))
    })
}

fn resolve_doc_id(db: &redb::Database, path: &str) -> Option<deve_core::models::DocId> {
    deve_core::ledger::doc_lookup::resolve_doc_id(db, path)
        .ok()
        .flatten()
}
