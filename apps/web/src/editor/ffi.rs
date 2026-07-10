// apps/web/src/editor/ffi.rs
//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! # FFI Bindings (JavaScript 互操作)
//!
//! 定义与 JavaScript (CodeMirror adapter) 交互的外部函数接口。
//!
//! ## 性能优化 (v4)
//! - `setupCodeMirror` 现在接收 Delta 回调 (JSON 字符串)，而不是全文回调
//! - 避免了每次按键时的 JS->WASM 全文拷贝
//! - 添加了 `destroyEditor` 用于清理资源

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

/// 初始化 CodeMirror 编辑器
///
/// `on_delta`: 接收 JSON 格式的 Delta 数组: `[{from, to, insert}, ...]`
#[allow(non_snake_case)]
pub fn setupCodeMirror(
    element: &web_sys::HtmlElement,
    on_delta: &Closure<dyn FnMut(String)>,
    on_ready: &Closure<dyn FnMut()>,
) -> bool {
    bridge_call3(
        "setupCodeMirror",
        element.as_ref(),
        on_delta.as_ref(),
        on_ready.as_ref(),
    )
    .and_then(|value| value.as_bool())
    .unwrap_or(false)
}

/// 销毁编辑器实例，释放资源
#[allow(non_snake_case)]
pub fn destroyEditor() -> bool {
    bridge_call0("destroyEditor").is_some()
}

