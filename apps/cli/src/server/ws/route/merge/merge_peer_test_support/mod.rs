//! Shared peer merge route test support.

mod assertions;
mod ledger;
mod state;

pub(super) use assertions::{
    MergeConflictExpectation, expect_merge_complete, expect_merge_conflict,
};
pub(super) use ledger::{
    doc_content, doc_entry_count, local_doc_content, local_doc_entry_count, seed_local_doc,
    seed_local_replace, seed_remote_insert, seed_remote_replace, seed_shared_base,
};
pub(super) use state::{
    browser_local_session, browser_remote_session, browser_writer_ready_session,
    ensure_local_projection_ready, ensure_remote_repo, reopen_state, request_merge_peer,
};
