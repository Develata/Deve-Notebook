use std::path::Path;

use redb::Database;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub(crate) static OPENED_DBS: std::sync::LazyLock<
    RwLock<HashMap<std::path::PathBuf, Arc<Database>>>,
> = std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) fn evict_database_paths_under(root: &Path) {
    OPENED_DBS
        .write()
        .unwrap()
        .retain(|path, _| !path.starts_with(root));
}
