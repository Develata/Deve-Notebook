use leptos::prelude::*;

/// 菜单操作类型
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAction {
    Rename,
    CopyLink,
    OpenInNewWindow,
    MoveTo,
    Duplicate,
    Delete,
}

/// 菜单项配置
struct MenuItem {
    action: MenuAction,
    label: &'static str,
    icon: &'static str, // SVG path
    is_danger: bool,
    is_separator_before: bool,
}

impl MenuItem {
    const fn new(action: MenuAction, label: &'static str, icon: &'static str) -> Self {
        Self { action, label, icon, is_danger: false, is_separator_before: false }
    }
    
    const fn danger(mut self) -> Self {
        self.is_danger = true;
        self
    }
    
    const fn with_separator(mut self) -> Self {
        self.is_separator_before = true;
        self
    }
}

/// 定义所有菜单项
const MENU_ITEMS: &[MenuItem] = &[
    MenuItem::new(
        MenuAction::Rename,
        "Rename",
        "M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10"
    ),
    MenuItem::new(
        MenuAction::CopyLink,
        "Copy Link",
        "M13.19 8.688a4.5 4.5 0 011.242 7.244l-4.5 4.5a4.5 4.5 0 01-6.364-6.364l1.757-1.757m13.35-.622l1.757-1.757a4.5 4.5 0 00-6.364-6.364l-4.5 4.5a4.5 4.5 0 001.242 7.244"
    ),
    MenuItem::new(
        MenuAction::OpenInNewWindow,
        "Open in New Window",
        "M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25"
    ),
    MenuItem {
        action: MenuAction::MoveTo,
        label: "Move to...",
        icon: "M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15M12 9l-3 3m0 0l3 3m-3-3h12.75",
        is_danger: false,
        is_separator_before: true,
    },
    MenuItem::new(
        MenuAction::Duplicate,
        "Duplicate",
        "M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 002.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 00-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 00.75-.75 2.25 2.25 0 00-.1-.664m-5.8 0A2.251 2.251 0 0113.5 2.25H15c1.012 0 1.867.668 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25zM6.75 12h.008v.008H6.75V12zm0 3h.008v.008H6.75V15zm0 3h.008v.008H6.75V18z"
    ),
    MenuItem {
        action: MenuAction::Delete,
        label: "Delete",
        icon: "M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0",
        is_danger: true,
        is_separator_before: true,
    },
];

#[component]
pub fn SidebarMenu(
    /// 当用户选择一个操作时触发
    #[prop(into)] on_action: Callback<MenuAction>,
    /// 关闭菜单
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <>
            // Backdrop
            <div 
                class="fixed inset-0 z-40" 
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_close.run(());
                }
            ></div>
            
            // Menu Panel
            <div 
                class="absolute right-0 top-6 w-48 bg-white rounded-md shadow-lg border border-gray-100 py-1 z-50 text-sm text-gray-700 select-none animate-in fade-in zoom-in-95 duration-100 ease-out origin-top-right"
                on:click=move |ev| ev.stop_propagation()
            >
                {MENU_ITEMS.iter().map(|item| {
                    let action = item.action;
                    let label = item.label;
                    let icon_path = item.icon;
                    let is_danger = item.is_danger;
                    let has_separator = item.is_separator_before;
                    
                    view! {
                        <>
                            {if has_separator {
                                Some(view! { <div class="my-1 border-t border-gray-100"></div> })
                            } else {
                                None
                            }}
                            <button 
                                class=format!(
                                    "w-full text-left px-3 py-1.5 hover:bg-gray-50 flex items-center gap-2 {}",
                                    if is_danger { "text-red-600 group" } else { "" }
                                )
                                on:click=move |_| { 
                                    on_action.run(action);
                                    on_close.run(()); 
                                }
                            >
                                <svg 
                                    xmlns="http://www.w3.org/2000/svg" 
                                    fill="none" 
                                    viewBox="0 0 24 24" 
                                    stroke-width="1.5" 
                                    stroke="currentColor" 
                                    class=format!(
                                        "w-4 h-4 {}",
                                        if is_danger { "text-red-500 group-hover:text-red-600" } else { "text-gray-400" }
                                    )
                                >
                                    <path stroke-linecap="round" stroke-linejoin="round" d={icon_path} />
                                </svg>
                                {label}
                            </button>
                        </>
                    }
                }).collect_view()}
            </div>
        </>
    }
}
