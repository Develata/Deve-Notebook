//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 04_repository#repo-scope-runtime

use super::docs_test_support::DocsHarness;
use deve_core::models::{LedgerEntry, Op, PeerId};

pub(crate) fn seed_file(h: &DocsHarness, path: &str, content: &str) -> anyhow::Result<()> {
    let repo_name = h.state.repo.local_repo_name();
    let (doc_id, _ops) = h
        .state
        .repo
        .apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    h.state
        .repo
        .append_generated_op_in_local_repo(repo_name, doc_id, PeerId::new("test-peer"), |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("test-peer"),
                seq,
                None,
                None,
            )
        })?;
    Ok(())
}
