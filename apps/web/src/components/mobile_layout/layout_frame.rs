//! plan_ref:
//!   - 11_ui_design_03_mobile#mobile-responsive-layout
//!   - 11_ui_design_03_mobile#mobile-current-native-boundary
//!
use super::chat_sheet::MobileChatSheet;
use super::content::MobileContent;
use super::drawers::MobileDrawers;
use super::footer::MobileFooter;
use super::header::MobileHeader;
use super::layout_backdrop::MobileDrawerBackdrop;
use super::layout_banner::MobileSyncBanner;
use super::outline_button::OutlineToggleButton;
use super::toolbar::MobileAccessoryToolbar;
use crate::components::activity_bar::SidebarView;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::write_gate::repo_write_allowed_for_core_tracked;
use crate::i18n::Locale;
use leptos::ev::TouchEvent;
use leptos::prelude::*;

pub(crate) fn mobile_bottom_bar_visible(keyboard_offset: i32, chat_expanded: bool) -> bool {
    keyboard_offset <= 0 && !chat_expanded
}

pub(crate) fn mobile_accessory_toolbar_visible(
    has_doc: bool,
    diff_open: bool,
    drawer_open: bool,
    keyboard_offset: i32,
    chat_expanded: bool,
) -> bool {
    has_doc && !diff_open && !drawer_open && keyboard_offset > 0 && !chat_expanded
}

#[component]
pub fn MobileLayoutFrame(
    core: CoreState,
    locale: RwSignal<Locale>,
    title: Memo<String>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    show_sidebar: ReadSignal<bool>,
    set_show_sidebar: WriteSignal<bool>,
    show_outline: ReadSignal<bool>,
    set_show_outline: WriteSignal<bool>,
    drawer_open: Signal<bool>,
    keyboard_offset: ReadSignal<i32>,
    chat_expanded: ReadSignal<bool>,
    set_chat_expanded: WriteSignal<bool>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_logout: Callback<()>,
    on_doc_select: Callback<deve_core::models::DocId>,
    on_close_drawers: Callback<()>,
    banner_toggle: CoreState,
    banner_text: CoreState,
    content_signal: Option<ReadSignal<String>>,
    on_touch_start: Callback<TouchEvent>,
    on_touch_end: Callback<TouchEvent>,
    on_touch_cancel: Callback<()>,
) -> impl IntoView {
    let current_doc = core.current_doc;
    let diff_content = core.diff_content;
    let toolbar_core = core.clone();

    view! {
        <div
            data-deve-layout-mode="mobile"
            class="flex flex-col flex-1 overflow-hidden bg-sidebar"
            style="touch-action: pan-y;"
            on:touchstart=move |ev| on_touch_start.run(ev)
            on:touchend=move |ev| on_touch_end.run(ev)
            on:touchcancel=move |_| on_touch_cancel.run(())
        >
            <MobileHeader
                title=title
                on_menu=Callback::new(move |_| {
                    set_show_outline.set(false);
                    set_show_sidebar.set(true);
                })
                on_home=on_home
                on_open=on_open
                on_command=on_command
                on_logout=on_logout
            />

            <MobileSyncBanner banner_toggle=banner_toggle banner_text=banner_text />

            <MobileContent core=core.clone() drawer_open=drawer_open />

            <Show when=move || current_doc.get().is_some() && diff_content.get().is_none() && !show_sidebar.get()>
                <OutlineToggleButton
                    show_outline=show_outline
                    set_show_outline=set_show_outline
                    set_show_sidebar=set_show_sidebar
                    locale=locale
                />
            </Show>

            <MobileDrawerBackdrop drawer_open=drawer_open on_close=on_close_drawers />

            <MobileDrawers
                core=core.clone()
                active_view=active_view
                set_active_view=set_active_view
                pinned_views=pinned_views
                set_pinned_views=set_pinned_views
                show_sidebar=show_sidebar
                show_outline=show_outline
                on_doc_select=on_doc_select
                on_close=on_close_drawers
                content_signal=content_signal
            />

            <MobileAccessoryToolbar
                keyboard_offset=keyboard_offset
                readonly=Signal::derive(move || !repo_write_allowed_for_core_tracked(&toolbar_core))
                visible=Signal::derive(move || {
                    mobile_accessory_toolbar_visible(
                        current_doc.get().is_some(),
                        diff_content.get().is_some(),
                        drawer_open.get(),
                        keyboard_offset.get(),
                        chat_expanded.get(),
                    )
                })
            />

            <MobileChatSheet
                keyboard_offset=keyboard_offset
                drawer_open=drawer_open
                diff_open=Signal::derive(move || diff_content.get().is_some())
                expanded=chat_expanded
                set_expanded=set_chat_expanded
            />

            <Show when=move || mobile_bottom_bar_visible(keyboard_offset.get(), chat_expanded.get())>
                <MobileFooter core=core.clone() />
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{mobile_accessory_toolbar_visible, mobile_bottom_bar_visible};

    #[test]
    fn mobile_chat_keyboard_hides_bottom_bar() {
        assert!(!mobile_bottom_bar_visible(280, true));
        assert!(!mobile_bottom_bar_visible(280, false));
        assert!(!mobile_bottom_bar_visible(0, true));
        assert!(mobile_bottom_bar_visible(0, false));
    }

    #[test]
    fn mobile_diff_hides_accessory_toolbar() {
        assert!(mobile_accessory_toolbar_visible(
            true, false, false, 280, false
        ));
        assert!(!mobile_accessory_toolbar_visible(
            true, true, false, 280, false
        ));
    }

    #[test]
    fn mobile_diff_keeps_accessory_toolbar_gate_strict() {
        assert!(!mobile_accessory_toolbar_visible(
            false, false, false, 280, false
        ));
        assert!(!mobile_accessory_toolbar_visible(
            true, false, true, 280, false
        ));
        assert!(!mobile_accessory_toolbar_visible(
            true, false, false, 0, false
        ));
        assert!(!mobile_accessory_toolbar_visible(
            true, false, false, 280, true
        ));
    }
}
