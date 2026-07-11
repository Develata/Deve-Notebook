//! STORE-001 / STORE-003 / STORE-004: storage layer acceptance tests.

use deve_core::ledger::RepoManager;
use deve_core::ledger::{
    DOC_OPS, LEDGER_OPS, NODEID_TO_META, PATH_TO_NODEID, SNAPSHOT_DATA, SNAPSHOT_INDEX,
};
use deve_core::models::{LedgerEntry, Op};
use tempfile::TempDir;

fn new_repo(snapshot_depth: usize) -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("tempdir");
    let repo =
        RepoManager::init(dir.path().join("ledger"), snapshot_depth, None, None).expect("init");
    (dir, repo)
}

/// STORE-001: After init, the trinity directory structure exists.
#[test]
fn trinity_dir_structure_after_init() {
    let (dir, _repo) = new_repo(10);
    let ledger = dir.path().join("ledger");
    assert!(ledger.join("local").exists(), "local/ must exist");
    assert!(ledger.join("remotes").exists(), "remotes/ must exist");

    let has_redb = std::fs::read_dir(ledger.join("local"))
        .expect("read local/")
        .filter_map(Result::ok)
        .any(|e| e.path().extension().is_some_and(|ext| ext == "redb"));
    assert!(has_redb, "local/ must contain a .redb file");
}

/// STORE-003: All required redb tables are reachable after init.
#[test]
fn required_redb_tables_exist_after_init() {
    let (_dir, repo) = new_repo(10);
    let name = repo.local_repo_name().to_string();

    repo.run_on_local_repo(&name, |db| {
        let rtx = db.begin_read()?;
        // Regular tables
        rtx.open_table(NODEID_TO_META)?;
        rtx.open_table(PATH_TO_NODEID)?;
        rtx.open_table(LEDGER_OPS)?;
        rtx.open_table(SNAPSHOT_DATA)?;
        // Multimap tables
        rtx.open_multimap_table(DOC_OPS)?;
        rtx.open_multimap_table(SNAPSHOT_INDEX)?;
        Ok(())
    })
    .expect("all required tables must be reachable");
}

/// STORE-004: Snapshot pruning respects depth limit.
#[test]
fn snapshot_respects_depth_limit() {
    let (_dir, repo) = new_repo(3);
    let name = repo.local_repo_name().to_string();
    let peer = repo.local_peer_id().clone();

    let (doc_id, _) = repo
        .apply_file_structure_in_local_repo(&name, "snap.md", None, "test")
        .expect("create file");

    // Append 5 content ops, saving a snapshot after each.
    let mut content = String::new();
    for i in 0..5u32 {
        let chunk = format!("L{i}\n");
        let pos = content.encode_utf16().count() as u32;
        let (global_seq, _) = repo
            .append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
                LedgerEntry::new_content(
                    doc_id,
                    Op::Insert {
                        pos,
                        content: chunk.clone().into(),
                    },
                    i as i64,
                    peer.clone(),
                    seq,
                    None,
                    None,
                )
            })
            .expect("append op");
        content.push_str(&chunk);
        repo.save_snapshot_in_local_repo(&name, doc_id, global_seq, &content)
            .expect("save snapshot");
    }

    // Latest snapshot must be loadable.
    let (_, snap_content) = repo
        .load_latest_snapshot_in_local_repo(&name, doc_id)
        .expect("load")
        .expect("snapshot must exist");
    assert_eq!(snap_content, content);

    // Count entries in SNAPSHOT_INDEX — must be ≤ depth (3).
    let count = repo
        .run_on_local_repo(&name, |db| {
            let rtx = db.begin_read()?;
            let table = rtx.open_multimap_table(SNAPSHOT_INDEX)?;
            let iter = table.get(doc_id.as_u128())?;
            Ok(iter.count())
        })
        .expect("read snapshot index");
    assert!(count <= 3, "snapshot count {count} must be ≤ depth limit 3");
}
