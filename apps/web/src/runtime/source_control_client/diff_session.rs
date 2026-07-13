//! Browser-owned state for one backend-computed diff projection.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 10_rendering#large-document-runtime
//!
//! The browser owns draft text and request identity only. Diff rows, hunks,
//! folds, word ranges and statistics always come from `deve_core`.

use std::sync::Arc;
use std::{cell::Cell, thread_local};

use deve_core::models::DocId;
use deve_core::protocol::{ClientMessage, MergeConflictAction, ServerError};
use deve_core::source_control::diff_projection::DiffProjection;
use deve_core::utils::path::to_forward_slash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffProjectionStatus {
    Loading,
    Ready,
    Debouncing { revision: u64 },
    Computing { request_id: String, revision: u64 },
    Unavailable(ServerError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSessionWire {
    pub doc_id: Option<DocId>,
    pub path: String,
    pub display_path: String,
    pub projection: Option<Arc<DiffProjection>>,
    pub draft_content: Option<String>,
    pub cache_key: Option<String>,
    pub pending_request_id: Option<String>,
    pub latest_revision: u64,
    pub status: DiffProjectionStatus,
    pub merge_conflict: Option<MergeConflictSession>,
    pub opened_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeConflictSession {
    pub doc_id: DocId,
    pub result_content: String,
    pub actions: Vec<MergeConflictAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffProjectionIntent {
    pub request_id: String,
    pub revision: u64,
    pub base_content: String,
    pub target_content: String,
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
    pub fn loading(path: String, display_path: String) -> Self {
        Self {
            doc_id: None,
            path: normalize_path(path),
            display_path,
            projection: None,
            draft_content: None,
            cache_key: None,
            pending_request_id: None,
            latest_revision: 0,
            status: DiffProjectionStatus::Loading,
            merge_conflict: None,
            opened_at_ms: now_ms(),
        }
    }

    pub fn from_projection(path: String, projection: Arc<DiffProjection>) -> Self {
        let path = normalize_path(path);
        Self::with_projection_and_display_path(path.clone(), path, projection)
    }

    pub fn with_projection_and_display_path(
        path: String,
        display_path: String,
        projection: Arc<DiffProjection>,
    ) -> Self {
        Self {
            doc_id: None,
            path: normalize_path(path),
            display_path,
            projection: Some(projection),
            draft_content: None,
            cache_key: None,
            pending_request_id: None,
            latest_revision: 0,
            status: DiffProjectionStatus::Ready,
            merge_conflict: None,
            opened_at_ms: now_ms(),
        }
    }

    pub fn begin_compute(&mut self, intent: &DiffProjectionIntent) {
        self.cache_key = None;
        self.draft_content = Some(intent.target_content.clone());
        self.latest_revision = self.latest_revision.max(intent.revision);
        self.pending_request_id = Some(intent.request_id.clone());
        self.status = DiffProjectionStatus::Computing {
            request_id: intent.request_id.clone(),
            revision: intent.revision,
        };
    }

    pub fn accepts_result(&self, request_id: &str, revision: u64) -> bool {
        matches!(
            &self.status,
            DiffProjectionStatus::Computing {
                request_id: expected,
                revision: expected_revision,
            } if expected == request_id && *expected_revision == revision
        )
    }

    pub fn accepts_error(&self, request_id: &str, revision: u64) -> bool {
        self.accepts_result(request_id, revision)
            || (revision == 0
                && matches!(self.status, DiffProjectionStatus::Loading)
                && self.pending_request_id.as_deref() == Some(request_id))
    }

    pub fn matches_pending_request(&self, request_id: Option<&str>) -> bool {
        request_id.is_some() && self.pending_request_id.as_deref() == request_id
    }

    pub fn install_projection(&mut self, projection: Arc<DiffProjection>) {
        self.draft_content = None;
        self.pending_request_id = None;
        self.projection = Some(projection);
        self.status = DiffProjectionStatus::Ready;
    }

    pub fn install_document_projection(
        &mut self,
        path: String,
        doc_id: Option<DocId>,
        projection: Arc<DiffProjection>,
    ) {
        let path = normalize_path(path);
        if self.path != path {
            self.display_path = path.clone();
        }
        self.path = path;
        self.doc_id = doc_id;
        self.install_projection(projection);
    }

    pub fn with_cache_key(mut self, cache_key: String) -> Self {
        self.cache_key = Some(cache_key);
        self
    }

    pub fn with_pending_request(mut self, request_id: String) -> Self {
        self.pending_request_id = Some(request_id);
        self
    }

    pub fn persist_draft(&mut self, draft: String) {
        self.draft_content = Some(draft);
    }

    pub fn install_error(&mut self, error: ServerError) {
        self.status = DiffProjectionStatus::Unavailable(error);
    }

    pub fn with_merge_conflict(mut self, merge_conflict: MergeConflictSession) -> Self {
        self.draft_content = Some(merge_conflict.result_content.clone());
        self.merge_conflict = Some(merge_conflict);
        self
    }

    pub fn with_doc_id(mut self, doc_id: Option<DocId>) -> Self {
        self.doc_id = doc_id;
        self
    }

    #[cfg(test)]
    pub fn new(path: String, old_content: String, new_content: String) -> Self {
        let projection = deve_core::source_control::diff_projection::compute_diff_projection(
            old_content,
            new_content,
        )
        .expect("test diff projection");
        Self::from_projection(path, Arc::new(projection))
    }

    #[cfg(test)]
    pub fn with_display_path(
        path: String,
        display_path: String,
        old_content: String,
        new_content: String,
    ) -> Self {
        let projection = deve_core::source_control::diff_projection::compute_diff_projection(
            old_content,
            new_content,
        )
        .expect("test diff projection");
        Self::with_projection_and_display_path(path, display_path, Arc::new(projection))
    }
}

fn normalize_path(path: String) -> String {
    to_forward_slash(&path)
}

thread_local! {
    static NEXT_DIFF_REVISION: Cell<u64> = const { Cell::new(0) };
}

pub fn next_diff_revision() -> u64 {
    NEXT_DIFF_REVISION.with(|revision| {
        let next = revision.get().saturating_add(1).max(1);
        revision.set(next);
        next
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DiffProjectionIntent, DiffProjectionStatus, DiffSessionWire, MergeConflictSession,
    };
    use deve_core::models::DocId;
    use deve_core::protocol::{ClientMessage, MergeConflictAction};

    #[test]
    fn result_identity_is_request_and_revision_exact() {
        let mut session = DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into());
        let intent = DiffProjectionIntent {
            request_id: "req-1".into(),
            revision: 7,
            base_content: "old".into(),
            target_content: "draft".into(),
        };
        session.begin_compute(&intent);
        assert!(session.accepts_result("req-1", 7));
        assert!(!session.accepts_result("req-1", 6));
        assert!(!session.accepts_result("req-2", 7));
        assert!(matches!(
            session.status,
            DiffProjectionStatus::Computing { .. }
        ));
    }

    #[test]
    fn projection_error_keeps_base_projection_and_draft_for_retry() {
        let mut session = DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into());
        let intent = DiffProjectionIntent {
            request_id: "req-1".into(),
            revision: 1,
            base_content: "old".into(),
            target_content: "draft".into(),
        };
        session.begin_compute(&intent);
        session.install_error(deve_core::protocol::ServerError::new(
            deve_core::protocol::ServerErrorCode::DiffComputeFailed,
        ));

        assert_eq!(session.draft_content.as_deref(), Some("draft"));
        assert_eq!(
            session
                .projection
                .as_ref()
                .map(|projection| projection.base_content.as_str()),
            Some("old")
        );
        assert!(matches!(
            session.status,
            DiffProjectionStatus::Unavailable(_)
        ));
        assert_eq!(session.latest_revision, 1);
    }

    #[test]
    fn commit_file_loading_accepts_only_correlated_revision_zero_error() {
        let session = DiffSessionWire::loading("notes/a.md".into(), "old -> new".into())
            .with_pending_request("commit-file-1".into());
        assert!(session.accepts_error("commit-file-1", 0));
        assert!(!session.accepts_error("commit-file-2", 0));
        assert!(!session.accepts_error("commit-file-1", 1));
    }

    #[test]
    fn keeps_display_label_separate_from_normalized_path() {
        let session = DiffSessionWire::with_display_path(
            "notes\\new.md".into(),
            "notes/old.md -> notes/new.md".into(),
            "old".into(),
            "new".into(),
        );
        assert_eq!(session.path, "notes/new.md");
        assert_eq!(session.display_path, "notes/old.md -> notes/new.md");
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
