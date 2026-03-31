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

#[component]
pub(super) fn LeftDrawerTabs(
    locale: RwSignal<Locale>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    open: ReadSignal<bool>,
) -> impl IntoView {
    let (show_more, set_show_more) = signal(false);
    let more_menu_ref = NodeRef::<html::Div>::new();

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
                <div class="flex-1 overflow-x-auto">
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
                                        set_active_view
                                        on_open_more=Callback::new(move |_| set_show_more.set(false))
                                    />
                                }
                            }
                        />
                    </div>
                </div>
                <button
                    class="mobile-more-button h-11 min-w-11 px-2 rounded-md bg-panel border border-default text-secondary active:bg-hover active:scale-95 transition-transform duration-150 ease-out"
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
                show_more
                set_show_more
                more_menu_ref
            />
        </div>
    }
}
