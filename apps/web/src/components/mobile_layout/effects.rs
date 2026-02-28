// apps/web/src/components/mobile_layout/effects.rs

use js_sys::{Function, Reflect};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;

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

pub fn apply_visual_viewport_offset(set_keyboard_offset: WriteSignal<i32>) {
    let update_offset: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(move || {
        let Some(window) = web_sys::window() else {
            set_keyboard_offset.set(0);
            return;
        };
        let viewport = match Reflect::get(window.as_ref(), &JsValue::from_str("visualViewport")) {
            Ok(v) if !v.is_null() && !v.is_undefined() => v,
            _ => {
                set_keyboard_offset.set(0);
                return;
            }
        };
        let height = Reflect::get(&viewport, &JsValue::from_str("height"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let offset_top = Reflect::get(&viewport, &JsValue::from_str("offsetTop"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if height <= 0.0 {
            set_keyboard_offset.set(0);
            return;
        }
        let inner_h = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let overlap = (inner_h - (height + offset_top)).max(0.0);
        set_keyboard_offset.set(overlap.round() as i32);
    });

    update_offset();

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
            if let Some(vp) = viewport_stored.try_get_value().flatten() {
                if let Ok(remove_fn) =
                    Reflect::get(&vp, &JsValue::from_str("removeEventListener"))
                {
                    if let Ok(remove_fn) = remove_fn.dyn_into::<Function>() {
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
                }
            }
            // StoredValue 是 Copy，离开作用域后自动释放内部 Closure
            let _ = resize_stored;
            let _ = scroll_stored;
            let _ = viewport_stored;
        });
    }
}
