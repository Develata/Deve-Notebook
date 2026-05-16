// apps\web\src\i18n
//! plan_ref:
//!   - 11_i18n#i18n-facade-contract
//!   - 11_i18n#i18n-resource-management

use super::Locale;

#[cfg(target_arch = "wasm32")]
use super::extensions;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub fn publish_browser_i18n(locale: Locale) {
    publish_editor_widget_i18n(locale);
}

#[cfg(target_arch = "wasm32")]
fn publish_editor_widget_i18n(locale: Locale) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let editor = js_sys::Object::new();
    set_str(&editor, "copyCode", extensions::editor_copy_code(locale));
    set_str(
        &editor,
        "moreActions",
        extensions::editor_more_actions(locale),
    );
    set_str(
        &editor,
        "noActionsAvailable",
        extensions::editor_no_actions_available(locale),
    );
    set_str(
        &editor,
        "mermaidError",
        extensions::editor_mermaid_error(locale),
    );

    let root = js_sys::Object::new();
    set_str(&root, "locale", locale.as_bcp47());
    let editor_value: JsValue = editor.into();
    let _ = js_sys::Reflect::set(root.as_ref(), &JsValue::from_str("editor"), &editor_value);

    let _ = js_sys::Reflect::set(
        window.as_ref(),
        &JsValue::from_str("deve_i18n"),
        root.as_ref(),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn publish_editor_widget_i18n(_locale: Locale) {}

#[cfg(target_arch = "wasm32")]
fn set_str(object: &js_sys::Object, key: &str, value: &str) {
    let _ = js_sys::Reflect::set(
        object.as_ref(),
        &JsValue::from_str(key),
        &JsValue::from_str(value),
    );
}
