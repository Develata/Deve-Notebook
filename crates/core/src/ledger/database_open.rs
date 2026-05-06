//! plan_ref:
//!   - 04_storage#facts-partition
//!   - 04_storage#repo-runtime-layout
//!   - 06_repository#repo-catalog-contract
//!
//! Global Redb open/create helpers backed by the process-wide database cache.

use super::database_cache::{
    CachedDatabaseEntry, OPENED_DBS, current_file_stamp, reusable_cached_database,
};
use anyhow::{Context, Result};
use redb::Database;
use std::path::Path;
use std::sync::Arc;

pub(crate) fn cached_database(db_path: &Path) -> Result<Arc<Database>> {
    if !checked_exists(db_path, "database path")? {
        return Err(anyhow::anyhow!("Repository not found: {:?}", db_path));
    }
    cached_or_create_database(db_path)
}

pub(crate) fn cached_or_create_database(db_path: &Path) -> Result<Arc<Database>> {
    if let Some(db) = reusable_cached_database(db_path)? {
        return Ok(db);
    }
    OPENED_DBS
        .write()
        .map_err(|_| anyhow::anyhow!("Database cache lock poisoned while removing {:?}", db_path))?
        .remove(db_path);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Database::create(db_path)?;
    let arc_db = Arc::new(db);

    {
        let mut cache = OPENED_DBS.write().map_err(|_| {
            anyhow::anyhow!("Database cache lock poisoned while storing {:?}", db_path)
        })?;
        cache.insert(
            db_path.to_path_buf(),
            CachedDatabaseEntry {
                db: arc_db.clone(),
                stamp: current_file_stamp(db_path)?,
            },
        );
    }

    tracing::info!("Opened and cached database: {:?}", db_path);
    Ok(arc_db)
}

fn checked_exists(path: &Path, context: &str) -> Result<bool> {
    path.try_exists()
        .with_context(|| format!("Failed to stat {}: {:?}", context, path))
}
