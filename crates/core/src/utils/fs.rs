//! plan_ref: infra

use anyhow::{Context, Result};
use std::path::Path;

pub fn checked_exists(path: &Path, context: &str) -> Result<bool> {
    path.try_exists()
        .with_context(|| format!("Failed to stat {}: {:?}", context, path))
}
