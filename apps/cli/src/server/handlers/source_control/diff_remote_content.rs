//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
//! Remote source-control diff content resolution.

use crate::server::AppState;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::DOCID_TO_PATH;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::ScPathTarget;
use deve_core::utils::path::to_forward_slash;
use std::sync::Arc;

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
            let Some(doc_id) = resolve_tracked_doc_id(db, target)? else {
                return Ok(None);
            };
            Ok(Some((doc_id, reconstruct_doc_content(db, doc_id)?)))
        })
}

pub(crate) fn local_counterpart_content(
    repo: &RepoManager,
    doc_id: DocId,
    repo_name: &str,
) -> anyhow::Result<Option<String>> {
    repo.run_on_local_repo(repo_name, |db| {
        if deve_core::ledger::node_meta::file_meta_for_doc(db, doc_id)?.is_none() {
            if let Some(path) = legacy_doc_path(db, doc_id)? {
                anyhow::bail!(
                    "Tracked document projection missing for legacy-mapped doc: {}",
                    path
                );
            }
            return Ok(None);
        }
        Ok(Some(reconstruct_doc_content(db, doc_id)?))
    })
}

fn reconstruct_doc_content(db: &redb::Database, doc_id: DocId) -> anyhow::Result<String> {
    let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
    let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
    Ok(deve_core::state::reconstruct_content(&entries))
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
) -> anyhow::Result<Option<DocId>> {
    if let Some(doc_id) = target.doc_id {
        let Some(meta) = deve_core::ledger::node_meta::file_meta_for_doc(db, doc_id)? else {
            return Ok(None);
        };
        let requested = to_forward_slash(&target.path);
        let canonical = to_forward_slash(&meta.path);
        if requested != canonical {
            anyhow::bail!(
                "Remote document target path mismatch: requested {}, doc {} is at {}",
                requested,
                doc_id,
                canonical
            );
        }
        return Ok(Some(doc_id));
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
