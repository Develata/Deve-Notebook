//! plan_ref:
//!   - 03_storage/repair#backup-export
//!   - 03_storage/authority#redb-schema-version-contract
//!
//! Explicit, offline-only schema-v2 export. This module never opens a write transaction.

use super::doc;
use anyhow::{Context, Result, anyhow, bail};
use deve_core::codec;
use deve_core::ledger::schema::{LEDGER_OPS, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY};
use deve_core::models::{DocId, LedgerEvent, NodeId, Op, PeerId, StructureOp};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Component, Path, PathBuf};

const LEGACY_SCHEMA_VERSION: u16 = 2;
const LEGACY_ENTRY_MAGIC: &[u8; 8] = b"DEVELDG2";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyLedgerEntry {
    doc_id: Option<DocId>,
    event: LedgerEvent,
    timestamp: i64,
    peer_id: PeerId,
    seq: u64,
    client_id: Option<u64>,
    client_op_id: Option<u64>,
}

#[derive(Serialize, Deserialize)]
struct LegacyEntryEnvelope {
    format_version: u16,
    entry: LegacyLedgerEntry,
}

#[derive(Serialize)]
struct LegacyJsonExportEntry<'a> {
    legacy_schema_version: u16,
    global_seq: u64,
    entry: &'a LegacyLedgerEntry,
}

#[derive(Clone)]
struct LegacyNode {
    parent_id: Option<NodeId>,
    name: String,
    doc_id: Option<DocId>,
    deleted: bool,
}

pub(super) fn run(
    ledger_dir: &Path,
    output: Option<String>,
    repo_name: Option<String>,
    doc_id: Option<String>,
    format: &str,
) -> Result<()> {
    let db_path = resolve_legacy_repo_path(ledger_dir, repo_name.as_deref())?;
    let db = redb::Database::open(&db_path).with_context(|| {
        format!(
            "legacy v2 export requires exclusive offline DB access: {}",
            db_path.display()
        )
    })?;
    verify_schema_v2(&db)?;
    let entries = read_entries(&db)?;
    match format {
        "json" => {
            if doc_id.is_some() {
                bail!("Legacy v2 JSON export does not support --doc");
            }
            write_json(output, &entries)
        }
        "markdown" | "md" => write_markdown(output, doc_id, &entries),
        _ => bail!("Unsupported export format: {format}. Use 'json' or 'markdown'."),
    }
}

fn resolve_legacy_repo_path(ledger_dir: &Path, repo_name: Option<&str>) -> Result<PathBuf> {
    let local_dir = ledger_dir.join("local");
    if let Some(repo_name) = repo_name {
        if repo_name.is_empty()
            || Path::new(repo_name)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            bail!("Invalid legacy repo execution name: {repo_name}");
        }
        let path = local_dir.join(format!("{repo_name}.redb"));
        if !path.is_file() {
            bail!("Legacy repo database not found: {}", path.display());
        }
        return Ok(path);
    }

    let mut candidates = std::fs::read_dir(&local_dir)
        .with_context(|| {
            format!(
                "Failed to list legacy repo directory {}",
                local_dir.display()
            )
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "redb"))
        .collect::<Vec<_>>();
    candidates.sort();
    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => bail!(
            "No legacy repo database found under {}",
            local_dir.display()
        ),
        _ => bail!(
            "Multiple legacy repos found under {}; specify --repo",
            local_dir.display()
        ),
    }
}

fn verify_schema_v2(db: &redb::Database) -> Result<()> {
    let read = db.begin_read()?;
    let metadata = read
        .open_table(REPO_METADATA)
        .context("legacy repo metadata table missing")?;
    let raw = metadata
        .get(&REPO_SCHEMA_VERSION_METADATA_KEY)?
        .ok_or_else(|| anyhow!("legacy repo schema version metadata missing"))?;
    let version: u16 = codec::decode(raw.value()).context("invalid legacy schema version")?;
    if version != LEGACY_SCHEMA_VERSION {
        bail!("--allow-legacy-v2 only accepts schema v2; database reports schema v{version}");
    }
    Ok(())
}

fn read_entries(db: &redb::Database) -> Result<Vec<(u64, LegacyLedgerEntry)>> {
    let read = db.begin_read()?;
    let table = read
        .open_table(LEDGER_OPS)
        .context("legacy ledger_ops table missing")?;
    let mut entries = Vec::new();
    for item in table.iter()? {
        let (global_seq, bytes) = item?;
        let payload = bytes
            .value()
            .strip_prefix(LEGACY_ENTRY_MAGIC)
            .ok_or_else(|| {
                anyhow!(
                    "legacy entry {} is missing DEVELDG2 magic",
                    global_seq.value()
                )
            })?;
        let envelope: LegacyEntryEnvelope = codec::decode(payload)
            .with_context(|| format!("failed to decode legacy entry {}", global_seq.value()))?;
        if envelope.format_version != LEGACY_SCHEMA_VERSION {
            bail!(
                "legacy entry {} has format {}, expected 2",
                global_seq.value(),
                envelope.format_version
            );
        }
        entries.push((global_seq.value(), envelope.entry));
    }
    Ok(entries)
}

