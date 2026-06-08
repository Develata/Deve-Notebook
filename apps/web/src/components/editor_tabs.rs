//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher
//!
//! View-local editor surface tab state shared by desktop and mobile shells.

mod close;
mod model;
mod ops;
mod policy;
mod runtime;

#[cfg(test)]
mod tests;

pub(crate) use model::{
    EditorDiffTab, EditorDocumentTab, EditorTabKey, diff_tab_from_session, document_tab_from_docs,
};
pub(crate) use runtime::{
    EditorTabRuntimeInputs, create_current_editor_doc, create_editor_tab_runtime,
};
