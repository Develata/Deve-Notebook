//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 04_repository#repo-scope-runtime
//!
//! Feature-local typed runtime handle for the search surface.

use crate::hooks::use_core::{BranchContext, DocContext, EditorContext};
use crate::i18n::Locale;
use crate::runtime::session_client::SessionClient;
use leptos::prelude::RwSignal;

#[derive(Clone)]
pub struct SearchRuntime {
    pub session: SessionClient,
    pub document: DocContext,
    pub editor: EditorContext,
    pub branch: BranchContext,
    pub locale: RwSignal<Locale>,
}
