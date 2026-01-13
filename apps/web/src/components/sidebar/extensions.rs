use leptos::prelude::*;

#[component]
pub fn ExtensionsView() -> impl IntoView {
    view! {
         <div class="h-full w-full bg-[#f7f7f7] flex flex-col">
            <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-gray-100">
                 <span class="font-medium text-sm text-gray-700">"Extensions"</span>
            </div>
            <div class="flex-1 flex flex-col items-center justify-center text-gray-400 p-4 text-center">
                <svg xmlns="http://www.w3.org/2000/svg" class="w-12 h-12 mb-2 text-gray-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="2" y="2" width="9" height="9" rx="2"/><rect x="13" y="2" width="9" height="9" rx="2"/><rect x="13" y="13" width="9" height="9" rx="2"/><line x1="8" y1="21" x2="8" y2="12"/><line x1="8" y1="12" x2="3" y2="12"/><path d="M2.5 21h5.5a2 2 0 0 0 2-2v-5a2 2 0 0 0-2-2H2.5a.5.5 0 0 0-.5.5v8a.5.5 0 0 0 .5.5z"/>
                </svg>
                <p class="text-sm">"Plugin System coming in Phase 3"</p>
            </div>
        </div>
    }
}
