//! plan_ref:
//!   - 04_storage.md §Data Integrity & Recovery (Disaster Recovery)
//!
//! Builds JSONL export rows from `LEDGER_OPS`. Resolves each entry's
//! current path through the node/projection layer so exported lines are
//! self-describing for offline audit and migration.

use crate::admin_api::ExportEntry;
use anyhow::Result;
use deve_core::ledger::{RepoManager, node_meta, range};
use deve_core::models::LedgerEntry;

pub fn build(repo: &RepoManager, repo_name: &str) -> Result<Vec<ExportEntry>> {
    let ops = repo.run_on_local_repo(repo_name, |db| {
        let max_seq = range::get_max_seq(db)?;
        range::get_ops_in_range(db, 1, max_seq + 1)
    })?;
    let mut entries = Vec::with_capacity(ops.len());
    for (global_seq, entry) in ops {
        entries.push(ExportEntry {
            global_seq,
            current_path: resolve_path(repo, repo_name, &entry)?,
            entry,
        });
    }
    Ok(entries)
}

fn resolve_path(
    repo: &RepoManager,
    repo_name: &str,
    entry: &LedgerEntry,
) -> Result<Option<String>> {
    if let Some(doc_id) = entry.doc_id {
        return Ok(repo
            .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
            .map(|meta| meta.path));
    }
    if let Some(node_id) = entry.structure_node_id() {
        return repo.run_on_local_repo(repo_name, |db| {
            Ok(node_meta::get_node_meta(db, node_id)?.map(|meta| meta.path))
        });
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
    use deve_core::models::{LedgerEntry, Op, PeerId};
    use tempfile::TempDir;

    #[test]
    fn jsonl_roundtrip_is_monotonic_and_line_stable() {
        let dir = TempDir::new().expect("create tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
        for name in ["a.md", "b.md", "c.md"] {
            repo.apply_file_structure_in_local_repo(repo.local_repo_name(), name, None, "test")
                .expect("create file");
        }
        let entries = build(&repo, repo.local_repo_name()).expect("build export");
        assert!(!entries.is_empty(), "export must emit rows");

        // Serialize each row as one JSONL line and parse it back.
        let mut last_seq = 0u64;
        for entry in &entries {
            let line = serde_json::to_string(entry).expect("serialize");
            assert!(!line.contains('\n'), "JSONL row must be single-line");
            let parsed: ExportEntry = serde_json::from_str(&line).expect("roundtrip parse");
            assert_eq!(parsed.global_seq, entry.global_seq);
            assert!(
                parsed.global_seq > last_seq,
                "global_seq must be strictly monotonic"
            );
            last_seq = parsed.global_seq;
        }
    }

    #[test]
    fn includes_dir_structure_fact_in_export() {
        let dir = TempDir::new().expect("create tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
        repo.apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes", "test")
            .expect("create dir");

        let entries = build(&repo, repo.local_repo_name()).expect("build export");
        assert!(entries.iter().any(|entry| {
            entry.entry.doc_id.is_none() && entry.current_path.as_deref() == Some("notes")
        }));
    }

    #[test]
    fn file_entry_prefers_node_projection_path() {
        let dir = TempDir::new().expect("create tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
        let (doc_id, _ops) = repo
            .apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/a.md", None, "test")
            .expect("create file");
        repo.append_generated_op_in_local_repo(
            repo.local_repo_name(),
            doc_id,
            PeerId::new("local"),
            |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    Op::Insert {
                        pos: 0,
                        content: "hello".into(),
                    },
                    1,
                    PeerId::new("local"),
                    seq,
                    None,
                    None,
                )
            },
        )
        .expect("append content");
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            let write_txn = db.begin_write()?;
            {
                let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
                let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
                p2d.remove("notes/a.md")?;
                p2d.insert("stale/a.md", doc_id.as_u128())?;
                d2p.insert(doc_id.as_u128(), "stale/a.md")?;
            }
            write_txn.commit()?;
            Ok(())
        })
        .expect("poison metadata");

        let entries = build(&repo, repo.local_repo_name()).expect("build export");
        assert!(entries.iter().any(|entry| {
            entry.entry.doc_id == Some(doc_id)
                && entry.current_path.as_deref() == Some("notes/a.md")
        }));
    }
}
