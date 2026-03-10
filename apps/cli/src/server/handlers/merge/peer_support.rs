use crate::server::{AppState, channel::DualChannel};
use deve_core::models::DocId;
use std::sync::Arc;

use super::errors;

pub(super) fn resolve_doc_path(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_name: &str,
    doc_id: DocId,
) -> Option<String> {
    match state
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
    {
        Ok(Some(meta)) => Some(meta.path),
        Ok(None) => {
            errors::storage_not_found(ch, "Doc path not found for merged document");
            None
        }
        Err(e) => {
            errors::request_failed(ch, format!("Failed to resolve merged doc path: {}", e));
            None
        }
    }
}
