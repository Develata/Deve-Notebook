use crate::models::{
    DocId, LedgerEntry, LedgerEvent, NodeId, NodeKind, NodeMeta, StructureOp,
    deserialize_ledger_entry,
};
use crate::state::{InvalidContentOp, describe_invalid_content_op, find_invalid_content_op};
use anyhow::{Result, anyhow};
use redb::{ReadableMultimapTable, ReadableTable, WriteTransaction};
use std::collections::HashSet;

use super::schema::{DOC_OPS, LEDGER_OPS, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID};

pub(crate) fn validate_ledger_append(
    write_txn: &WriteTransaction,
    entry: &LedgerEntry,
    repo_scope: &str,
) -> Result<()> {
    match &entry.event {
        LedgerEvent::Content(_) => validate_content_append(write_txn, entry, repo_scope),
        LedgerEvent::Structure(op) => validate_structure_append(write_txn, op, repo_scope),
    }
}

fn validate_content_append(
    write_txn: &WriteTransaction,
    entry: &LedgerEntry,
    repo_scope: &str,
) -> Result<()> {
    let Some(doc_id) = entry.doc_id else {
        return Err(reject_missing_doc_id(entry, repo_scope));
    };
    let mut ops = load_doc_entries(write_txn, doc_id)?;
    if let Some(issue) = find_invalid_content_op(&ops) {
        return Err(reject_invalid_content(
            doc_id, entry, &issue, true, repo_scope,
        ));
    }
    ops.push(entry.clone());
    if let Some(issue) = find_invalid_content_op(&ops) {
        return Err(reject_invalid_content(
            doc_id, entry, &issue, false, repo_scope,
        ));
    }
    Ok(())
}

fn validate_structure_append(
    write_txn: &WriteTransaction,
    op: &StructureOp,
    repo_scope: &str,
) -> Result<()> {
    validate_structure_state(write_txn, op)
        .map_err(|err| reject_invalid_structure(op, &err.to_string(), repo_scope))
}

fn validate_structure_state(write_txn: &WriteTransaction, op: &StructureOp) -> Result<()> {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => {
            ensure_name_segment(name)?;
            if *node_id != NodeId::from_doc_id(*doc_id) {
                anyhow::bail!("CreateFile node/doc mismatch for {}", doc_id);
            }
            ensure_node_absent(write_txn, *node_id)?;
            ensure_parent_dir(write_txn, *parent_id)?;
            ensure_path_free(write_txn, &child_path(write_txn, *parent_id, name)?)
        }
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => {
            ensure_name_segment(name)?;
            ensure_node_absent(write_txn, *node_id)?;
            ensure_parent_dir(write_txn, *parent_id)?;
            ensure_path_free(write_txn, &child_path(write_txn, *parent_id, name)?)
        }
        StructureOp::RenameNode {
            node_id,
            doc_id,
            new_name,
        } => {
            ensure_name_segment(new_name)?;
            let meta = load_meta_required(write_txn, *node_id)?;
            ensure_doc_match(*node_id, meta.doc_id, *doc_id)?;
            let new_path = child_path(write_txn, meta.parent_id, new_name)?;
            ensure_rename_target_free(write_txn, &meta, new_path)
        }
        StructureOp::MoveNode {
            node_id,
            doc_id,
            new_parent_id,
        } => {
            let meta = load_meta_required(write_txn, *node_id)?;
            ensure_doc_match(*node_id, meta.doc_id, *doc_id)?;
            ensure_parent_dir(write_txn, *new_parent_id)?;
            ensure_not_descendant(write_txn, *node_id, *new_parent_id)?;
            let new_path = child_path(write_txn, *new_parent_id, &meta.name)?;
            ensure_rename_target_free(write_txn, &meta, new_path)
        }
        StructureOp::DeleteNode { node_id, doc_id } => {
            if let Some(meta) = load_meta(write_txn, *node_id)? {
                ensure_doc_match(*node_id, meta.doc_id, *doc_id)?;
            }
            Ok(())
        }
    }
}