fn write_json(output: Option<String>, entries: &[(u64, LegacyLedgerEntry)]) -> Result<()> {
    let mut writer: Box<dyn Write> = match output {
        Some(path) => Box::new(BufWriter::new(File::create(path)?)),
        None => Box::new(std::io::stdout()),
    };
    for (global_seq, entry) in entries {
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&LegacyJsonExportEntry {
                legacy_schema_version: LEGACY_SCHEMA_VERSION,
                global_seq: *global_seq,
                entry,
            })?
        )?;
    }
    Ok(())
}

fn write_markdown(
    output: Option<String>,
    selected_doc: Option<String>,
    entries: &[(u64, LegacyLedgerEntry)],
) -> Result<()> {
    let mut nodes = HashMap::new();
    let mut content_ops: HashMap<DocId, Vec<Op>> = HashMap::new();
    for (_global_seq, entry) in entries {
        match &entry.event {
            LedgerEvent::Content(op) => {
                let doc_id = entry
                    .doc_id
                    .ok_or_else(|| anyhow!("legacy content fact is missing doc_id"))?;
                content_ops.entry(doc_id).or_default().push(op.clone());
            }
            LedgerEvent::Structure(op) => apply_structure(&mut nodes, op)?,
            LedgerEvent::MergeAnchor(_) => {}
        }
    }

    if let Some(selected_doc) = selected_doc {
        let doc_id = doc::parse_doc_id(&selected_doc)?;
        let ops = content_ops.get(&doc_id).map(Vec::as_slice).unwrap_or(&[]);
        let content = deve_core::state::try_apply_content_ops("", ops)
            .ok_or_else(|| anyhow!("legacy content facts for {doc_id} are invalid"))?;
        return doc::write_markdown_file(&doc::output_file(output)?, &content);
    }

    let output_dir = PathBuf::from(output.unwrap_or_else(|| "export-v2".into()));
    let mut exported = 0_u32;
    let mut seen_paths = HashSet::new();
    for (node_id, node) in &nodes {
        let Some(doc_id) = node.doc_id else {
            continue;
        };
        let Some(path) = node_path(*node_id, &nodes, &mut HashSet::new())? else {
            continue;
        };
        if !seen_paths.insert(path.clone()) {
            bail!("legacy structure facts resolve multiple documents to {path}");
        }
        let ops = content_ops.get(&doc_id).map(Vec::as_slice).unwrap_or(&[]);
        let content = deve_core::state::try_apply_content_ops("", ops)
            .ok_or_else(|| anyhow!("legacy content facts for {doc_id} are invalid"))?;
        doc::write_markdown_file(&output_dir.join(&path), &content)?;
        exported += 1;
    }
    println!(
        "Exported {exported} markdown files from schema v2 to {:?}",
        output_dir
    );
    Ok(())
}

fn apply_structure(nodes: &mut HashMap<NodeId, LegacyNode>, op: &StructureOp) -> Result<()> {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => {
            nodes.insert(
                *node_id,
                LegacyNode {
                    parent_id: *parent_id,
                    name: validate_name(name)?,
                    doc_id: Some(*doc_id),
                    deleted: false,
                },
            );
        }
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => {
            nodes.insert(
                *node_id,
                LegacyNode {
                    parent_id: *parent_id,
                    name: validate_name(name)?,
                    doc_id: None,
                    deleted: false,
                },
            );
        }
        StructureOp::RenameNode {
            node_id, new_name, ..
        } => {
            nodes
                .get_mut(node_id)
                .ok_or_else(|| anyhow!("legacy rename references missing node {node_id}"))?
                .name = validate_name(new_name)?;
        }
        StructureOp::MoveNode {
            node_id,
            new_parent_id,
            ..
        } => {
            nodes
                .get_mut(node_id)
                .ok_or_else(|| anyhow!("legacy move references missing node {node_id}"))?
                .parent_id = *new_parent_id;
        }
        StructureOp::DeleteNode { node_id, .. } => {
            nodes
                .get_mut(node_id)
                .ok_or_else(|| anyhow!("legacy delete references missing node {node_id}"))?
                .deleted = true;
        }
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<String> {
    if name.is_empty()
        || Path::new(name)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("unsafe legacy structure name: {name:?}");
    }
    Ok(name.to_string())
}

