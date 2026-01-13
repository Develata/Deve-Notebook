//! # 主应用组件
//!
//! 本模块包含根 `App` 组件和主布局。
//!
//! ## 结构说明
//!
//! - `App`: 根组件，提供语言环境上下文
//! - `AppContent`: 主内容区，包含头部、侧边栏、编辑器和模态框
//!
//! 使用自定义 Hooks: `use_core`, `use_layout`, `use_shortcuts`

use crate::editor::Editor;
use leptos::prelude::*;
use deve_core::models::DocId;
use crate::i18n::Locale;
use web_sys::KeyboardEvent;

use crate::hooks::use_core::use_core;
use crate::hooks::use_layout::use_layout;
use crate::hooks::use_shortcuts::use_shortcuts;

use crate::components::activity_bar::SidebarView;

#[component]
pub fn App() -> impl IntoView {
    // Global Locale State
    let locale = RwSignal::new(Locale::default());
    provide_context(locale);

    view! {
        <AppContent/>
    }
}

#[component]
fn AppContent() -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // 1. Core State (Global Logic)
    let core = use_core();

    // 2. Layout Logic
    let (sidebar_width, start_resize, stop_resize, do_resize, is_resizing) = use_layout();

    // 3. UI State
    let (show_cmd, set_show_cmd) = signal(false);
    let (show_settings, set_show_settings) = signal(false);
    let (show_open_modal, set_show_open_modal) = signal(false);
    let (active_view, set_active_view) = signal(SidebarView::Explorer);

    // 4. Shortcuts
    let handle_keydown = use_shortcuts(locale, show_cmd.into(), set_show_cmd, set_show_open_modal);

    // 5. Derived UI Callbacks
    let on_settings = Callback::new(move |_| set_show_settings.set(true));
    let on_command = Callback::new(move |_| set_show_cmd.update(|s| *s = !*s));
    let on_open = Callback::new(move |_| set_show_open_modal.set(true));
    
    // Home Action (Clear Selection)
    let set_doc = core.set_current_doc;
    let on_home = Callback::new(move |_| set_doc.set(None));

    // Open Confirm Logic - Opens existing doc or creates new one
    let docs = core.docs;
    let on_select = core.on_doc_select;
    let on_create = core.on_doc_create;
    let on_open_confirm = Callback::new(move |path: String| {
        let normalized = path.replace('\\', "/");
        let target = if normalized.ends_with(".md") { normalized.clone() } else { format!("{}.md", normalized) };
        
        // Try to find existing doc
        let list = docs.get_untracked();
        if let Some((id, _)) = list.iter().find(|(_, p)| p == &target || p == &normalized) {
            on_select.run(*id);
        } else {
            // Not found, create new document
            leptos::logging::log!("Document not found, creating: {}", target);
            on_create.run(target);
        }
    });

    view! {
        <div 
            class="h-screen w-screen flex flex-col bg-gray-50 text-gray-900 font-sans"
            on:keydown=handle_keydown
            on:mousemove=move |ev| do_resize.run(ev)
            on:mouseup=move |_| stop_resize.run(())
            on:mouseleave=move |_| stop_resize.run(())
            tabindex="-1" 
            style=move || if is_resizing.get() { "cursor: col-resize; user-select: none;" } else { "" }
        >
            <crate::components::command_palette::CommandPalette 
                show=show_cmd 
                set_show=set_show_cmd
                on_settings=on_settings 
                on_open=on_open
            />
            <crate::components::header::Header 
                status_text=core.status_text 
                on_home=on_home
                on_open=on_open
                on_command=on_command
            />
            
            <crate::components::settings::SettingsModal 
                show=show_settings 
                set_show=set_show_settings
            />

            <crate::components::input_modal::InputModal
                show=show_open_modal
                set_show=set_show_open_modal
                title="Open Document"
                placeholder="folder/file.md or folder\\file"
                confirm_label="Open"
                initial_value={None::<String>}
                on_confirm=on_open_confirm
            />
            
            // Manual Merge Modal
            {move || {
                let (show_merge, set_show_merge) = signal(false);
                provide_context(set_show_merge); // Allow triggering from deep components
                view! {
                    <crate::components::merge_modal::MergeModal 
                        show=show_merge
                        set_show=set_show_merge
                    />
                }
            }}

            <main class="flex-1 w-full max-w-[1400px] mx-auto p-4 flex overflow-hidden">
                 // Activity Bar (Fixed Left)
                 <crate::components::activity_bar::ActivityBar 
                    active_view=active_view 
                    set_active_view=set_active_view 
                    on_settings=on_settings 
                 />

                 // Left Sidebar (Resizable & Swappable)
                 <aside 
                    class="flex-none bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden"
                    style=move || format!("width: {}px", sidebar_width.get())
                 >
                     <crate::components::sidebar::Sidebar 
                        active_view=active_view
                        docs=core.docs
                        current_doc=core.current_doc
                        on_select=core.on_doc_select
                        on_create=core.on_doc_create
                        on_rename=core.on_doc_rename
                        on_delete=core.on_doc_delete
                        on_copy=core.on_doc_copy
                        on_move=core.on_doc_move
                     />
                 </aside>
                 
                 // Drag Handle
                 <div 
                    class="w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-blue-50/50 group transition-colors"
                    on:mousedown=move |ev| start_resize.run(ev)
                 >
                    <div class="w-[1px] h-8 bg-gray-200 group-hover:bg-blue-300 transition-colors"></div>
                 </div>

                 // Main Editor
                 <div class="flex-1 bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden relative flex flex-col min-w-0">
                    
                    {move || match core.current_doc.get() {
                        Some(id) => view! { 
                             <Editor doc_id=id on_stats=core.on_stats /> 
                        }.into_any(),
                        None => view! { 
                            <div class="flex items-center justify-center h-full text-gray-400">
                                "Select a document to edit"
                            </div> 
                        }.into_any()
                    }}
                 </div>
            </main>
            
            <crate::components::bottom_bar::BottomBar status=core.ws.status stats=core.stats />
            
            // Disconnect Lock / Loading Screen
            {move || {
                let status = core.ws.status.get();
                if status != crate::api::ConnectionStatus::Connected {
                    view! {
                        <div class="fixed inset-0 z-[9999] bg-white/80 backdrop-blur-sm flex flex-col items-center justify-center">
                            <div class="bg-white p-8 rounded-xl shadow-lg border border-gray-200 text-center">
                                <div class="text-4xl mb-4">"🔒"</div>
                                <h1 class="text-2xl font-bold text-gray-800 mb-2">"Disconnected"</h1>
                                <p class="text-gray-600 mb-6">"Reconnecting to server... please wait."</p>
                                <div class="w-full bg-gray-200 rounded-full h-2.5 dark:bg-gray-700">
                                  <div class="bg-blue-600 h-2.5 rounded-full animate-pulse" style="width: 100%"></div>
                                </div>
                                <div class="mt-4 text-sm text-gray-400">
                                    {format!("Status: {}", status)}
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                     view! {}.into_any()
                }
            }}
        </div>
    }
}
