//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!
use crate::components::activity_bar::SidebarView;
use crate::components::main_layout::SidebarControl;
use crate::hooks::use_core::{
    SourceControlContext,
    source_control_notice::{SourceControlNotice, is_local_command_notice},
};
use leptos::prelude::*;

pub(super) fn show_source_control_notice(
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
    notice: SourceControlNotice,
    set_show: WriteSignal<bool>,
) {
    if let Some(set_notice) = set_notice {
        set_notice.set(Some(notice));
    }
    if let Some(sidebar_control) = sidebar_control {
        sidebar_control.show_view(SidebarView::SourceControl);
    }
    set_show.set(false);
}

pub(super) fn show_ngit_status_notice(
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
    source_control: Option<SourceControlContext>,
    set_show: WriteSignal<bool>,
) {
    show_ngit_status_notice_for_viewport(
        set_notice,
        sidebar_control,
        source_control,
        set_show,
        current_command_surface_maps_to_mobile(),
    );
}

pub(super) fn show_ngit_status_notice_for_viewport(
    set_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
    sidebar_control: Option<SidebarControl>,
    source_control: Option<SourceControlContext>,
    set_show: WriteSignal<bool>,
    viewport_mobile: bool,
) {
    let is_mobile = sidebar_control
        .as_ref()
        .is_some_and(|sidebar_control| sidebar_control.is_mobile.get_untracked())
        || viewport_mobile;
    if is_mobile {
        clear_local_command_notice(source_control.as_ref());
        if let Some(sidebar_control) = sidebar_control {
            sidebar_control.show_view(SidebarView::SourceControl);
        }
        set_show.set(false);
        return;
    }
    if let Some(set_notice) = set_notice {
        set_notice.set(Some(SourceControlNotice::git_status_cli_only()));
    }
    if let Some(sidebar_control) = sidebar_control {
        sidebar_control.show_view(SidebarView::SourceControl);
    }
    set_show.set(false);
}

fn clear_local_command_notice(source_control: Option<&SourceControlContext>) {
    let Some(source_control) = source_control else {
        return;
    };
    if source_control
        .notice
        .get_untracked()
        .as_ref()
        .is_some_and(is_local_command_notice)
    {
        source_control.clear_notice.run(());
    }
}

fn current_command_surface_maps_to_mobile() -> bool {
    crate::components::layout_breakpoint::current_command_surface_maps_to_mobile()
}
