//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
use rhai::EvalAltResult;
use std::path::Path;

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
    let bytes = std::fs::read(path)
        .map_err(|err| format!("Search read failed for {}: {err}", path.display()))?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}
