use leptos::prelude::*;
use leptos::html::Div;
use wasm_bindgen::prelude::*;
use crate::api::WsService;
use crate::hooks::use_core::CoreState;
use deve_core::protocol::ClientMessage;
use deve_core::models::DocId;
use super::ffi::{setupCodeMirror, set_read_only};
use super::EditorStats;
use super::sync;
use super::playback;

pub struct EditorState {
    pub content: ReadSignal<String>,
    pub is_playback: ReadSignal<bool>,
    pub playback_version: ReadSignal<u64>,
    pub local_version: ReadSignal<u64>, 
    pub on_playback_change: Box<dyn Fn(u64) + Send + Sync>, 
}

pub fn use_editor(
    doc_id: DocId,
    editor_ref: NodeRef<Div>,
    on_stats: Option<Callback<EditorStats>>,
) -> EditorState {
    let ws = use_context::<WsService>().expect("WsService should be provided");
    let core = expect_context::<CoreState>();
    
    // Local state of the document to compute diffs against
    let (content, set_content) = signal("".to_string()); // Start empty
    let (local_version, set_local_version) = signal(0u64);
    
    // Playback State
    let (history, set_history) = signal(Vec::<(u64, deve_core::models::Op)>::new());
    // We use Core's playback version signal
    // let (playback_version, set_playback_version) = signal(0u64); <-- REPLACED
    let playback_version = core.playback_version;
    let set_playback_version = core.set_playback_version;
    
    let (is_playback, set_is_playback) = signal(false);
    
    // Generate a session client_id (using random rough)
    let client_id = (js_sys::Math::random() * 1_000_000.0) as u64;
    
    // Initial Request: Open Document
    // We send OpenDoc on mount AND when doc_id changes.
    // NOTE: Effect runs on prop change.
    let ws_clone = ws.clone();
    let set_doc_ver = core.set_doc_version;
    Effect::new(move |_| {
         // Reset state when doc changes
         set_content.set("Loading...".to_string());
         set_local_version.set(0);
         set_history.set(Vec::new());
         
         // Reset Core State for this doc
         set_doc_ver.set(0);
         set_playback_version.set(0);
         
         ws_clone.send(ClientMessage::OpenDoc { doc_id });
    });

    // Sync Local Version to Core Doc Version
    Effect::new(move |_| {
         let ver = local_version.get();
         set_doc_ver.set(ver);
    });

    // Effect to handle incoming messages (Delegated to sync module)
    let ws_clone_2 = ws.clone();
    Effect::new(move |_| {
         if let Some(msg) = ws_clone_2.msg.get() {
             sync::handle_server_message(
                 msg,
                 doc_id,
                 client_id,
                 &ws_clone_2,
                 set_content,
                 local_version,
                 set_local_version,
                 set_history,
                 is_playback,
                 set_playback_version,
                 on_stats
             );
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

                // Compute Stats
                if let Some(cb) = on_stats {
                     let lines = new_text.lines().count();
                     let words = new_text.split_whitespace().count();
                     cb.run(EditorStats { chars: new_text.len(), words, lines });
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

    // Playback Logic (Listens to Core Playback Version)
    Effect::new(move |_| {
         let ver = playback_version.get();
         let local = local_version.get_untracked();
         
         // Call logic
         playback::handle_playback_change(
            ver,
            doc_id,
            local,
            history, 
            set_is_playback
        );
        
        // Imperative sync
        let is_pb = ver < local;
        unsafe { set_read_only(is_pb); }
    });

    let on_playback_change = Box::new(move |ver: u64| {
        set_playback_version.set(ver);
    });

    EditorState {
        content: content,
        is_playback: is_playback,
        playback_version: playback_version,
        local_version: local_version,
        on_playback_change,
    }
}
