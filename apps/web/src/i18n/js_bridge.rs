// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-facade-contract
//!   - 13_i18n#i18n-resource-management

use super::Locale;
use super::extensions;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub fn publish_browser_i18n(locale: Locale) {
    publish_editor_widget_i18n(editor_widget_i18n(locale));
}

struct EditorWidgetI18n {
    locale: &'static str,
    copy_code: &'static str,
    more_actions: &'static str,
    no_actions_available: &'static str,
    mermaid_error: &'static str,
}

fn editor_widget_i18n(locale: Locale) -> EditorWidgetI18n {
    EditorWidgetI18n {
        locale: locale.as_bcp47(),
        copy_code: extensions::editor_copy_code(locale),
        more_actions: extensions::editor_more_actions(locale),
        no_actions_available: extensions::editor_no_actions_available(locale),
        mermaid_error: extensions::editor_mermaid_error(locale),
    }
}

#[cfg(target_arch = "wasm32")]
fn publish_editor_widget_i18n(copy: EditorWidgetI18n) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let editor = js_sys::Object::new();
    set_str(&editor, "copyCode", copy.copy_code);
    set_str(&editor, "moreActions", copy.more_actions);
    set_str(&editor, "noActionsAvailable", copy.no_actions_available);
    set_str(&editor, "mermaidError", copy.mermaid_error);

    let root = js_sys::Object::new();
    set_str(&root, "locale", copy.locale);
    let editor_value: JsValue = editor.into();
    let _ = js_sys::Reflect::set(root.as_ref(), &JsValue::from_str("editor"), &editor_value);

    let _ = js_sys::Reflect::set(
        window.as_ref(),
        &JsValue::from_str("deve_i18n"),
        root.as_ref(),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn publish_editor_widget_i18n(copy: EditorWidgetI18n) {
    let _ = (
        copy.locale,
        copy.copy_code,
        copy.more_actions,
        copy.no_actions_available,
        copy.mermaid_error,
    );
}

#[cfg(target_arch = "wasm32")]
fn set_str(object: &js_sys::Object, key: &str, value: &str) {
    let _ = js_sys::Reflect::set(
        object.as_ref(),
        &JsValue::from_str(key),
        &JsValue::from_str(value),
    );
}
