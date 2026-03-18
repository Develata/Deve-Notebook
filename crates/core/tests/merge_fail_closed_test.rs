use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS};
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use tempfile::tempdir;

#[test]
fn merge_peer_fails_closed_when_remote_ops_are_corrupted() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let peer_id = PeerId::new("peer-remote");
    let remote = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };
    repo.ensure_shadow_repo_info(&peer_id, &remote)?;

    let doc_id = DocId::new();
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "local".into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )?;

    repo.run_on_shadow_repo_by_id(&peer_id, &remote.uuid, |db| {
        let write = db.begin_write()?;
        write
            .open_table(LEDGER_OPS)?
            .insert(1, [1_u8, 2, 3].as_slice())?;
        write
            .open_multimap_table(DOC_OPS)?
            .insert(doc_id.as_u128(), 1)?;
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .merge_peer_in_local_repo(repo.local_repo_name(), &peer_id, &remote.uuid, doc_id)
        .expect_err("corrupted remote ops must fail closed");
    let detail = err.to_string().to_ascii_lowercase();
    assert!(
        detail.contains("decode")
            || detail.contains("deserialize")
            || detail.contains("unsupported ledger entry schema")
            || detail.contains("unexpected end")
    );
    Ok(())
}
