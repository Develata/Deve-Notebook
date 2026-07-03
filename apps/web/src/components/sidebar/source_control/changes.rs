// apps\web\src\components\sidebar\source_control
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! # Changes 组件 (变更列表组件)
//!
//! Source Control 只显示已进入 ledger、尚未被 commit anchor 覆盖的变更。

use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use super::confirmed_section::ConfirmedSection;

pub(crate) fn should_request_changes(
    has_repo: bool,
    _remote_branch_active: bool,
    branch_switching: bool,
    repo_switching: bool,
    read_blocked: bool,
) -> bool {
    has_repo && !branch_switching && !repo_switching && !read_blocked
}

/// 变更列表主组件
///
/// 职责:
/// - 触发变更数据获取
/// - 分发数据到子组件
#[component]
pub fn Changes() -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let read_block = core.read_block;

    Effect::new(move |_| {
        if !should_request_changes(
            core.current_repo_id.get().is_some(),
            core.active_branch.get().is_some(),
            core.pending_branch_switch.get().is_some(),
            core.pending_repo_switch.get().is_some(),
            read_block.get().is_some(),
        ) {
            return;
        }
        core.on_get_changes.run(());
    });

    view! {
        <div>
            {move || {
                let confirmed = core.confirmed_changes.get();

                view! {
                    <div>
                        {if confirmed.is_empty() {
                            view! {
                                <div class="px-3 py-6 text-xs text-muted text-center">
                                    {t::source_control::no_changes(locale.get())}
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <ConfirmedSection confirmed=confirmed show_empty_group=true />
                            }.into_any()
                        }}
                    </div>
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::should_request_changes;

    fn changes_component_source() -> &'static str {
        let source = include_str!("changes.rs");
        source
            .split("#[cfg(test)]")
            .next()
            .expect("component source before tests")
    }

    fn assert_confirmed_only_changes_source() {
        let source = changes_component_source();
        assert!(source.contains("use super::confirmed_section::ConfirmedSection;"));
        assert!(source.contains(concat!("<", "ConfirmedSection")));
        assert!(!source.contains("StagedSection"));
        assert!(!source.contains("UnstagedSection"));
        assert!(!source.contains("ExternalChanges"));
        assert!(!source.contains("external_changes"));
    }

    #[test]
    fn mobile_source_control_read_gate_allows_readonly_refresh() {
        assert!(should_request_changes(true, false, false, false, false));
        assert!(!should_request_changes(false, false, false, false, false));
        assert!(!should_request_changes(true, false, true, false, false));
        assert!(!should_request_changes(true, false, false, true, false));
        assert!(!should_request_changes(true, false, false, false, true));
    }

    #[test]
    fn remote_branch_uses_read_gate_for_changes_refresh() {
        assert!(should_request_changes(true, true, false, false, false));
    }

    #[test]
    fn read_block_still_suppresses_changes_refresh() {
        assert!(!should_request_changes(true, false, false, false, true));
    }

    #[test]
    fn source_control_changes_panel_is_confirmed_only() {
        assert_confirmed_only_changes_source();
    }

    #[test]
    fn source_control_confirmed_only_view() {
        assert_confirmed_only_changes_source();
    }
}
