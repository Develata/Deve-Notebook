use leptos::prelude::*;
use leptos::html::Div;
use deve_core::models::DocId;

pub mod ffi;
pub mod hook;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EditorStats {
    pub chars: usize,
    pub words: usize,
    pub lines: usize,
}

#[component]
pub fn Editor(
    doc_id: DocId,
    #[prop(optional)] on_stats: Option<Callback<EditorStats>>,
) -> impl IntoView {
    let editor_ref = NodeRef::<Div>::new();
    
    // Use the hook logic
    let state = hook::use_editor(doc_id, editor_ref, on_stats);
    
    // Unwrap state
    let local_version = state.local_version;
    let playback_version = state.playback_version;
    // let is_playback = state.is_playback; // Unused in view directly, logic handled in hook
    let on_playback_change = state.on_playback_change;

    view! {
        // Main container: Relative for positioning playback, 100% size
        <div class="relative w-full h-full flex flex-col overflow-hidden">
            // Editor Area
            <div 
                node_ref=editor_ref
                class="w-full flex-1 border border-gray-300 bg-white shadow-sm overflow-hidden"
            >
            </div>
            
            // Playback
            <div class="absolute bottom-4 left-4 right-4 z-10">
                <crate::components::playback::PlaybackController 
                    max_version=local_version
                    current_version=playback_version
                    on_change=on_playback_change
                />
            </div>
        </div>
    }
}
