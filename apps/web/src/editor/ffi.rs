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

#[wasm_bindgen]
unsafe extern "C" {
    /// 初始化 CodeMirror 编辑器
    ///
    /// `on_delta`: 接收 JSON 格式的 Delta 数组: `[{from, to, insert}, ...]`
    pub fn setupCodeMirror(element: &web_sys::HtmlElement, on_delta: &Closure<dyn FnMut(String)>);

    /// 销毁编辑器实例，释放资源
    pub fn destroyEditor();

    /// 应用远程快照 (全量替换)
    pub fn applyRemoteContent(text: &str);

    /// 应用远程操作 (增量)
    pub fn applyRemoteOp(op_json: &str);

    /// 批量应用远程操作 (增量)
    #[wasm_bindgen(js_namespace = window, js_name = applyRemoteOpsBatch)]
    pub fn applyRemoteOpsBatch(ops_json: &str) -> bool;

    /// 将当前编辑器全文状态同步回 Rust signals，不产生新的写入 delta
    #[wasm_bindgen(js_namespace = window, js_name = syncEditorStateToRust)]
    pub fn sync_editor_state_to_rust();

    /// 获取当前编辑器内容
    pub fn getEditorContent() -> String;

    /// 滚动到指定行
    #[wasm_bindgen(js_name = scrollGlobal)]
    pub fn scroll_global(line: usize);

    /// 设置只读状态
    #[wasm_bindgen(js_name = setReadOnly)]
    pub fn set_read_only(read_only: bool);

    /// Mobile: 在光标处插入文本
    #[wasm_bindgen(js_namespace = window, js_name = mobileInsertText)]
    pub fn mobile_insert_text(text: &str);

    /// Mobile: 包裹当前选区
    #[wasm_bindgen(js_namespace = window, js_name = mobileWrapSelection)]
    pub fn mobile_wrap_selection(prefix: &str, suffix: &str);

    /// Mobile: 撤销一步
    #[wasm_bindgen(js_namespace = window, js_name = mobileUndo)]
    pub fn mobile_undo();

    /// 更新行内 Diff Gutter 指示器
    #[wasm_bindgen(js_namespace = window, js_name = updateGutterDiff)]
    pub fn update_gutter_diff(ranges_json: &str);

    /// 获取当前编辑器选区信息 (JSON)
    ///
    /// 返回 `"null"` 表示无选区; 否则返回 `{from, to, text}` JSON
    #[wasm_bindgen(js_namespace = window, js_name = getEditorSelection)]
    pub fn get_editor_selection() -> String;
}

pub fn try_get_editor_selection() -> Option<String> {
    let window = web_sys::window()?;
    let value = js_sys::Reflect::get(window.as_ref(), &"getEditorSelection".into()).ok()?;
    let func = value.dyn_ref::<js_sys::Function>()?;
    func.call0(window.as_ref()).ok()?.as_string()
}
