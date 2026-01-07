use leptos::prelude::*;
use leptos::html::Div;
use wasm_bindgen::prelude::*;
use crate::api::WsService;
use deve_core::protocol::ClientMessage;
use deve_core::models::DocId;

#[wasm_bindgen]
extern "C" {
    fn setupCodeMirror(element: &web_sys::HtmlElement, on_update: &Closure<dyn FnMut(String)>);
}

#[component]
pub fn Editor() -> impl IntoView {
    let editor_ref = NodeRef::<Div>::new();
    let ws = use_context::<WsService>().expect("WsService should be provided");
    
    // Local state of the document to compute diffs against
    // Initial value matches index.html default
    let (content, set_content) = signal("# Hello from CodeMirror\n\nStart typing...".to_string());

    // Mock DocId for Phase 2
    let doc_id = DocId::from_u128(1001);

    Effect::new(move |_| {
        if let Some(element) = editor_ref.get() {
            let raw_element: &web_sys::HtmlElement = &element;
            let ws_for_update = ws.clone();
            let on_update = Closure::wrap(Box::new(move |new_text: String| {
                let old_text = content.get_untracked();
                if new_text == old_text {
                    return;
                }
                
                // Compute Diff
                let ops = deve_core::state::compute_diff(&old_text, &new_text);
                
                // Send Ops
                if !ops.is_empty() {
                    leptos::logging::log!("Generated {} ops", ops.len());
                    for op in ops {
                        ws_for_update.send(ClientMessage::Edit { 
                            doc_id, 
                            op: op.clone() 
                        });
                    }
                }
                
                // Update local state
                set_content.set(new_text);
                
            }) as Box<dyn FnMut(String)>);

            setupCodeMirror(raw_element, &on_update);
            on_update.forget(); // Keep callback alive
        }
    });

    view! {
        <div 
            node_ref=editor_ref
            class="w-full h-full min-h-[500px] border border-gray-300 bg-white shadow-sm"
        >
        </div>
    }
}
