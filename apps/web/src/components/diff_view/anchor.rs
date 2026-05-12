//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq)]
pub struct ScrollAnchor {
    pub key: String,
    pub offset_top: i32,
}

pub fn anchor_scroll_delta(current_offset_top: i32, anchor: &ScrollAnchor) -> i32 {
    current_offset_top - anchor.offset_top
}

fn anchor_candidates(container: &web_sys::HtmlElement) -> Vec<web_sys::HtmlElement> {
    let Ok(list) = container.query_selector_all("[data-anchor-key]") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..list.length() {
        if let Some(node) = list.item(i)
            && let Ok(el) = node.dyn_into::<web_sys::HtmlElement>()
        {
            out.push(el);
        }
    }
    out
}

pub fn capture_anchor(container: &web_sys::HtmlElement) -> Option<ScrollAnchor> {
    let crect = container.get_bounding_client_rect();
    let ctop = crect.top();
    for el in anchor_candidates(container) {
        let rect = el.get_bounding_client_rect();
        if rect.bottom() >= ctop
            && let Some(key) = el.get_attribute("data-anchor-key")
        {
            return Some(ScrollAnchor {
                key,
                offset_top: (rect.top() - ctop) as i32,
            });
        }
    }
    None
}

pub fn restore_anchor(container: &web_sys::HtmlElement, anchor: &ScrollAnchor) -> bool {
    let crect = container.get_bounding_client_rect();
    let ctop = crect.top();
    for el in anchor_candidates(container) {
        if el
            .get_attribute("data-anchor-key")
            .is_some_and(|k| k == anchor.key)
        {
            let top = el.get_bounding_client_rect().top();
            let delta = anchor_scroll_delta((top - ctop) as i32, anchor);
            container.set_scroll_top(container.scroll_top() + delta);
            return true;
        }
    }
    false
}

pub fn active_container(
    force_unified: bool,
    unified_ref: NodeRef<html::Div>,
    left_ref: NodeRef<html::Div>,
) -> Option<web_sys::HtmlElement> {
    if force_unified {
        return unified_ref
            .get()
            .map(|v| v.unchecked_into::<web_sys::HtmlElement>());
    }
    left_ref
        .get()
        .map(|v| v.unchecked_into::<web_sys::HtmlElement>())
}

#[cfg(test)]
mod tests {
    use super::{ScrollAnchor, anchor_scroll_delta};

    #[test]
    fn diff_semantic_anchor_delta_preserves_original_offset() {
        let anchor = ScrollAnchor {
            key: "row-42".to_string(),
            offset_top: 80,
        };

        assert_eq!(anchor_scroll_delta(120, &anchor), 40);
        assert_eq!(anchor_scroll_delta(40, &anchor), -40);
    }
}
