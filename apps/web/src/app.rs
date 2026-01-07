use crate::editor::Editor;
use crate::api::WsService;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    let ws = WsService::new();
    provide_context(ws.clone());
    let status_text = move || format!("{:?}", ws.status.get());

    view! {
        <div class="h-screen w-screen flex flex-col items-center justify-center bg-gray-50">
            <h1 class="text-3xl font-bold text-gray-800 mb-8">
                "Deve-Note Cockpit"
            </h1>
            <div class="w-full max-w-4xl h-[600px]">
                <Editor/>
            </div>
            <p class="mt-4 text-sm text-gray-400">
                "Phase 1: MVC Prototype | WS: " {status_text}
            </p>
        </div>
    }
}
