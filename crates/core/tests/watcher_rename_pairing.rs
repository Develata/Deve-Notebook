//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

mod common;
mod watcher_test_support;

use deve_core::models::NodeId;
use deve_core::source_control::ChangeStatus;
use watcher_test_support::Harness;

#[test]
fn watcher_pairs_rename_and_preserves_doc_identity() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let doc_id = h.commit_doc("main", "notes/a.md", "base")?;
    h.start_watchers()?;

    std::fs::rename(
        h.workspace_path("main", "notes/a.md")?,
        h.workspace_path("main", "notes/b.md")?,
    )?;

    let added = h.wait_pending("main", "notes/b.md", ChangeStatus::Added)?;
    let deleted = h.wait_pending("main", "notes/a.md", ChangeStatus::Deleted)?;
    assert_eq!(added.renamed_from.as_deref(), Some("notes/a.md"));
    assert_eq!(added.doc_id, Some(doc_id));
    assert_eq!(deleted.doc_id, Some(doc_id));
    assert_eq!(
        NodeId::from_doc_id(doc_id),
        NodeId::from_doc_id(added.doc_id.expect("doc id"))
    );
    Ok(())
}
