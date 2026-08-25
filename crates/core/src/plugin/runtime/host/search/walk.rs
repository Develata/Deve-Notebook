//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use crate::plugin::resource_budget::{MAX_PLUGIN_HOST_TEXT_BYTES, read_utf8_file_bounded};
use rhai::EvalAltResult;
use std::path::Path;

pub(super) fn is_regular_walk_file(entry: &ignore::DirEntry) -> bool {
    entry
        .file_type()
        .is_some_and(|file_type| file_type.is_file())
}

pub(super) fn next_walk_entry(
    entry: Result<ignore::DirEntry, ignore::Error>,
    root: &Path,
    context: &str,
) -> Result<Option<ignore::DirEntry>, Box<EvalAltResult>> {
    match entry {
        Ok(entry) => Ok(Some(entry)),
        Err(err) => Err(format!("{context} failed under {}: {err}", root.display()).into()),
    }
}

pub(super) fn read_searchable_text(path: &Path) -> Result<Option<String>, Box<EvalAltResult>> {
    match read_utf8_file_bounded(path, MAX_PLUGIN_HOST_TEXT_BYTES, "plugin grep candidate") {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => Ok(None),
        Err(error) => Err(format!("Search read failed for {}: {error}", path.display()).into()),
    }
}
