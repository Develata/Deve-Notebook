//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-contract
//!   - 06_backup#projection-backup-command-output-contract
//!   - 06_backup#projection-backup-verification-contract
//!
//! Runtime outcome contract guard for Remote Projection provider I/O.

use crate::remote_projection_transport::provider_io_not_ready;
use anyhow::Result;
use deve_core::remote_projection::{RemoteProjectionAuthorityEffects, RemoteProjectionPullOutcome};
use std::collections::BTreeSet;

pub(crate) fn ensure_projection_transport_pull_outcome_contract(
    outcome: &RemoteProjectionPullOutcome,
) -> Result<()> {
    ensure_projection_transport_effects_absent(&outcome.effects)?;
    ensure_unique_pull_paths(outcome)?;
    if !outcome.overwrites_projection_workspace {
        return Err(provider_io_not_ready(
            "provider outcome violates remote projection transport contract: pull must overwrite only the Projection Workspace",
        ));
    }
    if !outcome.external_changes_confirmation_required {
        return Err(provider_io_not_ready(
            "provider outcome violates remote projection transport contract: pull must require External Changes confirmation",
        ));
    }
    ensure_diagnostic_metadata(outcome.provider_metadata_is_diagnostic_only)
}

fn ensure_unique_pull_paths(outcome: &RemoteProjectionPullOutcome) -> Result<()> {
    let mut paths = BTreeSet::new();
    for file in &outcome.files {
        if !paths.insert(file.path()) {
            return Err(provider_io_not_ready(format!(
                "provider outcome violates remote projection transport contract: duplicate projection path {}",
                file.path()
            )));
        }
    }
    Ok(())
}

fn ensure_diagnostic_metadata(provider_metadata_is_diagnostic_only: bool) -> Result<()> {
    if provider_metadata_is_diagnostic_only {
        Ok(())
    } else {
        Err(provider_io_not_ready(
            "provider outcome violates remote projection transport contract: provider metadata must be diagnostic-only",
        ))
    }
}

fn ensure_projection_transport_effects_absent(
    effects: &RemoteProjectionAuthorityEffects,
) -> Result<()> {
    if effects.writes_ledger
        || effects.writes_source_control_staging
        || effects.writes_commit_anchor
        || effects.writes_git_main_mirror
        || effects.confirms_external_changes
    {
        return Err(provider_io_not_ready(format!(
            "provider outcome violates remote projection transport contract: authority effects must be absent \
             (writes_ledger={}, writes_source_control_staging={}, writes_commit_anchor={}, writes_git_main_mirror={}, confirms_external_changes={})",
            effects.writes_ledger,
            effects.writes_source_control_staging,
            effects.writes_commit_anchor,
            effects.writes_git_main_mirror,
            effects.confirms_external_changes,
        )));
    }
    Ok(())
}
