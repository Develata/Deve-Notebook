//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!
//! Desktop editor group tab strip modules. This is view-local shell state only.

mod model;
mod ops;
mod strip;

#[cfg(test)]
mod tests;

pub(crate) use model::{
    EditorDiffTab, EditorDocumentTab, EditorTabKey, diff_tab_from_session, diff_tab_key,
    document_tab_from_docs,
};
pub(crate) use ops::{remove_diff_tab, remove_document_tab, upsert_diff_tab, upsert_document_tab};
pub(crate) use strip::EditorTabStrip;
