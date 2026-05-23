//! plan_ref:
//!   - 18_backup#backup-locator-contract
//!
//! Backup runtime boundary.
//!
//! This module currently owns locator parsing and branch path derivation only.
//! It does not open network connections, write ledger state, modify staging, or
//! touch Projection Workspaces.

mod locator;

pub use locator::{BackupLocator, BackupLocatorError, BackupProviderKind, BranchBackupLocator};
