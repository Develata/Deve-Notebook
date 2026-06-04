//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip

use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditorTabKey {
    Document(DocId),
    Diff(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EditorDocumentTab {
    pub doc_id: DocId,
    pub title: String,
    pub tooltip: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EditorDiffTab {
    pub key: String,
    pub title: String,
    pub tooltip: String,
    pub session: DiffSessionWire,
}

pub(crate) fn display_name(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .next_back()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

pub(crate) fn document_tab_from_docs(
    docs: &[(DocId, String)],
    doc_id: DocId,
) -> Option<EditorDocumentTab> {
    docs.iter()
        .find(|(candidate, _)| *candidate == doc_id)
        .map(|(_, path)| EditorDocumentTab {
            doc_id,
            title: display_name(path),
            tooltip: path.clone(),
        })
}

pub(crate) fn diff_tab_key(session: &DiffSessionWire) -> String {
    if let Some(doc_id) = session.doc_id {
        return format!("doc:{doc_id}");
    }
    format!("path:{}:{}", session.path, session.opened_at_ms)
}

pub(crate) fn diff_tab_from_session(session: DiffSessionWire) -> EditorDiffTab {
    let title_source = if session.display_path.is_empty() {
        &session.path
    } else {
        &session.display_path
    };
    let tooltip = if session.display_path != session.path {
        format!("{}\n{}", session.display_path, session.path)
    } else {
        session.path.clone()
    };

    EditorDiffTab {
        key: diff_tab_key(&session),
        title: display_name(title_source),
        tooltip,
        session,
    }
}
