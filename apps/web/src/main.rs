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
    
    mount_to_body(|| {
        view! { <App/> }
    })
}
