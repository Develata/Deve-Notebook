//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!

use crate::components::activity_bar::SidebarView;
use crate::hooks::use_core::{
    SourceControlContext,
    source_control_notice::{SourceControlNotice, is_local_command_notice},
};
use leptos::prelude::*;

pub(super) fn should_clear_mobile_source_control_notice(
    view: SidebarView,
    notice: Option<&SourceControlNotice>,
) -> bool {
    view == SidebarView::SourceControl && notice.is_some_and(is_local_command_notice)
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

pub(super) fn clear_tracked_mobile_source_control_notice_for_view(
    view: SidebarView,
    source_control: Option<&SourceControlContext>,
) {
    if let Some(source_control) = source_control {
        let notice = source_control.notice.get();
        if should_clear_mobile_source_control_notice(view, notice.as_ref()) {
            source_control.clear_notice.run(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_clear_mobile_source_control_notice;
    use crate::components::activity_bar::SidebarView;
    use crate::hooks::use_core::source_control_notice::SourceControlNotice;

    #[test]
    fn mobile_source_control_read_gate_plain_entry_clears_local_command_notice() {
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
        assert!(should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            Some(&source_control_notice),
        ));
        assert!(!should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            None,
        ));
    }

    #[test]
    fn mobile_source_control_read_gate_active_view_tracks_local_command_notice() {
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
        assert!(should_clear_mobile_source_control_notice(
            SidebarView::SourceControl,
            Some(&source_control_notice),
        ));
    }
}
