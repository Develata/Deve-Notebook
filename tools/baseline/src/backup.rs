//! plan_ref:
//!   - 06_backup#projection-backup-scope
//!   - 06_backup#projection-backup-contract
//!   - 06_backup#projection-backup-locator-contract
//!   - 06_backup#projection-backup-remote-layout-contract
//!   - 06_backup#projection-backup-upload-state-machine-contract
//!   - 06_backup#projection-backup-pull-state-machine-contract
//!   - 06_backup#projection-backup-command-output-contract
//!   - 06_backup#projection-backup-secret-ref-contract
//!   - 06_backup#projection-backup-verification-contract
//!   - 06_backup#projection-backup-provider-dispatch-contract

use crate::context::BaselineContext;
use crate::spec::{RunMode, run_tsv_with_mode};
use anyhow::Result;

pub fn run() -> Result<()> {
    run_with_mode(RunMode::Full)
}

pub fn run_text() -> Result<()> {
    run_with_mode(RunMode::TextOnly)
}

fn run_with_mode(mode: RunMode) -> Result<()> {
    let ctx = BaselineContext::new("backup-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/backup.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
