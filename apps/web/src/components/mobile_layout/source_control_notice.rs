//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!

use crate::components::activity_bar::SidebarView;
use crate::hooks::use_core::{
    SourceControlContext,
    source_control_notice::{SourceControlNotice, is_git_cli_notice},
};
use leptos::prelude::*;

pub(super) fn should_clear_mobile_source_control_notice(
    view: SidebarView,
    notice: Option<&SourceControlNotice>,
) -> bool {
    view == SidebarView::SourceControl && notice.is_some_and(is_git_cli_notice)
}

pub(super) fn should_clear_mobile_source_control_notice_for_drawer(
    open: bool,
    view: SidebarView,
    notice: Option<&SourceControlNotice>,
) -> bool {
    open && should_clear_mobile_source_control_notice(view, notice)
}

pub(super) fn should_observe_mobile_source_control_notice_for_drawer(
    open: bool,
    view: SidebarView,
) -> bool {
    open && view == SidebarView::SourceControl
}

pub(super) fn clear_mobile_source_control_notice_for_view(
    view: SidebarView,
    source_control: Option<&SourceControlContext>,
) {
    if let Some(source_control) = source_control
        && should_clear_mobile_source_control_notice(
            view,
            source_control.notice.get_untracked().as_ref(),
        )
    {
        source_control.clear_notice.run(());
    }
}

pub(super) fn clear_mobile_source_control_notice_for_drawer(
    open: bool,
    view: SidebarView,
    source_control: Option<&SourceControlContext>,
) {
    if let Some(source_control) = source_control
        && should_observe_mobile_source_control_notice_for_drawer(open, view)
    {
        let notice = source_control.notice.get();
        if should_clear_mobile_source_control_notice_for_drawer(open, view, notice.as_ref()) {
            source_control.clear_notice.run(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        should_clear_mobile_source_control_notice,
        should_clear_mobile_source_control_notice_for_drawer,
        should_observe_mobile_source_control_notice_for_drawer,
    };
    use crate::components::activity_bar::SidebarView;
    use crate::hooks::use_core::source_control_notice::SourceControlNotice;

    #[test]
    fn mobile_source_control_read_gate_plain_entry_clears_only_git_cli_notice() {
        let git_notice = SourceControlNotice::git_status_cli_only();
        let source_control_notice = SourceControlNotice::establish_branch_unavailable();

        assert!(should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            Some(&git_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice(
            SidebarView::Explorer,
            Some(&git_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            Some(&source_control_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            None,
        ));
    }

    #[test]
    fn mobile_source_control_open_read_surface_observes_notice_changes() {
        let git_notice = SourceControlNotice::git_status_cli_only();
        let source_control_notice = SourceControlNotice::establish_branch_unavailable();

        assert!(should_observe_mobile_source_control_notice_for_drawer(
            true,
            SidebarView::SourceControl,
        ));
        assert!(!should_observe_mobile_source_control_notice_for_drawer(
            false,
            SidebarView::SourceControl,
        ));
        assert!(!should_observe_mobile_source_control_notice_for_drawer(
            true,
            SidebarView::Explorer,
        ));
        assert!(should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            Some(&git_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice(
            SidebarView::Explorer,
            Some(&git_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            Some(&source_control_notice),
        ));
    }

    #[test]
    fn mobile_source_control_read_gate_drawer_activation_clears_stale_git_cli_notice() {
        let git_notice = SourceControlNotice::git_status_cli_only();
        let source_control_notice = SourceControlNotice::establish_branch_unavailable();

        assert!(should_clear_mobile_source_control_notice_for_drawer(
            true,
            SidebarView::SourceControl,
            Some(&git_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice_for_drawer(
            false,
            SidebarView::SourceControl,
            Some(&git_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice_for_drawer(
            true,
            SidebarView::Explorer,
            Some(&git_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice_for_drawer(
            true,
            SidebarView::SourceControl,
            Some(&source_control_notice),
        ));
    }
}
