// apps\web\src\components\sidebar\source_control
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! # Changes 组件 (变更列表组件)
//!
//! 组合 `StagedSection` 和 `UnstagedSection` 子组件，
//! 显示完整的变更列表视图。

use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use super::confirmed_section::ConfirmedSection;
use super::staged_section::StagedSection;
use super::unstaged_section::UnstagedSection;

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
                let staged = core.staged_changes.get();
                let unstaged = core.unstaged_changes.get();
                let confirmed = core.confirmed_changes.get();

                view! {
                    <div>
                        {if staged.is_empty() && unstaged.is_empty() && confirmed.is_empty() {
                            view! {
                                <div class="px-3 py-6 text-xs text-muted text-center">
                                    {t::source_control::no_changes(locale.get())}
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <StagedSection staged=staged />
                                <UnstagedSection unstaged=unstaged />
                                <ConfirmedSection confirmed=confirmed />
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
}
