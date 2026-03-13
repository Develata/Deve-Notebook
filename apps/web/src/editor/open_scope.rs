use deve_core::models::{DocId, PeerId};

pub fn can_open_doc(
    doc_id: DocId,
    docs: &[(DocId, String)],
    handshake_ready: bool,
    active_branch: Option<PeerId>,
    doc_selected: bool,
    has_repo_scope: bool,
    branch_switch_idle: bool,
    repo_switch_idle: bool,
) -> bool {
    branch_switch_idle
        && repo_switch_idle
        && has_repo_scope
        && doc_selected
        && docs
            .iter()
            .any(|(listed_doc_id, _)| *listed_doc_id == doc_id)
        && (handshake_ready || active_branch.is_some())
}

#[cfg(test)]
mod tests {
    use super::can_open_doc;
    use deve_core::models::{DocId, PeerId};

    #[test]
    fn remote_branch_can_open_without_handshake_once_doc_is_scoped() {
        let doc_id = DocId::new();
        assert!(can_open_doc(
            doc_id,
            &[(doc_id, "notes/a.md".into())],
            false,
            Some(PeerId::new("peer-a")),
            true,
            true,
            true,
            true,
        ));
    }

    #[test]
    fn open_doc_rejects_docs_outside_current_repo_listing() {
        assert!(!can_open_doc(
            DocId::new(),
            &[],
            true,
            None,
            true,
            true,
            true,
            true,
        ));
    }
}
