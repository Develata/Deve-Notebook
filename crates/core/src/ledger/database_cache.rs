//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout

use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::time::SystemTime;

use redb::Database;
use std::collections::HashMap;
use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::sync::{Arc, RwLock};
use tracing::warn;

#[derive(Clone)]
pub(crate) struct CachedDatabaseEntry {
    pub db: Arc<Database>,
    pub stamp: Option<FileStamp>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStamp {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(windows)]
    file_id: file_id::FileId,
    len: u64,
    modified: Option<SystemTime>,
}

impl FileStamp {
    pub(crate) fn same_file_identity(self, other: Self) -> bool {
        #[cfg(unix)]
        {
            self.dev == other.dev && self.ino == other.ino
        }
        #[cfg(windows)]
        {
            self.file_id == other.file_id
        }
        #[cfg(not(any(unix, windows)))]
        {
            self.len == other.len && self.modified == other.modified
        }
    }
}

pub(crate) static OPENED_DBS: std::sync::LazyLock<
    RwLock<HashMap<std::path::PathBuf, CachedDatabaseEntry>>,
> = std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) fn reusable_cached_database(path: &Path) -> Result<Option<Arc<Database>>> {
    let current_stamp = current_file_stamp(path)?;
    let cache = OPENED_DBS.read().map_err(|_| {
        anyhow!(
            "Database cache lock poisoned while reading cached entry for {:?}",
            path
        )
    })?;
    let Some(entry) = cache.get(path) else {
        return Ok(None);
    };
    if entry.stamp == current_stamp {
        tracing::debug!("Database cache hit: {:?}", path);
        return Ok(Some(entry.db.clone()));
    }
    if let (Some(previous), Some(current)) = (entry.stamp, current_stamp)
        && previous.same_file_identity(current)
        && stale_same_identity_cache_entry_is_reusable(path)?
    {
        drop(cache);
        let mut cache = OPENED_DBS.write().map_err(|_| {
            anyhow!(
                "Database cache lock poisoned while refreshing cached stamp for {:?}",
                path
            )
        })?;
        let Some(entry) = cache.get_mut(path) else {
            return Ok(None);
        };
        entry.stamp = Some(current);
        tracing::debug!("Database cache stamp refreshed: {:?}", path);
        return Ok(Some(entry.db.clone()));
    }
    Ok(None)
}

#[cfg(windows)]
fn stale_same_identity_cache_entry_is_reusable(_path: &Path) -> Result<bool> {
    Ok(true)
}

#[cfg(not(windows))]
fn stale_same_identity_cache_entry_is_reusable(path: &Path) -> Result<bool> {
    path_looks_like_redb(path)
}

pub(crate) fn current_file_stamp(path: &Path) -> Result<Option<FileStamp>> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(file_stamp(path, &metadata)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(anyhow!(err)).context(format!(
            "Failed to read database cache metadata: {:?}",
            path
        )),
    }
}

#[cfg(not(windows))]
pub(crate) fn path_looks_like_redb(path: &Path) -> Result<bool> {
    const REDB_MAGIC: [u8; 9] = [b'r', b'e', b'd', b'b', 0x1A, 0x0A, 0xA9, 0x0D, 0x0A];
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open database header while checking {:?}", path))?;
    let mut buf = [0u8; REDB_MAGIC.len()];
    std::io::Read::read_exact(&mut file, &mut buf)
        .with_context(|| format!("Failed to read database header while checking {:?}", path))?;
    Ok(buf == REDB_MAGIC)
}

#[cfg(unix)]
fn file_stamp(_path: &Path, metadata: &Metadata) -> Result<FileStamp> {
    Ok(FileStamp {
        dev: metadata.dev(),
        ino: metadata.ino(),
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[cfg(windows)]
fn file_stamp(path: &Path, metadata: &Metadata) -> Result<FileStamp> {
    Ok(FileStamp {
        file_id: file_id::get_file_id(path)
            .with_context(|| format!("Failed to read database cache file identity: {:?}", path))?,
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[cfg(not(any(unix, windows)))]
fn file_stamp(_path: &Path, metadata: &Metadata) -> Result<FileStamp> {
    Ok(FileStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

pub(crate) fn register_database(db_path: &Path, db: Arc<Database>) -> Result<()> {
    OPENED_DBS
        .write()
        .map_err(|_| {
            anyhow!(
                "Database cache lock poisoned while registering {:?}",
                db_path
            )
        })?
        .insert(
            db_path.to_path_buf(),
            CachedDatabaseEntry {
                db,
                stamp: current_file_stamp(db_path)?,
            },
        );
    Ok(())
}

pub(crate) fn relocate_database_path(old_path: &Path, new_path: &Path) -> Result<()> {
    let mut cache = OPENED_DBS.write().map_err(|_| {
        anyhow!(
            "Database cache lock poisoned while relocating {:?} -> {:?}",
            old_path,
            new_path
        )
    })?;
    if let Some(mut entry) = cache.remove(old_path) {
        entry.stamp = current_file_stamp(new_path)?;
        cache.insert(new_path.to_path_buf(), entry);
    } else {
        warn!(
            "Database cache relocate skipped: {:?} not present while moving to {:?}",
            old_path, new_path
        );
    }
    Ok(())
}

pub(crate) fn evict_database_paths_under(root: &Path) -> Result<()> {
    OPENED_DBS
        .write()
        .map_err(|_| {
            anyhow!(
                "Database cache lock poisoned while evicting under {:?}",
                root
            )
        })?
        .retain(|path, _| !path.starts_with(root));
    Ok(())
}

#[cfg(test)]
mod tests;