/// 应用远程快照 (全量替换)
#[allow(non_snake_case)]
pub fn applyRemoteContent(text: &str) -> bool {
    bridge_call1("applyRemoteContent", &JsValue::from_str(text))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub fn try_apply_remote_op(op_json: &str) -> bool {
    bridge_call1("applyRemoteOp", &JsValue::from_str(op_json))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub fn try_get_editor_content() -> Option<String> {
    bridge_call0("getEditorContent").and_then(|value| value.as_string())
}

/// 滚动到指定行
pub fn scroll_global(line: usize) {
    let _ = bridge_call1("scrollGlobal", &JsValue::from_f64(line as f64));
}

/// 设置只读状态
pub fn set_read_only(read_only: bool) -> bool {
    bridge_call1("setReadOnly", &JsValue::from_bool(read_only))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

#[allow(non_snake_case)]
pub fn applyRemoteOpsBatch(ops_json: &str) -> bool {
    bridge_call1("applyRemoteOpsBatch", &JsValue::from_str(ops_json))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub fn sync_editor_state_to_rust() -> bool {
    bridge_call0("syncEditorStateToRust")
        .map(|value| value.as_bool().unwrap_or(true))
        .unwrap_or(false)
}

pub fn mobile_insert_text(text: &str) {
    let _ = bridge_call1("mobileInsertText", &JsValue::from_str(text));
}

pub fn mobile_wrap_selection(prefix: &str, suffix: &str) {
    let prefix = JsValue::from_str(prefix);
    let suffix = JsValue::from_str(suffix);
    let _ = bridge_call2("mobileWrapSelection", &prefix, &suffix);
}

pub fn mobile_undo() {
    let _ = bridge_call0("mobileUndo");
}

pub fn mobile_redo() {
    let _ = bridge_call0("mobileRedo");
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn update_gutter_diff(ranges_json: &str) -> bool {
    bridge_call1("updateGutterDiff", &JsValue::from_str(ranges_json)).is_some()
}

pub fn try_get_editor_selection() -> Option<String> {
    bridge_call0("getEditorSelection").and_then(|value| value.as_string())
}

fn bridge_call0(name: &str) -> Option<JsValue> {
    let (bridge, func) = bridge_call_function()?;
    func.call1(&bridge, &JsValue::from_str(name)).ok()
}

fn bridge_call1(name: &str, arg: &JsValue) -> Option<JsValue> {
    let (bridge, func) = bridge_call_function()?;
    func.call2(&bridge, &JsValue::from_str(name), arg).ok()
}

fn bridge_call2(name: &str, first: &JsValue, second: &JsValue) -> Option<JsValue> {
    let (bridge, func) = bridge_call_function()?;
    func.call3(&bridge, &JsValue::from_str(name), first, second)
        .ok()
}

fn bridge_call3(name: &str, first: &JsValue, second: &JsValue, third: &JsValue) -> Option<JsValue> {
    let (bridge, func) = bridge_call_function()?;
    func.call4(&bridge, &JsValue::from_str(name), first, second, third)
        .ok()
}

fn bridge_call_function() -> Option<(JsValue, js_sys::Function)> {
    let window = web_sys::window()?;
    let bridge = js_sys::Reflect::get(window.as_ref(), &"__deveWebBridge".into()).ok()?;
    let call = js_sys::Reflect::get(&bridge, &"call".into()).ok()?;
    let func = call.dyn_into::<js_sys::Function>().ok()?;
    Some((bridge, func))
}

#[cfg(test)]
mod tests {
    fn source_before_tests() -> &'static str {
        include_str!("ffi.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("source before tests")
    }

    #[test]
    fn editor_browser_calls_read_through_bridge_registry() {
        let source = source_before_tests();

        assert!(!source.contains("unsafe extern \"C\""));
        assert!(source.contains("\"__deveWebBridge\""));
        assert!(source.contains("\"call\""));
        assert!(source.contains("bridge_call3("));
        assert!(source.contains("\"setupCodeMirror\""));
        assert!(source.contains("bridge_call0(\"destroyEditor\""));
        assert!(source.contains("bridge_call1(\"applyRemoteContent\""));
        assert!(source.contains("bridge_call1(\"applyRemoteOp\""));
        assert!(source.contains("bridge_call0(\"getEditorContent\""));
        assert!(source.contains("bridge_call1(\"scrollGlobal\""));
        assert!(source.contains("bridge_call1(\"setReadOnly\""));
        assert!(source.contains("bridge_call1(\"applyRemoteOpsBatch\""));
        assert!(source.contains("bridge_call0(\"syncEditorStateToRust\""));
        assert!(source.contains("bridge_call1(\"mobileInsertText\""));
        assert!(source.contains("bridge_call2(\"mobileWrapSelection\""));
        assert!(source.contains("fn bridge_call3("));
        assert!(source.contains("bridge_call0(\"mobileUndo\""));
        assert!(source.contains("bridge_call0(\"mobileRedo\""));
        assert!(source.contains("bridge_call1(\"updateGutterDiff\""));
        assert!(source.contains("bridge_call0(\"getEditorSelection\""));
        for name in [
            "setupCodeMirror",
            "destroyEditor",
            "applyRemoteContent",
            "applyRemoteOp",
            "getEditorContent",
            "scrollGlobal",
            "setReadOnly",
            "applyRemoteOpsBatch",
            "syncEditorStateToRust",
            "mobileInsertText",
            "mobileWrapSelection",
            "mobileUndo",
            "mobileRedo",
            "updateGutterDiff",
            "getEditorSelection",
        ] {
            assert!(!source.contains(&format!("js_namespace = window, js_name = {name}")));
            assert!(!source.contains(&format!("js_name = {name}")));
            let direct_window_lookup =
                ["Reflect::get(window.as_ref(), &\"", name, "\".into())"].join("");
            assert!(!source.contains(&direct_window_lookup));
        }
        assert!(!source.contains("pub fn get_editor_selection()"));
    }

    #[test]
    fn editor_selection_reads_through_bridge_registry() {
        let source = source_before_tests();

        assert!(source.contains("pub fn try_get_editor_selection() -> Option<String>"));
        assert!(source.contains("bridge_call0(\"getEditorSelection\")"));
        assert!(source.contains("fn bridge_call0(name: &str) -> Option<JsValue>"));
        assert!(source.contains("let (bridge, func) = bridge_call_function()?;"));
        assert!(source.contains("JsValue::from_str(name)"));
        assert!(source.contains("\"__deveWebBridge\""));
        assert!(source.contains("\"call\""));
        assert!(!source.contains("Reflect::get(window.as_ref(), &\"getEditorSelection\".into())"));
        assert!(!source.contains("js_name = getEditorSelection"));
    }

    #[test]
    fn editor_ffi_unready_write_queries_fail_closed() {
        let source = source_before_tests();

        assert!(
            source.contains("fn bridge_call_function() -> Option<(JsValue, js_sys::Function)>")
        );
        assert!(source.contains("let window = web_sys::window()?;"));
        assert!(source.contains(
            "let bridge = js_sys::Reflect::get(window.as_ref(), &\"__deveWebBridge\".into()).ok()?;"
        ));
        assert!(
            source.contains("let call = js_sys::Reflect::get(&bridge, &\"call\".into()).ok()?;")
        );
        assert!(source.contains("let func = call.dyn_into::<js_sys::Function>().ok()?;"));
        assert!(source.matches(".unwrap_or(false)").count() >= 5);
        assert!(source.contains("pub fn try_get_editor_content() -> Option<String>"));
        assert!(source.contains("pub fn try_get_editor_selection() -> Option<String>"));
        assert!(!source.contains(".expect(\"window\")"));
        assert!(!source.contains(".unwrap()"));
    }
}
