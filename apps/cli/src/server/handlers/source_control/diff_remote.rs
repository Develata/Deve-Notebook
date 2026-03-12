use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::source_control::errors;
use crate::server::repo_scope::resolve_session_repo_and_sync;
use crate::server::session::WsSession;
use deve_core::ledger::RepoManager;
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub(super) async fn handle_remote_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    target: ScPathTarget,
) {
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(e) => return errors::send_ws(ch, errors::map_repo_scope_error(e)),
    };
    let path = deve_core::utils::path::to_forward_slash(&target.path);
    let new_content = match get_remote_doc_content(session, &target) {
        Some(content) => content,
        None => {
            return errors::send_ws_code(
                ch,
                ServerErrorCode::ScDocNotFound,
                format!("Remote document not found: {}", path),
            );
        }
    };

    let remote_url = state
        .repo
        .get_repo_url(session.active_branch.as_ref(), &scope.repo_name)
        .ok()
        .flatten();
    let local_repo_name = session
        .active_repo_id
        .and_then(|repo_id| {
            state
                .repo
                .find_local_repo_name_by_id(repo_id)
                .ok()
                .flatten()
        })
        .or_else(|| {
            remote_url.and_then(|url| state.repo.find_local_repo_name_by_url(&url).ok().flatten())
        });
    let old_content =
        local_counterpart_content(state.repo.as_ref(), &target, local_repo_name.as_deref());

    ch.unicast(ServerMessage::DocDiff {
        repo_id: Some(scope.repo_id),
        path,
        old_content,
        new_content,
    });
}

fn get_remote_doc_content(session: &WsSession, target: &ScPathTarget) -> Option<String> {
    let db = session.get_active_db()?;
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
) -> String {
    repo_name
        .and_then(|name| {
            repo.run_on_local_repo(name, |db| {
                let Some(doc_id) = target.doc_id.or_else(|| resolve_doc_id(db, &target.path))
                else {
                    return Ok(None);
                };
                let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
                let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
                Ok(Some(deve_core::state::reconstruct_content(&entries)))
            })
            .ok()
            .flatten()
        })
        .unwrap_or_default()
}

fn resolve_doc_id(db: &redb::Database, path: &str) -> Option<deve_core::models::DocId> {
    deve_core::ledger::doc_lookup::resolve_doc_id(db, path)
        .ok()
        .flatten()
}
