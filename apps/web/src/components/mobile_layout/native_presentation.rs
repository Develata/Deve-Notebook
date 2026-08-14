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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct NativeImePresentation {
    generation: u64,
    visible: bool,
    bottom_css_px: i32,
}

impl NativeImePresentation {
    pub(super) fn usable_offset(self) -> i32 {
        if self.visible {
            self.bottom_css_px.max(0)
        } else {
            0
        }
    }

    pub(super) fn generation(self) -> u64 {
        self.generation
    }

    pub(super) fn is_visible(self) -> bool {
        self.visible
    }

    #[cfg(test)]
    pub(super) fn from_generation_and_usable_offset_for_test(
        generation: u64,
        bottom_css_px: i32,
    ) -> Self {
        Self {
            generation,
            visible: bottom_css_px > 0,
            bottom_css_px: bottom_css_px.max(0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AndroidPresentation {
    gesture_insets: SystemGestureInsets,
    ime: NativeImePresentation,
}

pub(super) fn apply_android_presentation_insets(
    set_system_gesture_insets: WriteSignal<Option<SystemGestureInsets>>,
    set_native_ime_presentation: WriteSignal<Option<NativeImePresentation>>,
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
                set_native_ime_presentation.set(None);
                return false;
            };
            if order < apply_order.get() {
                return false;
            }
            apply_order.set(order);
            if js_string_field(detail, "kind").as_deref() == Some("system-gesture-insets-pending") {
                set_system_gesture_insets.set(None);
                set_native_ime_presentation.set(None);
                return true;
            }
            let presentation = window_width().and_then(|viewport_width| {
                parse_android_presentation(detail, order, viewport_width)
            });
            set_system_gesture_insets.set(presentation.map(|value| value.gesture_insets));
            set_native_ime_presentation.set(presentation.map(|value| value.ime));
            presentation.is_some()
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
    order: PresentationOrder,
    viewport_width: i32,
) -> Option<AndroidPresentation> {
    if js_string_field(detail, "kind").as_deref() != Some("system-gesture-insets") {
        return None;
    }
    let density = js_number_field(detail, "density")?;
    let gesture_insets = normalize_native_gesture_insets(
        order.generation,
        js_number_field(detail, "widthPx")?,
        js_number_field(detail, "leftPx")?,
        js_number_field(detail, "rightPx")?,
        density,
        viewport_width,
    )?;
    let ime = normalize_native_ime_presentation(
        order.generation,
        js_bool_field(detail, "imeVisible")?,
        js_number_field(detail, "imeBottomPx")?,
        js_number_field(detail, "heightPx")?,
        density,
    )?;
    Some(AndroidPresentation {
        gesture_insets,
        ime,
    })
}

fn normalize_native_ime_presentation(
    generation: u64,
    visible: bool,
    bottom_px: f64,
    height_px: f64,
    density: f64,
) -> Option<NativeImePresentation> {
    if !bottom_px.is_finite()
        || !height_px.is_finite()
        || !density.is_finite()
        || bottom_px < 0.0
        || height_px <= 0.0
        || bottom_px > height_px
        || density <= 0.0
    {
        return None;
    }
    let bottom_css_px = if visible && bottom_px > 1.0 {
        (bottom_px / density).round().clamp(0.0, i32::MAX as f64) as i32
    } else {
        0
    };
    Some(NativeImePresentation {
        generation,
        visible,
        bottom_css_px,
    })
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

fn js_bool_field(value: &JsValue, key: &str) -> Option<bool> {
    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|field| field.as_bool())
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

    #[test]
    fn mobile_toolbar_keyboard_native_ime_inset_normalizes_physical_pixels() {
        let presentation = normalize_native_ime_presentation(4, true, 929.0, 2400.0, 2.75)
            .expect("valid current-generation IME geometry");
        assert_eq!(presentation.usable_offset(), 338);
        assert_eq!(presentation.generation(), 4);
    }

    #[test]
    fn mobile_toolbar_keyboard_one_pixel_overlay_geometry_fails_closed() {
        let presentation = normalize_native_ime_presentation(4, true, 1.0, 2400.0, 2.75)
            .expect("one-pixel overlay remains a valid but unusable observation");
        assert_eq!(presentation.usable_offset(), 0);
        assert_eq!(
            normalize_native_ime_presentation(4, false, 929.0, 2400.0, 2.75)
                .expect("hidden IME geometry")
                .usable_offset(),
            0
        );
        assert!(normalize_native_ime_presentation(4, true, 2401.0, 2400.0, 2.75).is_none());
    }
}
