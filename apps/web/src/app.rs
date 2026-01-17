// apps\web\src
//! # App Component (App 组件)
//!
//! 本模块包含根 `App` 组件和主布局。
//!
//! ## 结构说明
//!
//! - `App`: 根组件，提供语言环境上下文。
//! - `AppContent`: 主内容区，包含头部、侧边栏、编辑器和模态框。
//!
//! 使用自定义 Hooks: `use_core`, `use_layout`, `use_shortcuts`

use crate::editor::Editor;
use leptos::prelude::*;
use crate::i18n::Locale;

use crate::hooks::use_core::use_core;
use crate::hooks::use_layout::use_layout;
use crate::shortcuts::create_global_shortcut_handler;

use crate::components::activity_bar::SidebarView;
use crate::components::diff_view::DiffView;

// Context for deep components to trigger search (e.g. BranchSwitcher)
#[derive(Clone, Copy)]
pub struct SearchControl {
    pub set_show: WriteSignal<bool>,
    pub set_mode: WriteSignal<String>,
}

/// 根应用程序组件
/// 
/// 初始化全局状态 (Locale) 并渲染 AppContent。
#[component]
pub fn App() -> impl IntoView {
    // 全局语言环境状态
    let locale = RwSignal::new(Locale::default());
    provide_context(locale);

    view! {
        <AppContent/>
    }
}

/// 主应用程序布局
/// 
/// 编排 UI 架构中定义的 "活动栏 + 可调整大小插槽" 布局。
/// 管理全局 UI 状态 (命令面板, 设置) 并与核心逻辑 (`use_core`) 集成。
#[component]
fn AppContent() -> impl IntoView {
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

    // 4. 快捷键
    let handle_keydown = create_global_shortcut_handler(
        show_search.into(), 
        set_show_search, 
        search_mode.into(),
        set_search_mode,
    );
    
    // Bind shortcuts globally to window to override browser defaults (like Ctrl+P)
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
        let target = if normalized.ends_with(".md") { normalized.clone() } else { format!("{}.md", normalized) };
        
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

            /*
            <crate::components::input_modal::InputModal
                show=show_open_modal
                set_show=set_show_open_modal
                title="Open Document"
                placeholder="folder/file.md or folder\\file"
                confirm_label="Open"
                initial_value={None::<String>}
                on_confirm=on_open_confirm
            />
            */
            
            // 手动合并模态框
            {move || {
                let (show_merge, set_show_merge) = signal(false);
                provide_context(set_show_merge); // 允许从深层组件触发
                view! {
                    <crate::components::merge_modal::MergeModal 
                        show=show_merge
                        set_show=set_show_merge
                    />
                }
            }}

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
            </main>
            
            <crate::components::bottom_bar::BottomBar status=core.ws.status stats=core.stats />
            
            // 断开连接锁定 / 加载屏幕
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
