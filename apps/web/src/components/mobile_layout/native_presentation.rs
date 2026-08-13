//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!

use js_sys::Reflect;
use leptos::prelude::*;
use std::cell::Cell;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;

use super::gesture::{SystemGestureInsets, normalize_native_gesture_insets, window_width};

const ANDROID_PRESENTATION_EVENT: &str = "deve-native-presentation-change";
const ANDROID_PRESENTATION_GLOBAL: &str = "__DEVE_ANDROID_PRESENTATION__";
const ANDROID_PRESENTATION_PENDING_GLOBAL: &str = "__DEVE_ANDROID_PRESENTATION_PENDING__";
const MAX_SAFE_JS_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct PresentationOrder {
    generation: u64,
    epoch: u64,
}

pub(super) fn apply_android_presentation_insets(
    set_system_gesture_insets: WriteSignal<Option<SystemGestureInsets>>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let latest_order = std::rc::Rc::new(Cell::new(PresentationOrder::default()));
    let apply_order = latest_order.clone();
    let apply_detail: std::rc::Rc<dyn Fn(&JsValue) -> bool> =
        std::rc::Rc::new(move |detail: &JsValue| {
            let Some(order) = parse_presentation_order(detail) else {
                set_system_gesture_insets.update(|current| {
                    if current.as_ref().is_some_and(|value| value.is_native()) {
                        *current = None;
                    }
                });
                return false;
            };
            if order < apply_order.get() {
                return false;
            }
            apply_order.set(order);
            if js_string_field(detail, "kind").as_deref() == Some("system-gesture-insets-pending") {
                set_system_gesture_insets.set(None);
                return true;
            }
            let insets = window_width()
                .and_then(|viewport_width| parse_android_presentation(detail, viewport_width));
            set_system_gesture_insets.set(insets);
            insets.is_some()
        });

    let apply_event_detail = apply_detail.clone();
    let listener = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(event) = event.dyn_ref::<web_sys::CustomEvent>() else {
            return;
        };
        let detail = event.detail();
        if apply_event_detail(&detail) {
            let _ = Reflect::set(&detail, &JsValue::from_str("listenerSeen"), &JsValue::TRUE);
        }
    }) as Box<dyn FnMut(_)>);
    if window
        .add_event_listener_with_callback(
            ANDROID_PRESENTATION_EVENT,
            listener.as_ref().unchecked_ref(),
        )
        .is_err()
    {
        return;
    }

    let native_pending = Reflect::get(
        window.as_ref(),
        &JsValue::from_str(ANDROID_PRESENTATION_PENDING_GLOBAL),
    )
    .ok()
    .and_then(|value| value.as_bool())
    .unwrap_or(false);
    let initial = Reflect::get(
        window.as_ref(),
        &JsValue::from_str(ANDROID_PRESENTATION_GLOBAL),
    )
    .ok()
    .filter(|value| !value.is_null() && !value.is_undefined());
    set_system_gesture_insets.set(initial_presentation_fallback(
        native_pending || initial.is_some(),
    ));
    if let Some(initial) = initial {
        let _ = apply_detail(&initial);
    }

    let window_stored = StoredValue::new_local(window);
    let listener_stored = StoredValue::new_local(Some(listener));
    on_cleanup(move || {
        window_stored.with_value(|window| {
            listener_stored.with_value(|listener| {
                if let Some(listener) = listener {
                    let _ = window.remove_event_listener_with_callback(
                        ANDROID_PRESENTATION_EVENT,
                        listener.as_ref().unchecked_ref(),
                    );
                }
            });
        });
    });
}

fn initial_presentation_fallback(native_document_marker_seen: bool) -> Option<SystemGestureInsets> {
    (!native_document_marker_seen).then_some(SystemGestureInsets::web_default())
}

fn parse_android_presentation(
    detail: &JsValue,
    viewport_width: i32,
) -> Option<SystemGestureInsets> {
    if js_string_field(detail, "kind").as_deref() != Some("system-gesture-insets") {
        return None;
    }
    let generation = parse_presentation_order(detail)?.generation;
    normalize_native_gesture_insets(
        generation,
        js_number_field(detail, "widthPx")?,
        js_number_field(detail, "leftPx")?,
        js_number_field(detail, "rightPx")?,
        js_number_field(detail, "density")?,
        viewport_width,
    )
}

fn parse_presentation_order(detail: &JsValue) -> Option<PresentationOrder> {
    Some(PresentationOrder {
        generation: js_u64_field(detail, "generation", 1)?,
        epoch: js_u64_field(detail, "epoch", 1)?,
    })
}

fn js_u64_field(value: &JsValue, key: &str, minimum: u64) -> Option<u64> {
    let number = js_number_field(value, key)?;
    if number.fract() != 0.0 || !(minimum as f64..=MAX_SAFE_JS_INTEGER).contains(&number) {
        return None;
    }
    Some(number as u64)
}

fn js_number_field(value: &JsValue, key: &str) -> Option<f64> {
    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|field| field.as_f64())
}

fn js_string_field(value: &JsValue, key: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|field| field.as_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_drawer_edge_swipe_native_document_marker_fails_closed_and_web_defaults() {
        assert_eq!(initial_presentation_fallback(true), None);
        assert_eq!(
            initial_presentation_fallback(false),
            Some(SystemGestureInsets::web_default())
        );
    }

    #[test]
    fn mobile_drawer_edge_swipe_presentation_order_rejects_stale_epochs() {
        let current = PresentationOrder {
            generation: 3,
            epoch: 8,
        };
        assert!(
            PresentationOrder {
                generation: 3,
                epoch: 9,
            } > current
        );
        assert!(
            PresentationOrder {
                generation: 2,
                epoch: 99,
            } < current
        );
    }
}