fn load_doc_entries(write_txn: &WriteTransaction, doc_id: DocId) -> Result<Vec<LedgerEntry>> {
    let doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
    let ops = write_txn.open_table(LEDGER_OPS)?;
    let mut entries = Vec::new();
    for seq in doc_ops.get(doc_id.as_u128())? {
        let global_seq = seq?.value();
        let Some(bytes) = ops.get(global_seq)? else {
            return Err(anyhow!(
                "Broken DOC_OPS index for {}: missing ledger op at seq {}",
                doc_id,
                global_seq
            ));
        };
        entries.push((global_seq, deserialize_ledger_entry(bytes.value())?));
    }
    entries.sort_by_key(|(global_seq, _)| *global_seq);
    Ok(entries.into_iter().map(|(_, entry)| entry).collect())
}

fn child_path(
    write_txn: &WriteTransaction,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<String> {
    if let Some(parent_id) = parent_id {
        let parent = load_meta_required(write_txn, parent_id)?;
        return Ok(format!("{}/{}", parent.path, name));
    }
    Ok(name.to_string())
}

fn load_meta(write_txn: &WriteTransaction, node_id: NodeId) -> Result<Option<NodeMeta>> {
    let table = write_txn.open_table(NODEID_TO_META)?;
    table
        .get(node_id.as_u128())?
        .map(|bytes| bincode::deserialize(bytes.value()).map_err(Into::into))
        .transpose()
}

fn load_meta_required(write_txn: &WriteTransaction, node_id: NodeId) -> Result<NodeMeta> {
    load_meta(write_txn, node_id)?
        .ok_or_else(|| anyhow!("structure projection missing node {}", node_id))
}

fn ensure_parent_dir(write_txn: &WriteTransaction, parent_id: Option<NodeId>) -> Result<()> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };
    let parent = load_meta_required(write_txn, parent_id)?;
    if parent.kind != NodeKind::Dir {
        anyhow::bail!("structure parent is not a directory: {}", parent_id);
    }
    Ok(())
}

fn ensure_node_absent(write_txn: &WriteTransaction, node_id: NodeId) -> Result<()> {
    if load_meta(write_txn, node_id)?.is_some() {
        anyhow::bail!("structure node already exists: {}", node_id);
    }
    Ok(())
}

fn ensure_path_free(write_txn: &WriteTransaction, path: &str) -> Result<()> {
    if path_node(write_txn, path)?.is_some() {
        anyhow::bail!("structure path already bound to a node: {}", path);
    }
    if path_doc(write_txn, path)?.is_some() {
        anyhow::bail!("structure path already bound to a document: {}", path);
    }
    Ok(())
}

fn ensure_rename_target_free(
    write_txn: &WriteTransaction,
    meta: &NodeMeta,
    new_path: String,
) -> Result<()> {
    if meta.path == new_path {
        return Ok(());
    }
    if meta.kind == NodeKind::File {
        ensure_exact_target_free(write_txn, meta, &new_path)
    } else {
        ensure_prefix_target_free(write_txn, &meta.path, &new_path)
    }
}

fn ensure_exact_target_free(
    write_txn: &WriteTransaction,
    meta: &NodeMeta,
    new_path: &str,
) -> Result<()> {
    if let Some(node_id) = path_node(write_txn, new_path)? {
        anyhow::bail!("structure target path already bound to node {}", node_id);
    }
    if let Some(doc_id) = path_doc(write_txn, new_path)?
        && Some(doc_id) != meta.doc_id
    {
        anyhow::bail!("structure target path already bound to document {}", doc_id);
    }
    Ok(())
}

