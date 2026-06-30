//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
use crate::components::activity_bar::SidebarView;
use crate::components::icons::MoreHorizontal;
use crate::hooks::use_core::SourceControlContext;
use crate::hooks::use_core::source_control_notice::is_local_command_notice;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

mod more_menu;
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

pub(super) fn select_mobile_sidebar_view(
    view: SidebarView,
    set_active_view: WriteSignal<SidebarView>,
    on_search: Callback<()>,
    on_view_select: Callback<()>,
    clear_source_control_local_notice: Callback<()>,
) {
    if view == SidebarView::Search {
        on_search.run(());
        return;
    }

    if view == SidebarView::SourceControl {
        clear_source_control_local_notice.run(());
    }

    set_active_view.set(view);
    on_view_select.run(());
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
    on_view_select: Callback<()>,
) -> impl IntoView {
    let source_control = expect_context::<SourceControlContext>();
    let source_control_notice = source_control.notice;
    let clear_source_control_notice = source_control.clear_notice;
    let (show_more, set_show_more) = signal(false);
    let more_menu_ref = NodeRef::<html::Div>::new();
    let select_view = Callback::new(move |view: SidebarView| {
        select_mobile_sidebar_view(
            view,
            set_active_view,
            on_search,
            on_view_select,
            Callback::new(move |_| {
                if source_control_notice
                    .get_untracked()
                    .as_ref()
                    .is_some_and(is_local_command_notice)
                {
                    clear_source_control_notice.run(());
                }
            }),
        );
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
                    type="button"
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
        select_mobile_sidebar_view,
    };
    use crate::components::activity_bar::SidebarView;
    use crate::hooks::use_core::source_control_notice::{
        SourceControlNotice, is_git_status_cli_notice,
    };
    use leptos::prelude::*;

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

    #[test]
    fn mobile_sidebar_tab_selection_closes_drawer_after_non_search_view_switch() {
        let owner = leptos::reactive::owner::Owner::new();
        owner.with(|| {
            let (active_view, set_active_view) = signal(SidebarView::Explorer);
            let (search_opened, set_search_opened) = signal(false);
            let (drawer_closed, set_drawer_closed) = signal(false);

            select_mobile_sidebar_view(
                SidebarView::SourceControl,
                set_active_view,
                Callback::new(move |_| set_search_opened.set(true)),
                Callback::new(move |_| set_drawer_closed.set(true)),
                Callback::new(move |_| ()),
            );

            assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
            assert!(!search_opened.get_untracked());
            assert!(drawer_closed.get_untracked());
        });
    }

    #[test]
    fn mobile_sidebar_search_tab_uses_search_handler_for_close() {
        let owner = leptos::reactive::owner::Owner::new();
        owner.with(|| {
            let (active_view, set_active_view) = signal(SidebarView::Explorer);
            let (search_opened, set_search_opened) = signal(false);
            let (drawer_closed, set_drawer_closed) = signal(false);

            select_mobile_sidebar_view(
                SidebarView::Search,
                set_active_view,
                Callback::new(move |_| set_search_opened.set(true)),
                Callback::new(move |_| set_drawer_closed.set(true)),
                Callback::new(move |_| ()),
            );

            assert_eq!(active_view.get_untracked(), SidebarView::Explorer);
            assert!(search_opened.get_untracked());
            assert!(!drawer_closed.get_untracked());
        });
    }

    #[test]
    fn mobile_source_control_tab_clears_local_git_command_notice() {
        let owner = leptos::reactive::owner::Owner::new();
        owner.with(|| {
            let (active_view, set_active_view) = signal(SidebarView::Explorer);
            let (notice, set_notice) = signal(Some(SourceControlNotice::git_status_cli_only()));
            let (drawer_closed, set_drawer_closed) = signal(false);

            select_mobile_sidebar_view(
                SidebarView::SourceControl,
                set_active_view,
                Callback::new(move |_| ()),
                Callback::new(move |_| set_drawer_closed.set(true)),
                Callback::new(move |_| {
                    if notice
                        .get_untracked()
                        .as_ref()
                        .is_some_and(is_git_status_cli_notice)
                    {
                        set_notice.set(None);
                    }
                }),
            );

            assert_eq!(active_view.get_untracked(), SidebarView::SourceControl);
            assert!(drawer_closed.get_untracked());
            assert_eq!(notice.get_untracked(), None);
        });
    }
}
