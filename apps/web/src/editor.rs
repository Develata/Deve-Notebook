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
pub fn Editor(
    doc_id: DocId,
) -> impl IntoView {
    let editor_ref = NodeRef::<Div>::new();
    let ws = use_context::<WsService>().expect("WsService should be provided");
    
    // Local state of the document to compute diffs against
    let (content, set_content) = signal("".to_string()); // Start empty
    let (local_version, set_local_version) = signal(0u64);
    
    // Playback State
    let (history, set_history) = signal(Vec::<(u64, deve_core::models::Op)>::new());
    let (playback_version, set_playback_version) = signal(0u64);
    let (is_playback, set_is_playback) = signal(false);
    
    // Generate a session client_id (using random rough)
    let client_id = (js_sys::Math::random() * 1_000_000.0) as u64;
    
    // Initial Request: Open Document
    // We send OpenDoc on mount AND when doc_id changes.
    // NOTE: Effect runs on prop change.
    let ws_clone = ws.clone();
    Effect::new(move |_| {
         // Reset state when doc changes
         set_content.set("Loading...".to_string());
         set_local_version.set(0);
         set_history.set(Vec::new());
         
         ws_clone.send(ClientMessage::OpenDoc { doc_id });
    });

    // Effect to handle incoming messages
    let ws_clone_2 = ws.clone();
    Effect::new(move |_| {
         if let Some(msg) = ws_clone_2.msg.get() {
             match msg {
                 ServerMessage::Snapshot { doc_id: msg_doc_id, content: new_content, version } => {
                     // Filter by DocId
                     if msg_doc_id != doc_id { return; }
                     
                     leptos::logging::log!("Received Snapshot: {} chars, Ver: {}", new_content.len(), version);
                     applyRemoteContent(&new_content);
                     set_content.set(new_content);
                     set_local_version.set(version);
                     
                     // Initialize playback range
                     set_playback_version.set(version);
                     
                     // Request History
                     ws_clone_2.send(ClientMessage::RequestHistory { doc_id });
                 }
                 ServerMessage::History { doc_id: msg_doc_id, ops } => {
                     if msg_doc_id != doc_id { return; }
                     leptos::logging::log!("Received History: {} ops", ops.len());
                     set_history.set(ops);
                 }
                 ServerMessage::NewOp { doc_id: msg_doc_id, op, seq, client_id: origin_id } => {
                     if msg_doc_id != doc_id { return; }
                     
                     let current_ver = local_version.get_untracked();
                     if seq > current_ver {
                         // Filter Echoes!
                         if origin_id != client_id {
                             if let Ok(json) = serde_json::to_string(&op) {
                                 applyRemoteOp(&json);
                             }
                             set_content.set(getEditorContent());
                         }
                         set_local_version.set(seq);
                         
                         // Append to History signal if valid
                         set_history.update(|h| h.push((seq, op)));
                         
                         // Auto-advance playback if we are at the "head" (live)
                         if !is_playback.get_untracked() {
                            set_playback_version.set(seq);
                         }
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
                // If in playback mode, ignore changes (readonly)
                if is_playback.get_untracked() {
                    return;
                }

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

    // Playback Logic
    let on_playback_change = Box::new(move |ver: u64| {
        // If we move the slider, we set playback mode.
        // If ver == local, we are "live", but maybe still considered in playback if manually dragging.
        // Let's say if ver < local, it's Playback.
        let local = local_version.get_untracked();
        let is_pb = ver < local;
        set_is_playback.set(is_pb);
        set_playback_version.set(ver);
        
        // Reconstruct
        let hist = history.get_untracked();
        // Filter history <= ver
        let relevant_ops: Vec<deve_core::models::LedgerEntry> = hist.into_iter()
            .filter(|(s, _)| *s <= ver)
            .map(|(_, op)| deve_core::models::LedgerEntry {
                 doc_id, op, timestamp: 0 // timestamp irrelevant for reconstruction
            })
            .collect();
            
        let reconstructed = deve_core::state::reconstruct_content(&relevant_ops);
        applyRemoteContent(&reconstructed);
        // We do NOT update `content` signal here to avoid triggering diffs loops.
        // CodeMirror update via applyRemoteContent is enough for visual.
    });
    
    view! {
        <div class="relative w-full h-full">
            <div 
                node_ref=editor_ref
                class="w-full h-full min-h-[500px] border border-gray-300 bg-white shadow-sm pb-16"
            >
            </div>
            
            <crate::components::playback::PlaybackController 
                max_version=local_version
                current_version=playback_version
                on_change=on_playback_change
            />
        </div>
    }
}
