use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Hook to synchronize scrolling between two containers
pub fn use_scroll_sync(left_container: NodeRef<html::Div>, right_container: NodeRef<html::Div>) {
    // Shared state to prevent infinite scroll loops
    let is_syncing_left = std::rc::Rc::new(std::cell::Cell::new(false));
    let is_syncing_right = std::rc::Rc::new(std::cell::Cell::new(false));

    let is_syncing_left_clone = is_syncing_left.clone();
    let is_syncing_right_clone = is_syncing_right.clone();

    Effect::new(move |_| {
        let left_div = left_container.get();
        let right_div = right_container.get();

        if let (Some(left), Some(right)) = (left_div, right_div) {
            let left_el: web_sys::HtmlElement = left.into();
            let right_el: web_sys::HtmlElement = right.into();

            let left_el_1 = left_el.clone();
            let right_el_1 = right_el.clone();
            let is_l = is_syncing_left_clone.clone();
            let is_r = is_syncing_right_clone.clone();

            // Left -> Right
            let on_scroll_left = Closure::wrap(Box::new(move |_| {
                if !is_l.get() {
                    is_r.set(true);
                    right_el_1.set_scroll_top(left_el_1.scroll_top());
                    is_r.set(false);
                }
            }) as Box<dyn FnMut(web_sys::Event)>);

            let _ = left_el.add_event_listener_with_callback(
                "scroll",
                on_scroll_left.as_ref().unchecked_ref(),
            );
            on_scroll_left.forget();

            let left_el_2 = left_el.clone();
            let right_el_2 = right_el.clone();
            let is_l = is_syncing_left.clone();
            let is_r = is_syncing_right.clone();

            // Right -> Left
            let on_scroll_right = Closure::wrap(Box::new(move |_| {
                if !is_r.get() {
                    is_l.set(true);
                    left_el_2.set_scroll_top(right_el_2.scroll_top());
                    is_l.set(false);
                }
            }) as Box<dyn FnMut(web_sys::Event)>);

            let _ = right_el.add_event_listener_with_callback(
                "scroll",
                on_scroll_right.as_ref().unchecked_ref(),
            );
            on_scroll_right.forget();
        }
    });
}
