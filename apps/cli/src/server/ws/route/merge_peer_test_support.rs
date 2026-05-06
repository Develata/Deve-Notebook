//! Shared peer merge route test support.

#[path = "merge_peer_test_support/assertions.rs"]
mod assertions;
#[path = "merge_peer_test_support/ledger.rs"]
mod ledger;
#[path = "merge_peer_test_support/state.rs"]
mod state;

pub(super) use assertions::{
    MergeConflictExpectation, expect_merge_complete, expect_merge_conflict,
};
pub(super) use ledger::{
    doc_content, doc_entry_count, local_doc_content, local_doc_entry_count, seed_local_doc,
    seed_local_replace, seed_remote_insert, seed_remote_replace, seed_shared_base,
};
pub(super) use state::{
    browser_remote_session, ensure_remote_repo, reopen_state, request_merge_peer,
};
