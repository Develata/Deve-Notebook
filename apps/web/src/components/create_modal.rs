use leptos::prelude::*;

#[component]
pub fn CreateModal(
    show: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
    parent_path: ReadSignal<Option<String>>,
    on_create: Callback<String>,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    
    // Focus input when shown
    let input_ref = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
         if show.get() {
             set_name.set(String::new()); // Reset
             if let Some(el) = input_ref.get() {
                 let _ = el.focus();
             }
         }
    });

    let submit = move || {
        let val = name.get();
        if !val.trim().is_empty() {
             let full_path = if let Some(parent) = parent_path.get() {
                 format!("{}/{}", parent, val)
             } else {
                 val
             };
             on_create.run(full_path);
             set_show.set(false);
        }
    };
    
    view! {
        <div 
            class=move || if show.get() { 
                "fixed inset-0 z-50 flex items-center justify-center bg-black/20 backdrop-blur-sm transition-opacity" 
            } else { 
                "hidden" 
            }
            on:click=move |_| set_show.set(false)
        >
            <div 
                class="bg-white rounded-lg shadow-xl w-96 p-4 transform transition-all scale-100"
                on:click=move |ev| ev.stop_propagation()
            >
                <div class="flex items-center justify-between mb-4">
                    <h3 class="text-lg font-medium text-gray-900">
                        {move || if let Some(p) = parent_path.get() {
                            format!("Create in '{}'", p)
                        } else {
                            "Create New Document".to_string()
                        }}
                    </h3>
                    <button 
                        class="text-gray-400 hover:text-gray-500"
                        on:click=move |_| set_show.set(false)
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>
                
                <input 
                    node_ref=input_ref
                    type="text"
                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    placeholder="filename or folder/filename"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            submit();
                        } else if ev.key() == "Escape" {
                            set_show.set(false);
                        }
                    }
                />
                
                <div class="mt-4 flex justify-end gap-2">
                    <button 
                        class="px-3 py-1.5 text-sm text-gray-600 hover:bg-gray-100 rounded-md"
                        on:click=move |_| set_show.set(false)
                    >
                        "Cancel"
                    </button>
                    <button 
                        class="px-3 py-1.5 text-sm text-white bg-blue-600 hover:bg-blue-700 rounded-md shadow-sm"
                        on:click=move |_| submit()
                    >
                        "Create"
                    </button>
                </div>
            </div>
        </div>
    }
}
