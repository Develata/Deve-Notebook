use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::source_control::errors;
use crate::server::repo_scope::resolve_session_repo;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub(super) async fn handle_remote_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return errors::send_ws(ch, errors::map_repo_scope_error(e)),
    };
    let new_content = match get_remote_doc_content(session, &path) {
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
    let local_repo_name =
        remote_url.and_then(|url| state.repo.find_local_repo_name_by_url(&url).ok().flatten());
    let old_content = get_local_counterpart(state, &path, local_repo_name);

    ch.unicast(ServerMessage::DocDiff {
        path,
        old_content,
        new_content,
    });
}

fn get_remote_doc_content(session: &WsSession, path: &str) -> Option<String> {
    let db = session.get_active_db()?;
    let doc_id = deve_core::ledger::metadata::get_docid(&db.db, path).ok()??;
    let ops = deve_core::ledger::ops::get_ops_from_db(&db.db, doc_id).ok()?;
    let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
    Some(deve_core::state::reconstruct_content(&entries))
}

fn get_local_counterpart(state: &Arc<AppState>, path: &str, repo_name: Option<String>) -> String {
    repo_name
        .and_then(|name| {
            state
                .repo
                .run_on_local_repo(&name, |db| {
                    let Some(doc_id) = deve_core::ledger::metadata::get_docid(db, path)? else {
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
