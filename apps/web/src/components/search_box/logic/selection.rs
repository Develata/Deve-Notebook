use crate::components::search_box::types::{SearchAction, SearchResult};
use leptos::prelude::*;

pub fn make_active_index(
    selected_index: Signal<usize>,
    providers_results: Memo<Vec<SearchResult>>,
) -> impl Fn() -> usize + Copy + Send + Sync + 'static {
    move || {
        let count = providers_results.get().len();
        if count == 0 {
            return 0;
        }
        let current = selected_index.get();
        let res = providers_results.get();
        if current >= count {
            return first_selectable(&res).unwrap_or(0);
        }
        if is_selectable(res.get(current)) {
            current
        } else {
            first_selectable(&res).unwrap_or(0)
        }
    }
}

pub(crate) fn next_selectable_index(results: &[SearchResult], current: usize, dir: i32) -> usize {
    if results.is_empty() {
        return 0;
    }
    let mut idx = current as i32;
    for _ in 0..results.len() {
        idx = (idx + dir).rem_euclid(results.len() as i32);
        if results[idx as usize].action != SearchAction::Noop {
            return idx as usize;
        }
    }
    current
}

pub(crate) fn first_selectable(results: &[SearchResult]) -> Option<usize> {
    results
        .iter()
        .position(|res| res.action != SearchAction::Noop)
}

pub(crate) fn is_selectable(result: Option<&SearchResult>) -> bool {
    matches!(result, Some(r) if r.action != SearchAction::Noop)
}
