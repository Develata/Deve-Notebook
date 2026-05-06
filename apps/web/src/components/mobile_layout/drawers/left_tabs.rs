//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-interaction-design
//!
use crate::components::activity_bar::SidebarView;
use crate::components::icons::MoreHorizontal;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

#[path = "left_more_menu.rs"]
mod more_menu;
#[path = "left_tab_button.rs"]
mod tab_button;

use more_menu::LeftDrawerMoreMenu;
use tab_button::LeftDrawerTabButton;

pub(super) fn mobile_sidebar_icon_tabs_marker(open: bool) -> Option<&'static str> {
    open.then_some("visible")
}

pub(super) fn mobile_more_button_marker() -> &'static str {
    "mobile-more-button"
}

pub(super) fn mobile_more_button_class() -> &'static str {
    "mobile-more-button h-11 min-w-[44px] px-2 rounded-md bg-panel border border-default text-secondary active:bg-hover active:scale-95 transition-transform duration-150 ease-out"
}

#[component]
pub(super) fn LeftDrawerTabs(
    locale: RwSignal<Locale>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    open: ReadSignal<bool>,
    on_search: Callback<()>,
) -> impl IntoView {
    let (show_more, set_show_more) = signal(false);
    let more_menu_ref = NodeRef::<html::Div>::new();
    let select_view = Callback::new(move |view: SidebarView| {
        if view == SidebarView::Search {
            on_search.run(());
        } else {
            set_active_view.set(view);
        }
        set_show_more.set(false);
    });

    Effect::new(move |_| {
        if !open.get() {
            set_show_more.set(false);
        }
    });

    Effect::new(move |_| {
        if show_more.get()
            && let Some(el) = more_menu_ref.get()
        {
            let _ = el.focus();
        }
    });

    view! {
        <div class="px-2 py-2 border-b border-default relative">
            <div class="flex items-center gap-2 w-full">
                <div
                    class="flex-1 overflow-x-auto"
                    data-deve-mobile-sidebar-icon-tabs=move || mobile_sidebar_icon_tabs_marker(open.get())
                >
                    <div class="flex items-center gap-2 min-w-max">
                        <For
                            each=move || pinned_views.get()
                            key=|v| *v
                            children=move |view| {
                                view! {
                                    <LeftDrawerTabButton
                                        locale
                                        view
                                        active_view
                                        select_view
                                        on_open_more=Callback::new(move |_| set_show_more.set(false))
                                    />
                                }
                            }
                        />
                    </div>
                </div>
                <button
                    class=mobile_more_button_class()
                    data-deve-mobile-sidebar-action=mobile_more_button_marker()
                    data-deve-mobile-touch-target=mobile_more_button_marker()
                    title=move || t::sidebar::more(locale.get())
                    aria-label=move || t::sidebar::more(locale.get())
                    on:click=move |_| set_show_more.update(|v| *v = !*v)
                >
                    <MoreHorizontal class="w-[18px] h-[18px] mx-auto"/>
                </button>
            </div>

            <LeftDrawerMoreMenu
                locale
                active_view
                pinned_views
                set_pinned_views
                select_view
                show_more
                set_show_more
                more_menu_ref
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_more_button_class, mobile_more_button_marker, mobile_sidebar_icon_tabs_marker,
    };

    #[test]
    fn mobile_sidebar_icon_tabs_marker_is_visible_when_drawer_open() {
        assert_eq!(mobile_sidebar_icon_tabs_marker(true), Some("visible"));
        assert_eq!(mobile_sidebar_icon_tabs_marker(false), None);
    }

    #[test]
    fn mobile_sidebar_more_button_marker_is_stable() {
        assert_eq!(mobile_more_button_marker(), "mobile-more-button");
    }

    #[test]
    fn mobile_sidebar_more_button_is_at_least_44px() {
        let class = mobile_more_button_class();

        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }
}
