use leptos::prelude::*;
use leptos::html::Div;
use wasm_bindgen::prelude::*;
use crate::api::WsService;
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::models::DocId;

#[wasm_bindgen]
extern "C" {
    fn setupCodeMirror(element: &web_sys::HtmlElement, on_update: &Closure<dyn FnMut(String)>);
    fn applyRemoteContent(text: &str);
    fn applyRemoteOp(op_json: &str);
    fn getEditorContent() -> String;
}

#[component]
pub fn Editor() -> impl IntoView {
    let editor_ref = NodeRef::<Div>::new();
    let ws = use_context::<WsService>().expect("WsService should be provided");
    
    // Local state of the document to compute diffs against
    let (content, set_content) = signal("# Hello from CodeMirror\n\nStart typing...".to_string());
    let (local_version, set_local_version) = signal(0u64);
    
    // Generate a session client_id (using random rough)
    let client_id = (js_sys::Math::random() * 1_000_000.0) as u64;
    
    // Mock DocId for Phase 2/3
    let doc_id = DocId::from_u128(1001);

    // Effect to handle incoming messages
    Effect::new(move |_| {
         if let Some(msg) = ws.msg.get() {
             match msg {
                 ServerMessage::Snapshot { doc_id: _, content: new_content, version } => {
                     leptos::logging::log!("Received Snapshot: {} chars, Ver: {}", new_content.len(), version);
                     applyRemoteContent(&new_content);
                     set_content.set(new_content);
                     set_local_version.set(version);
                 }
                 ServerMessage::NewOp { doc_id: _, op, seq, client_id: origin_id } => {
                     let current_ver = local_version.get_untracked();
                     if seq > current_ver {
                         // Filter Echoes!
                         if origin_id != client_id {
                             // Verified Remote Op from SOMEONE ELSE
                             // leptos::logging::log!("Applying Op Seq {}", seq);
                             if let Ok(json) = serde_json::to_string(&op) {
                                 applyRemoteOp(&json);
                             }
                             // Sync Ref
                             set_content.set(getEditorContent());
                         } else {
                             // This is our own echo. We already applied it locally.
                             // Just update the version.
                             // leptos::logging::log!("Ack Own Op Seq {}", seq);
                         }
                         set_local_version.set(seq);
                     }
                 }
                 _ => {}
             }
         }
    });

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
                    for op in ops {
                        ws_for_update.send(ClientMessage::Edit { 
                            doc_id, 
                            op: op.clone(),
                            client_id 
                        });
                    }
                }
                
                // Update local state
                set_content.set(new_text);
                
            }) as Box<dyn FnMut(String)>);

            setupCodeMirror(raw_element, &on_update);
            on_update.forget(); 
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
