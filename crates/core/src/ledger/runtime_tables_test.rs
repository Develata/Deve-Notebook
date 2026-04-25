use super::RepoManager;
use crate::ledger::schema::CLIENT_OP_INDEX;
use crate::models::{DocId, LedgerEntry, Op, PeerId};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn repairs_missing_client_op_index_for_secondary_local_repo_on_runtime_open() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    crate::test_support::create_initialized_local_repo_with_depth(
        &ledger_dir,
        2,
        "wiki",
        "urn:wiki",
    );
    let doc_id = DocId::new();
    let peer_id = PeerId::new("browser-peer");

    repo.run_on_local_repo("wiki", |db| {
        let write = db.begin_write()?;
        let _ = write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    repo.append_generated_client_op_in_local_repo("wiki", doc_id, peer_id.clone(), 42, 9, |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            1000,
            peer_id.clone(),
            seq,
            Some(42),
            Some(9),
        )
    })?;

    let found = repo
        .find_client_op_in_local_repo("wiki", doc_id, 42, 9)?
        .expect("client op should be rebuilt and indexed");
    assert_eq!(found.1.client_id, Some(42));
    assert_eq!(found.1.client_op_id, Some(9));
    Ok(())
}
