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
    let on_playback_change = state.on_playback_change;
    let content = state.content;
    
    // Outline State
    let (show_outline, set_show_outline) = signal(true);
    let on_toggle_outline = Callback::new(move |_| set_show_outline.update(|b| *b = !*b));
    
    let on_scroll = Callback::new(move |line: usize| {
        ffi::scroll_global(line);
    });

    view! {
        // Main container: Relative for positioning playback, 100% size
        <div class="relative w-full h-full flex flex-col overflow-hidden">
            // Top Bar / Toggle (Absolute to not consume flow, or part of editor header?)
            // Or just float it?
            
            // Content Area (Flex Row)
            <div class="flex-1 flex overflow-hidden relative">
                // Editor Area
                <div 
                    node_ref=editor_ref
                    class="flex-1 border-r border-gray-200 bg-white shadow-sm overflow-hidden relative"
                >
                     // Toggle Outline Button (Floating top right of editor)
                     <button
                        on:click=move |_| on_toggle_outline.run(())
                        class="absolute top-2 right-4 z-20 p-1 text-gray-400 hover:text-gray-600 bg-white/80 rounded"
                        title="Toggle Outline"
                     >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5">
                          <path fill-rule="evenodd" d="M3 4.25A2.25 2.25 0 015.25 2h9.5A2.25 2.25 0 0117 4.25v11.5A2.25 2.25 0 0114.75 18h-9.5A2.25 2.25 0 013 15.75V4.25zM6 13a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm0-5a1 1 0 11-2 0 1 1 0 012 0zm3 10a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2zm7 5a1 1 0 110-2 1 1 0 010 2zm0-5a1 1 0 110-2 1 1 0 010 2z" clip-rule="evenodd" />
                        </svg>
                     </button>
                </div>

                // Outline Sidebar
                <div 
                    class="bg-[#f9f9f9] border-l border-gray-200 transition-all duration-300 ease-in-out overflow-hidden"
                    style=move || if show_outline.get() { "width: 250px; opacity: 1;" } else { "width: 0px; opacity: 0;" }
                >
                    <crate::components::outline::Outline
                        content=content
                        on_scroll=on_scroll
                    />
                </div>
            </div>
            
            // Playback
            <div class="absolute bottom-4 left-4 right-4 z-10 pointer-events-none">
                 <div class="pointer-events-auto">
                    <crate::components::playback::PlaybackController 
                        max_version=local_version
                        current_version=playback_version
                        on_change=on_playback_change
                    />
                 </div>
            </div>
        </div>
    }
}
