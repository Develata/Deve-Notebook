//! # ActivityBar 组件 (ActivityBar Component)
//!
//! 侧边栏导航条，用于在不同的视图（资源管理器、搜索、源码管理、扩展）之间切换。

use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SidebarView {
    #[default]
    Explorer,      // 资源管理器
    Search,        // 搜索
    SourceControl, // 源代码管理 (Git)
    Extensions,    // 扩展
}

#[component]
pub fn ActivityBar(
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    on_settings: Callback<()>,
) -> impl IntoView {
    
    let icon_btn = move |view: SidebarView, icon: &'static str, label: &'static str| {
        let is_active = move || active_view.get() == view;
        view! {
            <button
                class=move || format!(
                    "p-3 mb-2 rounded-lg transition-colors relative group {}", 
                    if is_active() { "text-blue-600 dark:text-blue-400" } else { "text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100" }
                )
                title=label
                on:click=move |_| set_active_view.set(view)
            >
                <div class="w-6 h-6" inner_html=icon></div>
                
                // 活动指示器
                {move || if is_active() {
                    view! { <div class="absolute left-0 top-2 bottom-2 w-1 bg-blue-400 rounded-r"></div> }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </button>
        }
    };

    view! {
        <div class="w-12 flex flex-col items-center py-4 bg-[#f3f3f3] dark:bg-[#252526] border-r border-[#e5e5e5] dark:border-[#1e1e1e] flex-none z-20">
            // 顶部区域: 类似于 VS Code
            
            // 资源管理器
            {icon_btn(
                SidebarView::Explorer, 
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>"#,
                "Explorer"
            )}
            
            // 搜索
            {icon_btn(
                SidebarView::Search, 
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>"#,
                "Search"
            )}
            
            // 源代码管理 (Git)
            {icon_btn(
                SidebarView::SourceControl, 
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M6 9v12"/><path d="M18 9v12"/><path d="M12 15V3"/></svg>"#,
                "Source Control"
            )}
            
            // 扩展
            {icon_btn(
                SidebarView::Extensions, 
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="9" height="9" rx="2"/><rect x="13" y="2" width="9" height="9" rx="2"/><rect x="13" y="13" width="9" height="9" rx="2"/><line x1="8" y1="21" x2="8" y2="12"/><line x1="8" y1="12" x2="3" y2="12"/><path d="M2.5 21h5.5a2 2 0 0 0 2-2v-5a2 2 0 0 0-2-2H2.5a.5.5 0 0 0-.5.5v8a.5.5 0 0 0 .5.5z"/></svg>"#,
                "Extensions"
            )}
            
            <div class="flex-1"></div>
            
            // 底部:设置
            <button 
                class="p-3 mb-2 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100 rounded-lg transition-colors"
                title="Settings"
                on:click=move |_| on_settings.run(())
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 1-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.47a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.39a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
        </div>
    }
}
