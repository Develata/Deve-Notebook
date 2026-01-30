// apps/web/src/components/chat/drop_handler.rs
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

pub fn on_drag_over(set_is_drag_over: WriteSignal<bool>) -> impl Fn(web_sys::DragEvent) {
    move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(true);
    }
}

pub fn on_drag_leave(set_is_drag_over: WriteSignal<bool>) -> impl Fn(web_sys::DragEvent) {
    move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(false);
    }
}

pub fn on_drop(
    set_input: WriteSignal<String>,
    set_is_drag_over: WriteSignal<bool>,
) -> impl Fn(web_sys::DragEvent) {
    move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(false);

        if let Some(data_transfer) = ev.data_transfer()
            && let Some(files) = data_transfer.files()
        {
            for i in 0..files.length() {
                if let Some(file) = files.item(i) {
                    let name = file.name();
                    if file.size() > 1024.0 * 1024.0 {
                        leptos::logging::warn!("File too large: {}", name);
                        continue;
                    }

                    let reader = web_sys::FileReader::new().unwrap();
                    let reader_c = reader.clone();
                    let name_c = name.clone();
                    let set_input = set_input;

                    let onload = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                        if let Ok(content) = reader_c
                            .result()
                            .and_then(|r| r.as_string().ok_or(wasm_bindgen::JsValue::NULL))
                        {
                            set_input.update(|curr| {
                                let block = format!("\n```{}\n{}\n```\n", name_c, content);
                                curr.push_str(&block);
                            });
                        }
                    }) as Box<dyn FnMut(_)>);

                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    onload.forget();
                    let _ = reader.read_as_text(&file);
                }
            }
        }
    }
}