fn ensure_prefix_target_free(
    write_txn: &WriteTransaction,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<()> {
    ensure_node_prefix_target_free(write_txn, old_prefix, new_prefix)?;
    ensure_doc_prefix_target_free(write_txn, old_prefix, new_prefix)
}

fn ensure_node_prefix_target_free(
    write_txn: &WriteTransaction,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<()> {
    let table = write_txn.open_table(PATH_TO_NODEID)?;
    for item in table.iter()? {
        let (path_guard, _) = item?;
        let path = path_guard.value();
        if is_under(path, new_prefix) && !is_under(path, old_prefix) {
            anyhow::bail!("structure target path already bound to node: {}", path);
        }
    }
    Ok(())
}

fn ensure_doc_prefix_target_free(
    write_txn: &WriteTransaction,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<()> {
    let table = write_txn.open_table(PATH_TO_DOCID)?;
    for item in table.iter()? {
        let (path_guard, _) = item?;
        let path = path_guard.value();
        if is_under(path, new_prefix) && !is_under(path, old_prefix) {
            anyhow::bail!("structure target path already bound to document: {}", path);
        }
    }
    Ok(())
}

fn path_node(write_txn: &WriteTransaction, path: &str) -> Result<Option<NodeId>> {
    let table = write_txn.open_table(PATH_TO_NODEID)?;
    Ok(table.get(path)?.map(|v| NodeId::from_u128(v.value())))
}

fn path_doc(write_txn: &WriteTransaction, path: &str) -> Result<Option<DocId>> {
    let table = write_txn.open_table(PATH_TO_DOCID)?;
    Ok(table.get(path)?.map(|v| DocId::from_u128(v.value())))
}

fn is_under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{}/", prefix))
}

fn ensure_not_descendant(
    write_txn: &WriteTransaction,
    node_id: NodeId,
    new_parent_id: Option<NodeId>,
) -> Result<()> {
    let mut cursor = new_parent_id;
    let mut visiting = HashSet::new();
    while let Some(parent_id) = cursor {
        if parent_id == node_id {
            anyhow::bail!("structure move would create cycle at node {}", node_id);
        }
        if !visiting.insert(parent_id) {
            anyhow::bail!("structure projection contains cycle at node {}", parent_id);
        }
        cursor = load_meta_required(write_txn, parent_id)?.parent_id;
    }
    Ok(())
}

fn ensure_doc_match(node_id: NodeId, actual: Option<DocId>, expected: Option<DocId>) -> Result<()> {
    if actual != expected {
        anyhow::bail!(
            "structure doc mismatch for {}: actual={:?}, expected={:?}",
            node_id,
            actual,
            expected
        );
    }
    Ok(())
}

fn ensure_name_segment(name: &str) -> Result<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        anyhow::bail!("invalid structure name segment: {}", name);
    }
    Ok(())
}

fn reject_missing_doc_id(entry: &LedgerEntry, repo_scope: &str) -> anyhow::Error {
    tracing::warn!(repo_scope, doc_id = "<missing>", peer_id = %entry.peer_id, seq = entry.seq, issue = "content op missing doc id", "Rejecting invalid ledger append");
    anyhow!("Content op missing doc id")
}

fn reject_invalid_content(
    doc_id: DocId,
    entry: &LedgerEntry,
    issue: &InvalidContentOp,
    existing_history_invalid: bool,
    repo_scope: &str,
) -> anyhow::Error {
    let issue_text = describe_invalid_content_op(issue);
    tracing::warn!(repo_scope, doc_id = %doc_id, peer_id = %entry.peer_id, seq = entry.seq, issue = %issue_text, existing_history_invalid, "Rejecting invalid ledger append");
    if existing_history_invalid {
        anyhow!(
            "Refusing to append content op for {}: existing history invalid: {}",
            doc_id,
            issue_text
        )
    } else {
        anyhow!(
            "Refusing to append content op for {}: {}",
            doc_id,
            issue_text
        )
    }
}

fn reject_invalid_structure(op: &StructureOp, issue: &str, repo_scope: &str) -> anyhow::Error {
    tracing::warn!(repo_scope, node_id = %op.node_id(), doc_id = ?op.doc_id(), issue, "Rejecting invalid structure append");
    anyhow!(
        "Refusing to append structure op for {}: {}",
        op.node_id(),
        issue
    )
}
