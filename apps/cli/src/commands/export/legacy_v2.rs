//! plan_ref:
//!   - 03_storage/repair#backup-export
//!   - 03_storage/authority#redb-schema-version-contract
//!
//! Explicit, offline-only schema-v2 export. This module never opens a write transaction.

mod markdown;

use super::doc;
use anyhow::{Context, Result, anyhow, bail};
use deve_core::codec;
use deve_core::ledger::schema::{LEDGER_OPS, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY};
use deve_core::models::{DocId, LedgerEvent, PeerId};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};
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
        "markdown" | "md" => markdown::write(output, doc_id, &entries),
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

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::ledger::schema::{LEDGER_OPS, REPO_METADATA};
    use deve_core::models::{NodeId, Op, StructureOp};

    #[path = "legacy_v2_acceptance.rs"]
    mod acceptance;

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
        let (_doc_id, db_path) = write_v2_fixture(&ledger_dir)?;
        let output = dir.path().join("legacy.jsonl");
        let database_before = std::fs::read(&db_path)?;

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
        assert!(lines.contains("\"seq\":1"));
        assert!(!lines.contains("origin_peer_id"));
        assert!(!lines.contains("peer_seq"));
        assert_eq!(std::fs::read(db_path)?, database_before);
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
