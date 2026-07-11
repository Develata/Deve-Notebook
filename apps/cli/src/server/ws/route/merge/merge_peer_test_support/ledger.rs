use crate::server::AppState;
use deve_core::ledger::merge::MergeResult;
use deve_core::models::{DocId, FactActor, LedgerEntry, MergeResolution, Op, PeerId, RepoType};
use std::sync::Arc;

pub(crate) fn seed_local_doc(state: &Arc<AppState>, path: &str) -> anyhow::Result<DocId> {
    let (doc_id, _ops) = state
        .repo
        .apply_file_structure_in_local_repo("notes", path, None, "test")?;
    Ok(doc_id)
}

pub(crate) fn seed_remote_insert(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    content: &str,
) -> anyhow::Result<()> {
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    Ok(())
}

pub(crate) fn seed_shared_base(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    content: &str,
) -> anyhow::Result<()> {
    state
        .repo
        .local_fact_writer(FactActor::new("merge_test")?)
        .append_content_in_local_repo(
            "notes",
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
        )?;
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
            peer_id.clone(),
            1,
            None,
            None,
        ),
    )?;
    let evaluation = state
        .repo
        .merge_peer_in_local_repo("notes", peer_id, &repo_id, doc_id)?;
    let MergeResult::Success(content) = evaluation.result else {
        anyhow::bail!("equal merge fixture unexpectedly produced a conflict");
    };
    state.repo.commit_peer_merge_in_local_repo(
        "notes",
        &evaluation.preflight,
        &content,
        MergeResolution::EstablishEqual,
    )?;
    Ok(())
}

pub(crate) fn seed_local_replace(
    state: &Arc<AppState>,
    doc_id: DocId,
    before: &str,
    after: &str,
) -> anyhow::Result<()> {
    let writer = state.repo.local_fact_writer(FactActor::new("merge_test")?);
    writer.append_content_in_local_repo(
        "notes",
        doc_id,
        Op::Delete {
            pos: 0,
            len: utf16_len(before),
        },
        2,
    )?;
    writer.append_content_in_local_repo(
        "notes",
        doc_id,
        Op::Insert {
            pos: 0,
            content: after.into(),
        },
        3,
    )?;
    Ok(())
}

pub(crate) fn seed_remote_replace(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    doc_id: DocId,
    before: &str,
    after: &str,
) -> anyhow::Result<()> {
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Delete {
                pos: 0,
                len: utf16_len(before),
            },
            2,
            peer_id.clone(),
            2,
            None,
            None,
        ),
    )?;
    state.repo.append_remote_op(
        peer_id,
        &repo_id,
        &LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: after.into(),
            },
            3,
            peer_id.clone(),
            3,
            None,
            None,
        ),
    )?;
    Ok(())
}

pub(crate) fn local_doc_content(
    state: &Arc<AppState>,
    doc_id: DocId,
) -> anyhow::Result<(usize, String)> {
    let entries = state
        .repo
        .get_local_ops_in_local_repo("notes", doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .filter(|entry| entry.content_op().is_some())
        .collect::<Vec<_>>();
    Ok((
        entries.len(),
        deve_core::state::reconstruct_content(&entries),
    ))
}

pub(crate) fn local_doc_entry_count(state: &Arc<AppState>, doc_id: DocId) -> anyhow::Result<usize> {
    Ok(state
        .repo
        .get_local_ops_in_local_repo("notes", doc_id)?
        .len())
}

pub(crate) fn doc_content(
    state: &Arc<AppState>,
    repo_type: RepoType,
    doc_id: DocId,
) -> anyhow::Result<(usize, String)> {
    let entries = state
        .repo
        .get_ops(&repo_type, doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .filter(|entry| entry.content_op().is_some())
        .collect::<Vec<_>>();
    Ok((
        entries.len(),
        deve_core::state::reconstruct_content(&entries),
    ))
}

pub(crate) fn doc_entry_count(
    state: &Arc<AppState>,
    repo_type: RepoType,
    doc_id: DocId,
) -> anyhow::Result<usize> {
    Ok(state.repo.get_ops(&repo_type, doc_id)?.len())
}

fn utf16_len(value: &str) -> u32 {
    value.encode_utf16().count() as u32
}
