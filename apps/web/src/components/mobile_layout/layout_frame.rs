//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 11_ui_design/03_mobile#mobile-current-native-boundary
//!   - 11_ui_design/03_mobile#mobile-surface-switcher
//!
use super::chat_sheet::MobileChatSheet;
use super::content::MobileContent;
use super::drawers::MobileDrawers;
use super::footer::MobileFooter;
use super::footer_status::pending_ack_count_for_current_scope;
use super::header::MobileHeader;
use super::layout_backdrop::MobileDrawerBackdrop;
use super::layout_banner::MobileSyncBanner;
use super::outline_button::OutlineToggleButton;
use super::surface_runtime::collapse_surface_switcher_on_runtime_transition;
use super::surface_switcher::{MobileSurfaceSwitcher, mobile_surface_sheet_visible};
use super::toolbar::MobileAccessoryToolbar;
use crate::components::activity_bar::SidebarView;
use crate::components::editor_tabs::{
    EditorTabRuntimeInputs, create_current_editor_doc, create_editor_tab_runtime,
};
use crate::components::ui_back::{UiBackCoordinator, UiBackLayer};
use crate::hooks::use_core::EditorContext;
#[cfg(test)]
use crate::hooks::use_core::source_control_notice::{SourceControlNotice, is_local_command_notice};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_tracked};
use crate::i18n::Locale;
use crate::runtime::{
    document_client::DocumentClient, rendering_client::RenderingClient, scope_client::ScopeClient,
    session_client::SessionClient, source_control_client::SourceControlClient,
};
use leptos::ev::TouchEvent;
use leptos::prelude::*;

pub(crate) fn mobile_bottom_bar_visible(
    keyboard_offset: i32,
    chat_expanded: bool,
    surface_switcher_sheet_visible: bool,
) -> bool {
    keyboard_offset <= 0 && !chat_expanded && !surface_switcher_sheet_visible
}

pub(crate) fn mobile_accessory_toolbar_visible(
    has_doc: bool,
    diff_open: bool,
    drawer_open: bool,
    keyboard_offset: i32,
    chat_expanded: bool,
    surface_switcher_sheet_visible: bool,
) -> bool {
    has_doc
        && !diff_open
        && !drawer_open
        && keyboard_offset > 0
        && !chat_expanded
        && !surface_switcher_sheet_visible
}

#[cfg(test)]
pub(crate) fn should_clear_mobile_source_control_local_notice(
    active_view: SidebarView,
    notice: Option<&SourceControlNotice>,
) -> bool {
    active_view == SidebarView::SourceControl && notice.is_some_and(is_local_command_notice)
}

