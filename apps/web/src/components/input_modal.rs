use leptos::prelude::*;

#[component]
pub fn InputModal(
    show: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
    #[prop(into)] title: Signal<String>,
    #[prop(into)] initial_value: Signal<Option<String>>,
    #[prop(into)] placeholder: Signal<String>,
    #[prop(into)] confirm_label: Signal<String>,
    #[prop(into)] on_confirm: Callback<String>,
) -> impl IntoView {
    let (value, set_value) = signal(String::new());
    
    // Focus input and set initial value when shown
    let input_ref = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
         if show.get() {
             set_value.set(initial_value.get().unwrap_or_default());
             if let Some(el) = input_ref.get() {
                 let _ = el.focus();
                 // Select all if renaming (simple hack: timeout or set selection)
                 // For now just focus.
             }
         }
    });

    let submit = move || {
        let val = value.get();
        if !val.trim().is_empty() {
             on_confirm.run(val);
             set_show.set(false);
        }
    };
    
    view! {
        <div 
            class=move || if show.get() { 
                "fixed inset-0 z-[60]" // Transparent overlay, high z-index
            } else { 
                "hidden" 
            }
            on:click=move |_| set_show.set(false)
        >
            // The Box - Top Center Floating (Matching CommandPalette)
            <div 
                class="absolute top-2 left-1/2 -translate-x-1/2 w-full max-w-xl bg-white rounded-lg shadow-xl border border-gray-200 overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-100"
                on:click=move |ev| ev.stop_propagation()
            >
                // Title & Input Container
                <div class="flex flex-col">
                     // Optional Title Header (subtle)
                     <div class="px-3 py-1.5 text-xs font-semibold text-gray-500 uppercase bg-gray-50/50 border-b border-gray-100">
                        {move || title.get()}
                     </div>

                     // Input Row
                     <div class="p-3 flex items-center gap-3">
                        // Icon (Generic Edit/Input)
                        <svg class="w-4 h-4 text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                        </svg>
                        
                        <input 
                            node_ref=input_ref
                            type="text"
                            class="flex-1 outline-none text-base bg-transparent text-gray-800 placeholder:text-gray-400"
                            placeholder=move || placeholder.get()
                            prop:value=value
                            on:input=move |ev| set_value.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    submit();
                                } else if ev.key() == "Escape" {
                                    set_show.set(false);
                                }
                            }
                        />
                     </div>
                </div>
                
                // Footer Hints
                <div class="bg-gray-50 px-4 py-2 border-t border-gray-100 flex justify-end items-center text-xs text-gray-500 gap-4">
                     <span class="flex items-center gap-1">
                        <kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Enter</kbd> 
                        <span>{move || confirm_label.get()}</span>
                     </span>
                     <span class="flex items-center gap-1">
                        <kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Esc</kbd> 
                        <span>"Cancel"</span>
                     </span>
                </div>
            </div>
        </div>
    }
}
