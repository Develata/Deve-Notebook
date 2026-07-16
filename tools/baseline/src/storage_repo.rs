//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 03_storage/index#browser-storage-layering
//!   - 03_storage/index#internal-path-normalization
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/authority#ledger-entry-format-contract
//!   - 03_storage/authority#redb-schema-version-contract
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/projection#projection-locator-contract
//!   - 03_storage/watcher#watcher-contract
//!   - 03_storage/repair#backup-export
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-health-and-repair
//!   - 04_repository#repo-scope-runtime
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-contract

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("storage-repo-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/storage_repo.tsv"))?;
    ctx.ok();
    Ok(())
}
