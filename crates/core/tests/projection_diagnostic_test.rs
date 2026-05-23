use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, StructureOp};
use deve_core::sync::{ProjectionDiagnosticStatus, SyncManager};
use std::sync::Arc;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_projection_base_for_all_local_repos(dir.path().join("vault"));
    (dir, Arc::new(repo))
}

#[test]
fn projection_diagnostic_reports_missing_parent_as_authority_corrupt() {
    let (_dir, repo) = new_repo();
    let doc_id = DocId::new();
    common::append_unvalidated_local_op(
        repo.as_ref(),
        repo.local_repo_name(),
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(doc_id),
                doc_id,
                parent_id: Some(NodeId::new()),
                name: "orphan.md".into(),
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );

    let sync = SyncManager::new(repo);
    let diagnostic = sync
        .diagnose_projection_local_repo("default")
        .expect("diagnose projection");

    assert_eq!(
        diagnostic.status,
        ProjectionDiagnosticStatus::AuthorityCorrupt
    );
    assert!(!diagnostic.rebuild_supported);
    assert!(diagnostic.repair_hint.contains("rebuild is unsupported"));
    let issue = diagnostic.issue.expect("authority issue");
    assert_eq!(issue.code, "missing_parent");
    assert!(issue.detail.contains("missing parent"));
}
