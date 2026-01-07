use leptos::prelude::*;

#[component]
pub fn Header(
    #[prop(into)] status_text: Signal<String>,
) -> impl IntoView {
    view! {
        <header class="w-full h-12 bg-white border-b border-gray-200 flex items-center justify-between px-4 shadow-sm z-50">
            <div class="flex items-center gap-2">
                <span class="font-bold text-gray-800 text-lg">"Deve-Note"</span>
                <span class="text-xs text-gray-400 border border-gray-200 rounded px-1">{move || status_text.get()}</span>
            </div>
            
            <div class="flex items-center gap-2">
                <button 
                    class="p-2 text-gray-500 hover:bg-gray-100 rounded-full transition-colors"
                    title="Settings"
                >
                    // Simple Gear Icon SVG
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.1a2 2 0 0 1-1-1.72v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"></path>
                        <circle cx="12" cy="12" r="3"></circle>
                    </svg>
                </button>
            </div>
        </header>
    }
}
