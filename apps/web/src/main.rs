mod app;
mod editor;
mod api;
mod components;
mod i18n;
use app::App;
use leptos::prelude::*;

pub fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    tracing::info!("Initializing Deve-Note Web App");
    
    // Hide overlay manually on mount to prevent hanging if no doc selected
    let window = web_sys::window().unwrap();
    let doc = window.document().unwrap();
    if let Some(el) = doc.get_element_by_id("loading-overlay") {
        let _ = el.class_list().add_1("hidden");
    }

    mount_to_body(|| {
        view! { <App/> }
    })
}
