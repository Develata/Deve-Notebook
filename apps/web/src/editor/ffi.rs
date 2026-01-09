use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub fn setupCodeMirror(element: &web_sys::HtmlElement, on_update: &Closure<dyn FnMut(String)>);
    pub fn applyRemoteContent(text: &str);
    pub fn applyRemoteOp(op_json: &str);
    pub fn getEditorContent() -> String;
}
