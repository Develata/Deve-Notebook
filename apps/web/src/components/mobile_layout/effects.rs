// apps/web/src/components/mobile_layout/effects.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!

use js_sys::{Function, Reflect};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;

use super::keyboard_presentation::{
    KeyboardPresentationResolver, KeyboardPresentationSource, ViewportObservation,
};
use super::native_presentation::NativeImePresentation;

pub fn apply_body_scroll_lock(drawer_open: Signal<bool>) {
    Effect::new(move |_| {
        let open = drawer_open.get();
        if let Some(document) = web_sys::window().and_then(|w| w.document())
            && let Some(body) = document.body()
        {
            let _ = if open {
                body.style().set_property("overflow", "hidden")
            } else {
                body.style().set_property("overflow", "")
            };
        }
    });
}

pub fn apply_visual_viewport_offset(
    native_ime: ReadSignal<Option<NativeImePresentation>>,
    set_keyboard_offset: WriteSignal<i32>,
    set_keyboard_source: WriteSignal<KeyboardPresentationSource>,
) {
    let resolver = std::rc::Rc::new(std::cell::RefCell::new(
        KeyboardPresentationResolver::default(),
    ));
    let update_resolver = resolver.clone();
    let update_offset: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(move || {
        let viewport = web_sys::window().and_then(|window| {
            let viewport = Reflect::get(window.as_ref(), &JsValue::from_str("visualViewport"))
                .ok()
                .filter(|value| !value.is_null() && !value.is_undefined())?;
            let height = Reflect::get(&viewport, &JsValue::from_str("height"))
                .ok()
                .and_then(|value| value.as_f64())?;
            let offset_top = Reflect::get(&viewport, &JsValue::from_str("offsetTop"))
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or_default();
            let inner_height = window.inner_height().ok()?.as_f64()?;
            let width = window.inner_width().ok()?.as_f64()?;
            if !width.is_finite() || width <= 0.0 || width > i32::MAX as f64 {
                return None;
            }
            Some(ViewportObservation {
                width: width.round() as i32,
                inner_height,
                viewport_height: height,
                offset_top,
            })
        });
        let presentation = update_resolver
            .borrow_mut()
            .resolve(viewport, native_ime.get_untracked());
        set_keyboard_offset.set(presentation.offset);
        set_keyboard_source.set(presentation.source);
    });

    update_offset();

    let update_for_native = update_offset.clone();
    Effect::new(move |_| {
        let _ = native_ime.get();
        update_for_native();
    });

    if let Some(window) = web_sys::window() {
        let Ok(viewport) = Reflect::get(window.as_ref(), &JsValue::from_str("visualViewport"))
        else {
            return;
        };
        if viewport.is_null() || viewport.is_undefined() {
            return;
        }
        let Ok(add_listener) = Reflect::get(&viewport, &JsValue::from_str("addEventListener"))
        else {
            return;
        };
        let Ok(add_listener) = add_listener.dyn_into::<Function>() else {
            return;
        };

        let on_resize = update_offset.clone();
        let resize_cb =
            Closure::wrap(Box::new(move |_ev: JsValue| on_resize()) as Box<dyn FnMut(_)>);
        let _ = add_listener.call2(
            &viewport,
            &JsValue::from_str("resize"),
            resize_cb.as_ref().unchecked_ref(),
        );

        let on_scroll = update_offset.clone();
        let scroll_cb =
            Closure::wrap(Box::new(move |_ev: JsValue| on_scroll()) as Box<dyn FnMut(_)>);
        let _ = add_listener.call2(
            &viewport,
            &JsValue::from_str("scroll"),
            scroll_cb.as_ref().unchecked_ref(),
        );

        // 存储闭包和 viewport 引用，on_cleanup 时移除事件监听并释放内存
        let viewport_stored = StoredValue::new_local(Some(viewport));
        let resize_stored = StoredValue::new_local(Some(resize_cb));
        let scroll_stored = StoredValue::new_local(Some(scroll_cb));

        on_cleanup(move || {
            if let Some(vp) = viewport_stored.try_get_value().flatten()
                && let Ok(remove_fn) = Reflect::get(&vp, &JsValue::from_str("removeEventListener"))
                && let Ok(remove_fn) = remove_fn.dyn_into::<Function>()
            {
                resize_stored.with_value(|cb_opt| {
                    if let Some(cb) = cb_opt {
                        let _ = remove_fn.call2(
                            &vp,
                            &JsValue::from_str("resize"),
                            cb.as_ref().unchecked_ref(),
                        );
                    }
                });
                scroll_stored.with_value(|cb_opt| {
                    if let Some(cb) = cb_opt {
                        let _ = remove_fn.call2(
                            &vp,
                            &JsValue::from_str("scroll"),
                            cb.as_ref().unchecked_ref(),
                        );
                    }
                });
            }
            // StoredValue 是 Copy，离开作用域后自动释放内部 Closure
            let _ = resize_stored;
            let _ = scroll_stored;
            let _ = viewport_stored;
        });
    }
}
