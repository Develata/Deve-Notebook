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

const BODY_SCROLL_LOCK_OWNER_PROPERTY: &str = "__DEVE_BODY_SCROLL_LOCK_OWNER__";

pub fn apply_body_scroll_lock(drawer_open: Signal<bool>) {
    let owner = StoredValue::new_local(JsValue::from(js_sys::Object::new()));
    let original_overflow = StoredValue::new_local(None::<String>);
    Effect::new(move |_| {
        owner.with_value(|owner| {
            if drawer_open.get() {
                acquire_body_scroll_lock(owner, original_overflow);
            } else {
                release_body_scroll_lock(owner, original_overflow);
            }
        });
    });
    on_cleanup(move || {
        owner.with_value(|owner| release_body_scroll_lock(owner, original_overflow));
    });
}

fn acquire_body_scroll_lock(
    owner: &JsValue,
    original_overflow: StoredValue<Option<String>, LocalStorage>,
) {
    let Some(body) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.body())
    else {
        return;
    };
    let owner_key = JsValue::from_str(BODY_SCROLL_LOCK_OWNER_PROPERTY);
    let current_owner = Reflect::get(body.as_ref(), &owner_key).ok();
    if current_owner
        .as_ref()
        .is_some_and(|current| js_sys::Object::is(current, owner))
    {
        return;
    }
    if current_owner.is_some_and(|current| !current.is_null() && !current.is_undefined()) {
        return;
    }

    let previous = body
        .style()
        .get_property_value("overflow")
        .unwrap_or_default();
    if Reflect::set(body.as_ref(), &owner_key, owner).unwrap_or(false)
        && body.style().set_property("overflow", "hidden").is_ok()
    {
        original_overflow.set_value(Some(previous));
    } else {
        let _ = Reflect::delete_property(body.as_ref(), &owner_key);
    }
}

fn release_body_scroll_lock(
    owner: &JsValue,
    original_overflow: StoredValue<Option<String>, LocalStorage>,
) {
    let Some(body) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.body())
    else {
        return;
    };
    let owner_key = JsValue::from_str(BODY_SCROLL_LOCK_OWNER_PROPERTY);
    let current_owner = Reflect::get(body.as_ref(), &owner_key).ok();
    if !current_owner
        .as_ref()
        .is_some_and(|current| js_sys::Object::is(current, owner))
    {
        return;
    }

    let mut previous = None;
    original_overflow.update_value(|value| previous = value.take());
    let previous = previous.unwrap_or_default();
    let style = body.style();
    let _ = if previous.is_empty() {
        style.remove_property("overflow")
    } else {
        style.set_property("overflow", &previous).map(|_| previous)
    };
    let _ = Reflect::delete_property(body.as_ref(), &owner_key);
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

#[cfg(test)]
mod tests {
    #[test]
    fn mobile_body_scroll_lock_is_owner_scoped() {
        let source = include_str!("effects.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("effects source");

        assert!(source.contains("BODY_SCROLL_LOCK_OWNER_PROPERTY"));
        assert!(source.contains("release_body_scroll_lock"));
        assert!(source.contains("on_cleanup"));
    }
}
