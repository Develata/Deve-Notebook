//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::DocId;
use leptos::prelude::{GetUntracked, Set};

pub fn reconcile_doc_selection(docs: &[(DocId, String)], signals: CoreSignals) {
    let query_doc_path = initial_doc_path_from_location();
    reconcile_doc_selection_for_query(docs, signals, query_doc_path.as_deref());
}

pub(super) fn reconcile_doc_selection_for_query(
    docs: &[(DocId, String)],
    signals: CoreSignals,
    query_doc_path: Option<&str>,
) {
    if let Some(selected) = signals.current_doc.get_untracked()
        && !docs.iter().any(|(doc_id, _)| *doc_id == selected)
    {
        leptos::logging::log!("清理过期 current_doc: {} 不在当前 DocList 中", selected);
        signals.set_current_doc.set(None);
    }
    if signals.current_doc.get_untracked().is_none()
        && let Some(pending_path) = signals.pending_created_doc_path.get_untracked()
        && let Some((doc_id, _)) = docs.iter().find(|(_, path)| *path == pending_path)
    {
        signals.set_current_doc.set(Some(*doc_id));
        signals.set_pending_created_doc_path.set(None);
    }
    if signals.current_doc.get_untracked().is_none()
        && let Some(query_path) = query_doc_path
        && let Some((doc_id, _)) = docs.iter().find(|(_, path)| path == query_path)
    {
        signals.set_current_doc.set(Some(*doc_id));
    }
}

#[cfg(target_arch = "wasm32")]
fn initial_doc_path_from_location() -> Option<String> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    query_doc_path_from_search(&search)
}

#[cfg(not(target_arch = "wasm32"))]
fn initial_doc_path_from_location() -> Option<String> {
    None
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn query_doc_path_from_search(search: &str) -> Option<String> {
    let query = search.strip_prefix('?').unwrap_or(search);
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key != "doc" {
            return None;
        }
        decode_query_component(value).filter(|path| !path.is_empty())
    })
}

#[cfg(any(target_arch = "wasm32", test))]
fn decode_query_component(value: &str) -> Option<String> {
    let src = value.as_bytes();
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        match src[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                if i + 2 >= src.len() {
                    return None;
                }
                let high = hex_value(src[i + 1])?;
                let low = hex_value(src[i + 2])?;
                out.push((high << 4) | low);
                i += 3;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(any(target_arch = "wasm32", test))]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
