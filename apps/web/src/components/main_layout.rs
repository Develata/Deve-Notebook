// apps\web\src\components\main_layout.rs
use crate::editor::Editor;
use crate::i18n::Locale;
use leptos::prelude::*;

use crate::hooks::use_core::use_core;
use crate::hooks::use_layout::use_layout;
use crate::shortcuts::create_global_shortcut_handler;

use crate::components::activity_bar::SidebarView;
use crate::components::chat::ChatPanel;
use crate::components::diff_view::DiffView; // [NEW]
use crate::components::disconnect_overlay::DisconnectedOverlay;
use crate::components::merge_modal_slot::MergeModalSlot;

// Context for deep components to trigger search (e.g. BranchSwitcher)
#[derive(Clone, Copy)]
pub struct SearchControl {
    pub set_show: WriteSignal<bool>,
    pub set_mode: WriteSignal<String>,
}

/// Context for toggling AI Chat panel visibility
#[derive(Clone, Copy)]
pub struct ChatControl {
    pub chat_visible: ReadSignal<bool>,
    pub set_chat_visible: WriteSignal<bool>,
}

/// 主应用程序布局
///
/// 编排 UI 架构中定义的 "Activity Bar + Resizable Slot" 布局。
/// 管理全局 UI 状态 (命令面板, 设置) 并与核心逻辑 (`use_core`) 集成。
#[component]
pub fn MainLayout() -> impl IntoView {
    let _locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // 1. 核心状态 (全局逻辑)
    let core = use_core();

    // 2. 布局逻辑
    let (sidebar_width, start_resize, stop_resize, do_resize, is_resizing) = use_layout();

    // 3. UI 状态
    let (show_search, set_show_search) = signal(false);
    let (search_mode, set_search_mode) = signal(String::new()); // ">" for commands, "" for files, "@" for branches

    // Context for deep components to trigger search (e.g. BranchSwitcher)
    provide_context(SearchControl {
        set_show: set_show_search,
        set_mode: set_search_mode,
    });

    let (show_settings, set_show_settings) = signal(false);
    let (_show_open_modal, _set_show_open_modal) = signal(false);
    let (active_view, set_active_view) = signal(SidebarView::Explorer);
    let (pinned_views, set_pinned_views) = signal(SidebarView::all());

    // AI Chat Visibility (Default Visible for testing, can be toggled)
    let (chat_visible, set_chat_visible) = signal(true); // Default to visible

    // Provide ChatControl context for command palette
    provide_context(ChatControl {
        chat_visible,
        set_chat_visible,
    });

    // 4. 快捷键
    let handle_keydown = create_global_shortcut_handler(
        show_search.into(),
        set_show_search,
        search_mode.into(),
        set_search_mode,
    );

    // Bind shortcuts globally to window to override browser defaults (like Ctrl+P)
    // Note: We use window_event_listener here just like in the original Code.
    window_event_listener(leptos::ev::keydown, handle_keydown.clone());

    // 5. 派生 UI 回调
    let on_settings = Callback::new(move |_| set_show_settings.set(true));

    // Command Button: Smart Toggle
    let on_command = Callback::new(move |_| {
        let is_visible = show_search.get_untracked();
        let mode = search_mode.get_untracked();
        let target_mode = ">".to_string();

        if is_visible && mode == target_mode {
            // Already visible in this mode -> Toggle Off
            set_show_search.set(false);
        } else {
            // Hidden OR Different Mode -> Open & Switch
            set_search_mode.set(target_mode);
            set_show_search.set(true);
        }
    });

    // Open Button: Smart Toggle (SilverBullet style)
    let on_open = Callback::new(move |_| {
        let is_visible = show_search.get_untracked();
        let mode = search_mode.get_untracked();
        let target_mode = String::new(); // Empty for files

        if is_visible && mode == target_mode {
            // Already visible in this mode -> Toggle Off
            set_show_search.set(false);
        } else {
            // Hidden OR Different Mode -> Open & Switch
            set_search_mode.set(target_mode);
            set_show_search.set(true);
        }
    });

    // 主页操作 (清除选择)
    let set_doc = core.set_current_doc;
    let on_home = Callback::new(move |_| set_doc.set(None));

    // 打开确认逻辑 - 打开现有文档或创建新文档
    let docs = core.docs;
    let on_select = core.on_doc_select;
    let on_create = core.on_doc_create;
    let _on_open_confirm = Callback::new(move |path: String| {
        let normalized = path.replace('\\', "/");
        let target = if normalized.ends_with(".md") {
            normalized.clone()
        } else {
            format!("{}.md", normalized)
        };

        // 尝试查找现有文档
        let list = docs.get_untracked();
        if let Some((id, _)) = list.iter().find(|(_, p)| p == &target || p == &normalized) {
            on_select.run(*id);
        } else {
            // 未找到，创建新文档
            leptos::logging::log!("Document not found, creating: {}", target);
            on_create.run(target);
        }
    });

    view! {
        <div
            class="h-screen w-screen flex flex-col bg-gray-50 text-gray-900 font-sans"
            // on:keydown removed - moved to window_event_listener
            on:mousemove=move |ev| do_resize.run(ev)
            on:mouseup=move |_| stop_resize.run(())
            on:mouseleave=move |_| stop_resize.run(())
            tabindex="-1"
            style=move || if is_resizing.get() { "cursor: col-resize; user-select: none;" } else { "" }
        >
            <crate::components::search_box::UnifiedSearch
                show=show_search
                set_show=set_show_search
                mode_signal=Signal::derive(move || search_mode.get())
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

            <MergeModalSlot />

            <main class="flex-1 w-full max-w-[1400px] mx-auto p-4 flex overflow-hidden">
                 // 左侧边栏容器 (Activity Bar + Sidebar)
                 <aside
                    class="flex-none bg-white rounded-lg shadow-sm border border-gray-200 flex flex-col z-20"
                    style=move || format!("width: {}px", sidebar_width.get())
                 >
                     // Top: Activity Bar (Horizontal)
                     <crate::components::activity_bar::ActivityBar
                        active_view=active_view
                        set_active_view=set_active_view
                        pinned_views=pinned_views
                        set_pinned_views=set_pinned_views
                     />

                     // Body: Specific Sidebar Content
                     <div class="flex-1 overflow-hidden">
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
                     </div>
                 </aside>

                 // 拖动手柄
                 <div
                    class="w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-blue-50/50 group transition-colors"
                    on:mousedown=move |ev| start_resize.run(ev)
                 >
                    <div class="w-[1px] h-8 bg-gray-200 group-hover:bg-blue-300 transition-colors"></div>
                 </div>

                 // 主编辑器
                 <div class="flex-1 bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden relative flex flex-col min-w-0">
                    {move || {
                        if let Some((path, old, new)) = core.diff_content.get() {
                            return view! {
                                <DiffView
                                    path=path
                                    old_content=old
                                    new_content=new
                                    is_readonly=core.is_spectator.get()
                                    on_close=move || core.set_diff_content.set(None)
                                />
                            }.into_any();
                        }

                        match core.current_doc.get() {
                            Some(id) => view! {
                                 <Editor doc_id=id on_stats=core.on_stats />
                            }.into_any(),
                            None => view! {
                                <div class="flex items-center justify-center h-full text-gray-400">
                                    "Select a document to edit"
                                </div>
                            }.into_any()
                        }
                    }}
                 </div>

                 // Column 5: AI Chat (Resizable Slot)
                 // Right Sidebar for Chat
                 // Currently fixed width 300px for simplicity, can be resizable later
                 {move || if chat_visible.get() {
                    view! {
                        <div class="w-[350px] flex-none ml-4 bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden flex flex-col">
                            <ChatPanel />
                        </div>
                    }.into_any()
                 } else {
                    // Chat Toggle Button (Floating or Integrated)
                    // Let's add a small toggle button if hidden?
                    // Or rely on command palette to toggle.
                    // For now, let's just use command palette to toggle `chat_visible`.
                    view! {}.into_any()
                 }}
            </main>
            <crate::components::bottom_bar::BottomBar ai_mode=core.ai_mode stats=core.stats />

            <DisconnectedOverlay status=core.ws.status.into() />
        </div>
    }
}
