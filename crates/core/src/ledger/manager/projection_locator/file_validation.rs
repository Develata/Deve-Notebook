//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract

use super::{ProjectionLocatorRecord, safe_repo_path_segment};
use anyhow::{Context, Result, anyhow};
use std::collections::HashSet;

pub(super) fn validate_projection_locator_file_shape(
    records: &[ProjectionLocatorRecord],
) -> Result<()> {
    let mut repo_ids = HashSet::with_capacity(records.len());
    for record in records {
        if !repo_ids.insert(record.repo_id) {
            return Err(anyhow!(
                "Projection Locator contains duplicate record for repo {}",
                record.repo_id
            ));
        }
        safe_repo_path_segment(&record.repo_name_hint).with_context(|| {
            format!(
                "Projection Locator for repo {} contains an invalid repo_name_hint",
                record.repo_id
            )
        })?;
        if !record.projection_base_abs.is_absolute() {
            return Err(anyhow!(
                "Projection Locator for {} must use an absolute projection base",
                record.repo_id
            ));
        }
    }
    Ok(())
}