fn node_path(
    node_id: NodeId,
    nodes: &HashMap<NodeId, LegacyNode>,
    visiting: &mut HashSet<NodeId>,
) -> Result<Option<String>> {
    if !visiting.insert(node_id) {
        bail!("legacy structure cycle detected at node {node_id}");
    }
    let node = nodes
        .get(&node_id)
        .ok_or_else(|| anyhow!("legacy structure references missing node {node_id}"))?;
    if node.deleted {
        visiting.remove(&node_id);
        return Ok(None);
    }
    let path = if let Some(parent_id) = node.parent_id {
        let Some(parent) = node_path(parent_id, nodes, visiting)? else {
            visiting.remove(&node_id);
            return Ok(None);
        };
        format!("{parent}/{}", node.name)
    } else {
        node.name.clone()
    };
    visiting.remove(&node_id);
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::ledger::schema::{LEDGER_OPS, REPO_METADATA};

    fn write_v2_fixture(ledger_dir: &Path) -> Result<(DocId, PathBuf)> {
        let local_dir = ledger_dir.join("local");
        std::fs::create_dir_all(&local_dir)?;
        let db_path = local_dir.join("default.redb");
        let db = redb::Database::create(&db_path)?;
        let doc_id = DocId::new();
        let dir_id = NodeId::new();
        let file_id = NodeId::from_doc_id(doc_id);
        let peer = PeerId::new("legacy-label");
        let entries = [
            LegacyLedgerEntry {
                doc_id: None,
                event: LedgerEvent::Structure(StructureOp::CreateDir {
                    node_id: dir_id,
                    parent_id: None,
                    name: "notes".into(),
                }),
                timestamp: 1,
                peer_id: peer.clone(),
                seq: 1,
                client_id: None,
                client_op_id: None,
            },
            LegacyLedgerEntry {
                doc_id: Some(doc_id),
                event: LedgerEvent::Structure(StructureOp::CreateFile {
                    node_id: file_id,
                    doc_id,
                    parent_id: Some(dir_id),
                    name: "legacy.md".into(),
                }),
                timestamp: 2,
                peer_id: peer.clone(),
                seq: 1,
                client_id: None,
                client_op_id: None,
            },
            LegacyLedgerEntry {
                doc_id: Some(doc_id),
                event: LedgerEvent::Content(Op::Insert {
                    pos: 0,
                    content: "legacy content".into(),
                }),
                timestamp: 3,
                peer_id: peer,
                seq: 1,
                client_id: None,
                client_op_id: None,
            },
        ];
        let write = db.begin_write()?;
        {
            let mut metadata = write.open_table(REPO_METADATA)?;
            metadata.insert(
                &REPO_SCHEMA_VERSION_METADATA_KEY,
                codec::encode(&LEGACY_SCHEMA_VERSION)?.as_slice(),
            )?;
            let mut ledger = write.open_table(LEDGER_OPS)?;
            for (index, entry) in entries.iter().enumerate() {
                let envelope = LegacyEntryEnvelope {
                    format_version: LEGACY_SCHEMA_VERSION,
                    entry: entry.clone(),
                };
                let payload = codec::encode(&envelope)?;
                let mut bytes = LEGACY_ENTRY_MAGIC.to_vec();
                bytes.extend(payload);
                ledger.insert(index as u64 + 1, bytes.as_slice())?;
            }
        }
        write.commit()?;
        drop(db);
        Ok((doc_id, db_path))
    }

    #[test]
    fn legacy_v2_export_json_is_explicit_and_preserves_raw_attribution() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger_dir = dir.path().join("ledger");
        write_v2_fixture(&ledger_dir)?;
        let output = dir.path().join("legacy.jsonl");

        run(
            &ledger_dir,
            Some(output.display().to_string()),
            Some("default".into()),
            None,
            "json",
        )?;

        let lines = std::fs::read_to_string(output)?;
        assert_eq!(lines.lines().count(), 3);
        assert!(lines.contains("\"legacy_schema_version\":2"));
        assert!(lines.contains("\"peer_id\":\"legacy-label\""));
        Ok(())
    }

    #[test]
    fn legacy_v2_export_markdown_replays_current_structure_and_content() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger_dir = dir.path().join("ledger");
        write_v2_fixture(&ledger_dir)?;
        let output = dir.path().join("recovered");

        run(
            &ledger_dir,
            Some(output.display().to_string()),
            Some("default".into()),
            None,
            "markdown",
        )?;

        assert_eq!(
            std::fs::read_to_string(output.join("notes/legacy.md"))?,
            "legacy content"
        );
        Ok(())
    }
}
