use super::*;
use crate::models::{DocId, LedgerEntry, PeerId};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn test_find_client_op_in_local_repo() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
    let doc_id = DocId::new();
    let peer_id = PeerId::new("browser-peer");

    repo.append_generated_client_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        42,
        9,
        |seq| LedgerEntry {
            doc_id,
            op: crate::models::Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            timestamp: 1000,
            peer_id: peer_id.clone(),
            seq,
            client_id: Some(42),
            client_op_id: Some(9),
        },
    )?;

    let found = repo
        .find_client_op_in_local_repo(repo.local_repo_name(), doc_id, 42, 9)?
        .expect("client op should be indexed");
    assert_eq!(found.1.seq, 1);
    assert_eq!(
        found.1.op,
        crate::models::Op::Insert {
            pos: 0,
            content: "hello".into(),
        }
    );
    assert_eq!(found.1.client_id, Some(42));
    assert_eq!(found.1.client_op_id, Some(9));
    Ok(())
}
