pub mod model;
pub mod sync;

use self::model::compute_diff;
use self::sync::use_scroll_sync;

use leptos::html;
use leptos::prelude::*;

#[component]
pub fn DiffView<F>(
    path: String,
    old_content: String,
    new_content: String,
    #[prop(default = false)] is_readonly: bool,
    on_close: F,
) -> impl IntoView
where
    F: Fn() + 'static + Clone,
{
    // Extract filename for display
    let filename = path
        .replace('\\', "/")
        .split('/')
        .last()
        .unwrap_or("?")
        .to_string();

    // State for Edit Mode
    let (is_editing, set_is_editing) = signal(false);
    let (content, set_content) = signal(new_content);

    // Compute Diff
    // Use Memo to allow shared access to diff results without moving ownership
    let diff_result = Memo::new(move |_| compute_diff(&old_content, &content.get())); // Refs for scroll sync
    let left_container = NodeRef::<html::Div>::new();
    let right_container = NodeRef::<html::Div>::new();

    // Activate Scroll Sync
    use_scroll_sync(left_container, right_container);

    view! {
        <div class="h-full w-full bg-white dark:bg-[#1e1e1e] flex flex-col font-mono text-[13px]">
            // Header
            <div class="flex-none h-10 border-b border-[#e5e5e5] dark:border-[#252526] flex items-center justify-between px-4 bg-[#f3f3f3] dark:bg-[#2d2d2d]">
                <div class="flex items-center gap-2">
                    <span class="font-bold text-[#3b3b3b] dark:text-[#cccccc]">"Diff: "</span>
                    <span class="text-[#237893] dark:text-[#4fc1ff]">{filename}</span>
                    {move || if is_readonly {
                        view! { <span class="text-xs bg-gray-200 dark:bg-gray-700 px-2 py-0.5 rounded text-gray-500">"Read Only"</span> }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
                <div class="flex items-center gap-2">
                    {move || if !is_readonly {
                        view! {
                            <button
                                class="px-3 py-1 bg-white dark:bg-[#3e3e42] border border-[#e5e5e5] dark:border-[#454545] rounded text-xs hover:bg-gray-50 dark:hover:bg-[#4d4d4d] text-[#3b3b3b] dark:text-[#cccccc]"
                                on:click=move |_| set_is_editing.update(|v| *v = !*v)
                            >
                                {move || if is_editing.get() { "Preview Diff" } else { "Edit" }}
                            </button>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                    <button
                        class="p-1 hover:bg-[#e1e1e1] dark:hover:bg-[#3e3e42] rounded text-[#616161]"
                        on:click=move |_| on_close()
                        title="Close Diff View"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    </button>
                </div>
            </div>


            // Content
            <div class="flex-1 overflow-hidden flex relative">
                // Left Pane (Old) - Always Visible
                <div
                    class="flex-1 flex overflow-auto border-r border-[#e5e5e5] dark:border-[#454545]"
                    node_ref=left_container
                >
                    // Line Nums
                    <div class="w-10 flex-none bg-[#f8f8f8] dark:bg-[#1e1e1e] text-right pr-3 text-[#aaa] select-none py-1 border-r border-[#eee] dark:border-[#333]">
                         <For
                            each=move || diff_result.get().0
                            key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                            children=|item| view! { <div class="h-[20px] leading-[20px]">{item.num.map(|n| n.to_string()).unwrap_or_default()}</div> }
                        />
                    </div>
                    // Code
                    <div class="flex-1 min-w-0 py-1 bg-white dark:bg-[#1e1e1e]">
                         <For
                            each=move || diff_result.get().0
                            key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                            children=|item| view! {
                                <div class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", item.class)>
                                    {item.content}
                                </div>
                            }
                        />
                    </div>
                </div>

                // Right Pane (New) - Switchable
                <div
                    class="flex-1 flex overflow-auto relative"
                    node_ref=right_container
                >
                    {move || if is_editing.get() {
                        view! {
                            <textarea
                                class="w-full h-full p-2 resize-none outline-none font-mono text-[13px] bg-white dark:bg-[#1e1e1e] text-[#3b3b3b] dark:text-[#cccccc] border-none"
                                prop:value=move || content.get()
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    set_content.set(val);
                                }
                            ></textarea>
                        }.into_any()
                    } else {
                        view! {
                            <div class="flex w-full min-h-full">
                                // Line Nums
                                <div class="w-10 flex-none bg-[#f8f8f8] dark:bg-[#1e1e1e] text-right pr-3 text-[#aaa] select-none py-1 border-r border-[#eee] dark:border-[#333]">
                                     <For
                                        each=move || diff_result.get().1
                                        key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                                        children=|item| view! { <div class="h-[20px] leading-[20px]">{item.num.map(|n| n.to_string()).unwrap_or_default()}</div> }
                                    />
                                </div>
                                // Code
                                <div class="flex-1 min-w-0 py-1 bg-white dark:bg-[#1e1e1e] select-text">
                                    <For
                                        each=move || diff_result.get().1
                                        key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                                        children=|item| view! {
                                            <div class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", item.class)>
                                                {item.content}
                                            </div>
                                        }
                                    />
                                </div>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
