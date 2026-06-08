//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! Source-control diff session wire object for the browser client runtime.
//!
//! Invariants:
//! - `doc_id` is the stable document identity for diff; historical messages may
//!   omit it for compatibility.
//! - `path` must be a non-empty normalized path.
//! - `display_path` is UI title only; by default it equals `path`.
//! - `old_content` and `new_content` must come from the same file snapshot pair.
//! - `opened_at_ms` is monotonic for the most recent diff tab open.

use deve_core::models::DocId;
use deve_core::protocol::{ClientMessage, MergeConflictAction};

#[derive(Clone, Debug, PartialEq)]
pub struct DiffSessionWire {
    pub doc_id: Option<DocId>,
    pub path: String,
    pub display_path: String,
    pub old_content: String,
    pub new_content: String,
    pub merge_conflict: Option<MergeConflictSession>,
    pub opened_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergeConflictSession {
    pub doc_id: DocId,
    pub result_content: String,
    pub actions: Vec<MergeConflictAction>,
}

impl MergeConflictSession {
    pub fn resolve_message(
        &self,
        action: MergeConflictAction,
        result_content: Option<String>,
        scope_nonce: u64,
    ) -> ClientMessage {
        ClientMessage::ResolveMergeConflict {
            doc_id: self.doc_id,
            action,
            result_content,
            scope_nonce: Some(scope_nonce),
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> u64 {
    0
}

impl DiffSessionWire {
    pub fn new(path: String, old_content: String, new_content: String) -> Self {
        let display_path = path.clone();
        Self::with_display_path(path, display_path, old_content, new_content)
    }

    pub fn with_display_path(
        path: String,
        display_path: String,
        old_content: String,
        new_content: String,
    ) -> Self {
        Self {
            doc_id: None,
            path,
            display_path,
            old_content,
            new_content,
            merge_conflict: None,
            opened_at_ms: now_ms(),
        }
    }

    pub fn with_merge_conflict(mut self, merge_conflict: MergeConflictSession) -> Self {
        self.merge_conflict = Some(merge_conflict);
        self
    }

    pub fn with_doc_id(mut self, doc_id: Option<DocId>) -> Self {
        self.doc_id = doc_id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{DiffSessionWire, MergeConflictSession};
    use deve_core::models::DocId;
    use deve_core::protocol::{ClientMessage, MergeConflictAction};

    #[test]
    fn defaults_display_path_to_canonical_path() {
        let session = DiffSessionWire::new("notes/new.md".into(), "old".into(), "new".into());
        assert_eq!(session.doc_id, None);
        assert_eq!(session.path, "notes/new.md");
        assert_eq!(session.display_path, "notes/new.md");
    }

    #[test]
    fn keeps_display_label_separate_from_canonical_path() {
        let session = DiffSessionWire::with_display_path(
            "notes/new.md".into(),
            "notes/old.md -> notes/new.md".into(),
            "old".into(),
            "new".into(),
        );
        assert_eq!(session.path, "notes/new.md");
        assert_eq!(session.display_path, "notes/old.md -> notes/new.md");
    }

    #[test]
    fn can_attach_merge_conflict_metadata() {
        let doc_id = DocId::new();
        let session = DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into())
            .with_merge_conflict(MergeConflictSession {
                doc_id,
                result_content: "base".into(),
                actions: vec![MergeConflictAction::AcceptCurrent],
            });

        let merge = session.merge_conflict.unwrap();
        assert_eq!(merge.doc_id, doc_id);
        assert_eq!(merge.result_content, "base");
        assert_eq!(merge.actions.len(), 1);
    }

    #[test]
    fn can_attach_doc_identity() {
        let doc_id = DocId::new();
        let session = DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into())
            .with_doc_id(Some(doc_id));
        assert_eq!(session.doc_id, Some(doc_id));
    }

    #[test]
    fn merge_conflict_session_builds_scoped_resolve_message() {
        let doc_id = DocId::new();
        let session = MergeConflictSession {
            doc_id,
            result_content: "base".into(),
            actions: vec![MergeConflictAction::AcceptBoth],
        };

        match session.resolve_message(MergeConflictAction::AcceptBoth, Some("merged".into()), 9) {
            ClientMessage::ResolveMergeConflict {
                doc_id: actual_doc,
                action,
                result_content,
                scope_nonce,
            } => {
                assert_eq!(actual_doc, doc_id);
                assert_eq!(action, MergeConflictAction::AcceptBoth);
                assert_eq!(result_content.as_deref(), Some("merged"));
                assert_eq!(scope_nonce, Some(9));
            }
            other => panic!("expected ResolveMergeConflict, got {other:?}"),
        }
    }
}
