use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;

#[wasm_bindgen]
unsafe extern "C" {
    pub fn setupCodeMirror(element: &web_sys::HtmlElement, on_update: &Closure<dyn FnMut(String)>);
    
    pub fn applyRemoteContent(text: &str);
    pub fn applyRemoteOp(op_json: &str);
    pub fn getEditorContent() -> String;
    
    
    #[wasm_bindgen(js_name = scrollGlobal)]
    pub fn scroll_global(line: usize);

    #[wasm_bindgen(js_name = setReadOnly)]
    pub fn set_read_only(read_only: bool);
}
