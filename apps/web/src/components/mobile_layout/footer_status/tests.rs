use super::{mobile_load_status_text, pending_ack_count_for_current_scope};
use crate::i18n::{Locale, t};
use crate::runtime::document::pending::{
    PendingLocalEditInput, PendingLocalEdits, push_pending_edit,
};
use deve_core::models::{DocId, Op, RepoId};

fn push_insert(
    pending: &mut PendingLocalEdits,
    repo_id: RepoId,
    doc_id: DocId,
    scope_nonce: u64,
    client_op_id: u64,
) {
    push_pending_edit(
        pending,
        PendingLocalEditInput {
            repo_id,
            doc_id,
            scope_nonce,
            client_id: 1,
            client_op_id,
            base_version: 0,
            op: Op::Insert {
                pos: 0,
                content: "x".into(),
            },
        },
    );
}

#[test]
fn pending_ack_count_uses_current_repo_scope() {
    let current_repo = RepoId::from_u128(1);
    let other_repo = RepoId::from_u128(2);
    let current_doc = DocId::from_u128(10);
    let mut pending = PendingLocalEdits::new();

    push_insert(&mut pending, current_repo, current_doc, 7, 1);
    push_insert(&mut pending, current_repo, current_doc, 8, 2);
    push_insert(&mut pending, other_repo, current_doc, 7, 3);

    assert_eq!(
        pending_ack_count_for_current_scope(
            &pending,
            Some(current_doc),
            Some(&current_repo.to_string()),
            7,
        ),
        1
    );
    assert_eq!(
        pending_ack_count_for_current_scope(&pending, Some(current_doc), None, 7),
        0
    );
    assert_eq!(
        pending_ack_count_for_current_scope(&pending, Some(current_doc), Some("not-a-uuid"), 7),
        0
    );
}

#[test]
fn mobile_load_status_text_uses_bottom_bar_i18n_facade() {
    assert_eq!(
        mobile_load_status_text(Locale::En, 2, 5, 40, false),
        "Loading 2/5 (~40ms)"
    );
    assert_eq!(
        mobile_load_status_text(Locale::Zh, 2, 5, 40, false),
        t::bottom_bar::loading_progress(Locale::Zh, 2, 5, 40)
    );
    assert_eq!(
        mobile_load_status_text(Locale::Zh, 2, 5, 40, true),
        t::bottom_bar::loading_progress_compact(Locale::Zh, 2, 5)
    );
    assert_eq!(
        mobile_load_status_text(Locale::Zh, 0, 0, 0, true),
        t::bottom_bar::loading(Locale::Zh)
    );
}
