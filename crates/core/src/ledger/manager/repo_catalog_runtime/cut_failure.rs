//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Exact publication-phase error projection for catalog lifecycle cuts.

use super::RepoCatalogError;
use super::store::{RepoCatalogPublishFailure, RepoCatalogPublishPhase};
use crate::models::RepoId;

pub(super) fn publish_error(
    repo_id: RepoId,
    failure: RepoCatalogPublishFailure,
) -> RepoCatalogError {
    publish_error_with_abort(repo_id, failure, None)
}

pub(super) fn publish_error_with_abort(
    repo_id: RepoId,
    failure: RepoCatalogPublishFailure,
    abort: Option<String>,
) -> RepoCatalogError {
    let phase = match failure.phase {
        RepoCatalogPublishPhase::BeforeReplace => "before_replace",
        RepoCatalogPublishPhase::AfterReplaceSync => "after_replace_sync",
    };
    let mut cleanup = failure.cleanup.map(|error| error.to_string());
    if let Some(abort) = abort {
        cleanup = Some(match cleanup {
            Some(cleanup) => format!("temp_cleanup={cleanup}; membership_abort={abort}"),
            None => format!("membership_abort={abort}"),
        });
    }
    match failure.phase {
        RepoCatalogPublishPhase::BeforeReplace => RepoCatalogError::PublishFailed {
            repo_id,
            phase,
            primary: failure.primary.to_string(),
            cleanup,
        },
        RepoCatalogPublishPhase::AfterReplaceSync => RepoCatalogError::CutOutcomeUnknown {
            repo_id,
            detail: match cleanup {
                Some(cleanup) => format!(
                    "phase={phase}; primary={}; cleanup={cleanup}",
                    failure.primary
                ),
                None => format!("phase={phase}; primary={}", failure.primary),
            },
        },
    }
}
