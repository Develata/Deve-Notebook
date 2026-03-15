use std::path::Path;
use std::time::SystemTime;

use redb::Database;
use std::collections::HashMap;
use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::sync::{Arc, RwLock};

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
    volume_serial_number: u64,
    #[cfg(windows)]
    file_index: u64,
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
            self.volume_serial_number == other.volume_serial_number
                && self.file_index == other.file_index
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

pub(crate) fn reusable_cached_database(path: &Path) -> Option<Arc<Database>> {
    let current_stamp = current_file_stamp(path);
    let cache = OPENED_DBS.read().unwrap();
    let entry = cache.get(path)?;
    if entry.stamp == current_stamp {
        tracing::debug!("Database cache hit: {:?}", path);
        return Some(entry.db.clone());
    }
    if let (Some(previous), Some(current)) = (entry.stamp, current_stamp)
        && previous.same_file_identity(current)
        && path_looks_like_redb(path)
    {
        drop(cache);
        let mut cache = OPENED_DBS.write().unwrap();
        let entry = cache.get_mut(path)?;
        entry.stamp = Some(current);
        tracing::debug!("Database cache stamp refreshed: {:?}", path);
        return Some(entry.db.clone());
    }
    None
}

pub(crate) fn current_file_stamp(path: &Path) -> Option<FileStamp> {
    std::fs::metadata(path)
        .ok()
        .map(|metadata| file_stamp(&metadata))
}

pub(crate) fn path_looks_like_redb(path: &Path) -> bool {
    const REDB_MAGIC: [u8; 9] = [b'r', b'e', b'd', b'b', 0x1A, 0x0A, 0xA9, 0x0D, 0x0A];
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; REDB_MAGIC.len()];
    match std::io::Read::read_exact(&mut file, &mut buf) {
        Ok(()) => buf == REDB_MAGIC,
        Err(_) => false,
    }
}

#[cfg(unix)]
fn file_stamp(metadata: &Metadata) -> FileStamp {
    FileStamp {
        dev: metadata.dev(),
        ino: metadata.ino(),
        len: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

#[cfg(windows)]
fn file_stamp(metadata: &Metadata) -> FileStamp {
    FileStamp {
        volume_serial_number: metadata.volume_serial_number().unwrap_or(0) as u64,
        file_index: metadata.file_index().unwrap_or(0),
        len: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

#[cfg(not(any(unix, windows)))]
fn file_stamp(metadata: &Metadata) -> FileStamp {
    FileStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

pub(crate) fn evict_database_paths_under(root: &Path) {
    OPENED_DBS
        .write()
        .unwrap()
        .retain(|path, _| !path.starts_with(root));
}
