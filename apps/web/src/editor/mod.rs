use leptos::prelude::*;
use leptos::html::Div;
use deve_core::models::DocId;

pub mod ffi;
pub mod hook;
pub mod sync;
pub mod playback;

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
    let content = state.content;
    
    // Outline State
    let (show_outline, set_show_outline) = signal(true);
    let on_toggle_outline = Callback::new(move |_| set_show_outline.update(|b| *b = !*b));
    
    let on_scroll = Callback::new(move |line: usize| {
        unsafe { ffi::scroll_global(line); }
    });

    view! {
        // Main container: Relative for positioning playback, 100% size
        <div class="relative w-full h-full flex flex-col overflow-hidden">
            // Top Bar / Toggle (Absolute to not consume flow, or part of editor header?)
            // Or just float it?
            
            // Content Area (Flex Row)
            <div class="flex-1 flex overflow-hidden relative">
                // Editor Wrapper
                <div class="flex-1 relative border-r border-gray-200 bg-white shadow-sm overflow-hidden">
                    <div 
                        node_ref=editor_ref
                        class="absolute inset-0"
                        class:bg-gray-100=move || playback_version.get() < local_version.get()
                    ></div>

                    // Spectator Badge
                    {move || if playback_version.get() < local_version.get() {
                        view! {
                            <div class="absolute top-2 left-1/2 -translate-x-1/2 z-50 px-3 py-1 bg-yellow-100 text-yellow-800 text-xs font-semibold rounded-full shadow-sm border border-yellow-200 pointer-events-none opacity-80 backdrop-blur-sm">
                                "Spectator Mode (Read Only)"
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                    
                     // Toggle Outline Button (Safe Sibling)
                     <button
                        on:click=move |_| on_toggle_outline.run(())
                        class="absolute top-2 right-4 z-50 p-1.5 text-gray-500 hover:text-gray-700 hover:bg-gray-100 bg-white/90 border border-gray-200 rounded shadow-sm transition-all"
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
            
        </div>
    }
}