#[component]
pub fn MobileLayoutFrame(
    locale: RwSignal<Locale>,
    title: Memo<String>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    show_sidebar: ReadSignal<bool>,
    set_show_sidebar: WriteSignal<bool>,
    on_open_left_drawer: Callback<()>,
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
    content_signal: Option<ReadSignal<String>>,
    on_touch_start: Callback<TouchEvent>,
    on_touch_end: Callback<TouchEvent>,
    on_touch_cancel: Callback<()>,
) -> impl IntoView {
    let document = expect_context::<DocumentClient>();
    let editor = expect_context::<EditorContext>();
    let source_control = expect_context::<SourceControlClient>();
    let scope = expect_context::<ScopeClient>();
    let session = expect_context::<SessionClient>();
    let rendering = expect_context::<RenderingClient>();
    let ui_back = expect_context::<UiBackCoordinator>();
    let current_doc = document.current_doc;
    let diff_content = source_control.diff_content;
    let current_editor_doc = create_current_editor_doc(&document, &editor);
    let pending_local_edits = document.pending_local_edits;
    let pending_repo_id = scope.current_repo_id;
    let pending_scope_nonce = scope.current_scope_nonce;
    let mobile_pending_ack_count = Memo::new(move |_| {
        let current_repo_id = pending_repo_id.get();
        pending_ack_count_for_current_scope(
            &pending_local_edits.get(),
            current_doc.get(),
            current_repo_id.as_deref(),
            pending_scope_nonce.get(),
        )
    });
    let tabs = create_editor_tab_runtime(
        EditorTabRuntimeInputs {
            document: document.clone(),
            editor: editor.clone(),
            scope: scope.clone(),
            source_control: source_control.clone(),
        },
        current_editor_doc,
    );
    let surface_doc_tabs = tabs.doc_tabs;
    let surface_diff_tabs = tabs.diff_tabs;
    let surface_switcher_has_tabs = Signal::derive(move || {
        !surface_doc_tabs.get().is_empty() || !surface_diff_tabs.get().is_empty()
    });
    let (surface_switcher_open, set_surface_switcher_open) = signal(false);
    let surface_switcher_sheet_visible = Signal::derive(move || {
        mobile_surface_sheet_visible(
            surface_switcher_open.get(),
            drawer_open.get(),
            surface_switcher_has_tabs.get(),
        )
    });
    let pending_branch_switch = editor.pending_branch_switch;
    let pending_repo_switch = scope.pending_repo_switch;
    collapse_surface_switcher_on_runtime_transition(
        scope.current_repo_id,
        scope.current_scope_nonce,
        scope.active_branch,
        Signal::derive(move || pending_branch_switch.get().is_some()),
        Signal::derive(move || pending_repo_switch.get().is_some()),
        set_surface_switcher_open,
    );
    let set_diff_content_for_back = source_control.set_diff_content;
    ui_back.register(UiBackLayer::TransientSheet, move || {
        if surface_switcher_open.try_get_untracked() == Some(true) {
            set_surface_switcher_open.set(false);
            return true;
        }
        if diff_content.try_get_untracked().flatten().is_some() {
            set_diff_content_for_back.set(None);
            return true;
        }
        false
    });

    view! {
        <div
            data-deve-layout-mode="mobile"
            data-deve-mobile-pending-ack-count=move || mobile_pending_ack_count.get().to_string()
            class="flex flex-col flex-1 overflow-hidden bg-sidebar"
            style="touch-action: pan-y;"
            on:touchstart=move |ev| on_touch_start.run(ev)
            on:touchend=move |ev| on_touch_end.run(ev)
            on:touchcancel=move |_| on_touch_cancel.run(())
        >
            <MobileHeader
                title=title
                on_menu=on_open_left_drawer
                on_home=on_home
                on_open=on_open
                on_command=on_command
                on_logout=on_logout
            />

            <MobileSyncBanner />

            <MobileSurfaceSwitcher
                doc_tabs=tabs.doc_tabs
                diff_tabs=tabs.diff_tabs
                active_tab=tabs.active_tab
                open=surface_switcher_open
                set_open=set_surface_switcher_open
                drawer_open=drawer_open
                on_select_document=tabs.on_select_document
                on_select_diff=tabs.on_select_diff
                on_close_document=tabs.on_close_document
                on_close_diff=tabs.on_close_diff
            />

            <MobileContent
                drawer_open=drawer_open
                current_editor_doc=current_editor_doc
            />

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
                readonly=Signal::derive(move || {
                    repo_write_block_tracked(
                        &session.ws,
                        RepoWriteSignals {
                            load_state: rendering.load_state,
                            is_spectator: scope.is_spectator,
                            handshake_ready: session.handshake_ready,
                            current_repo_id: scope.current_repo_id,
                            current_scope_nonce: scope.current_scope_nonce,
                            active_branch: scope.active_branch,
                            pending_branch_switch: editor.pending_branch_switch,
                            pending_repo_switch: scope.pending_repo_switch,
                        },
                    )
                    .is_some()
                })
                visible=Signal::derive(move || {
                    mobile_accessory_toolbar_visible(
                        current_doc.get().is_some(),
                        diff_content.get().is_some(),
                        drawer_open.get(),
                        keyboard_offset.get(),
                        chat_expanded.get(),
                        surface_switcher_sheet_visible.get(),
                    )
                })
            />

            <MobileChatSheet
                keyboard_offset=keyboard_offset
                drawer_open=drawer_open
                diff_open=Signal::derive(move || diff_content.get().is_some())
                surface_switcher_sheet_visible=surface_switcher_sheet_visible
                expanded=chat_expanded
                set_expanded=set_chat_expanded
            />

            <Show when=move || {
                mobile_bottom_bar_visible(
                    keyboard_offset.get(),
                    chat_expanded.get(),
                    surface_switcher_sheet_visible.get(),
                )
            }>
                    <MobileFooter pending_ack_count=mobile_pending_ack_count />
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests;
