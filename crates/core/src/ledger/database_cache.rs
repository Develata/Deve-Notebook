use std::path::Path;
#[cfg(not(any(unix, windows)))]
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
    #[cfg(not(any(unix, windows)))]
    len: u64,
    #[cfg(not(any(unix, windows)))]
    modified: Option<SystemTime>,
}

pub(crate) static OPENED_DBS: std::sync::LazyLock<
    RwLock<HashMap<std::path::PathBuf, CachedDatabaseEntry>>,
> = std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) fn current_file_stamp(path: &Path) -> Option<FileStamp> {
    std::fs::metadata(path)
        .ok()
        .map(|metadata| file_stamp(&metadata))
}

#[cfg(unix)]
fn file_stamp(metadata: &Metadata) -> FileStamp {
    FileStamp {
        dev: metadata.dev(),
        ino: metadata.ino(),
    }
}

#[cfg(windows)]
fn file_stamp(metadata: &Metadata) -> FileStamp {
    FileStamp {
        volume_serial_number: metadata.volume_serial_number().unwrap_or(0) as u64,
        file_index: metadata.file_index().unwrap_or(0),
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
