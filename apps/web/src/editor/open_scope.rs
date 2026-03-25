use deve_core::models::DocId;

#[derive(Clone)]
pub struct OpenDocScope<'a> {
    pub doc_id: DocId,
    pub docs: &'a [(DocId, String)],
    pub doc_selected: bool,
    pub has_repo_scope: bool,
    pub branch_switch_idle: bool,
    pub repo_switch_idle: bool,
}

pub fn can_open_doc(scope: OpenDocScope<'_>) -> bool {
    scope.branch_switch_idle
        && scope.repo_switch_idle
        && scope.has_repo_scope
        && scope.doc_selected
        && scope
            .docs
            .iter()
            .any(|(listed_doc_id, _)| *listed_doc_id == scope.doc_id)
}

#[cfg(test)]
mod tests {
    use super::{OpenDocScope, can_open_doc};
    use deve_core::models::DocId;

    #[test]
    fn remote_branch_can_open_without_handshake_once_doc_is_scoped() {
        let doc_id = DocId::new();
        assert!(can_open_doc(OpenDocScope {
            doc_id,
            docs: &[(doc_id, "notes/a.md".into())],
            doc_selected: true,
            has_repo_scope: true,
            branch_switch_idle: true,
            repo_switch_idle: true,
        }));
    }

    #[test]
    fn local_repo_can_open_before_handshake_once_repo_scope_is_stable() {
        let doc_id = DocId::new();
        assert!(can_open_doc(OpenDocScope {
            doc_id,
            docs: &[(doc_id, "notes/local.md".into())],
            doc_selected: true,
            has_repo_scope: true,
            branch_switch_idle: true,
            repo_switch_idle: true,
        }));
    }

    #[test]
    fn open_doc_rejects_docs_outside_current_repo_listing() {
        assert!(!can_open_doc(OpenDocScope {
            doc_id: DocId::new(),
            docs: &[],
            doc_selected: true,
            has_repo_scope: true,
            branch_switch_idle: true,
            repo_switch_idle: true,
        }));
    }
}
