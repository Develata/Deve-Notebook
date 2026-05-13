//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::hooks::use_core::diff_session::MergeConflictSession;
use crate::i18n::{Locale, t};
use deve_core::protocol::MergeConflictAction;
use leptos::prelude::*;

#[component]
pub fn MergeConflictActions(
    mobile: bool,
    conflict: MergeConflictSession,
    resolved_content: Signal<String>,
    is_editing: ReadSignal<bool>,
    accept_both_content: String,
    on_resolve: Callback<(MergeConflictAction, Option<String>)>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let has_current = conflict
        .actions
        .contains(&MergeConflictAction::AcceptCurrent);
    let has_incoming = conflict
        .actions
        .contains(&MergeConflictAction::AcceptIncoming);
    let has_both = conflict.actions.contains(&MergeConflictAction::AcceptBoth);
    let accept_both_content_for_click = StoredValue::new(accept_both_content);

    view! {
        <div class=move || if mobile {
            "flex-none border-b border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-3 py-2 flex flex-wrap gap-2"
        } else {
            "flex-none border-b border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-4 py-2 flex items-center gap-2"
        }>
            <span class="text-xs font-semibold text-[var(--diff-fg)]">
                {move || t::diff::merge_conflict(locale.get())}
            </span>
            <Show when=move || has_current>
                <button
                    class=button_class(mobile)
                    data-deve-merge-action="accept-current"
                    on:click=move |_| on_resolve.run((MergeConflictAction::AcceptCurrent, None))
                >
                    {move || t::diff::accept_current(locale.get())}
                </button>
            </Show>
            <Show when=move || has_incoming>
                <button
                    class=button_class(mobile)
                    data-deve-merge-action="accept-incoming"
                    on:click=move |_| on_resolve.run((MergeConflictAction::AcceptIncoming, None))
                >
                    {move || t::diff::accept_incoming(locale.get())}
                </button>
            </Show>
            <Show when=move || has_both>
                <button
                    class=button_class(mobile)
                    data-deve-merge-action="accept-both"
                    on:click=move |_| {
                        let content = if is_editing.get_untracked() {
                            resolved_content.get_untracked()
                        } else {
                            accept_both_content_for_click.get_value()
                        };
                        on_resolve.run((MergeConflictAction::AcceptBoth, Some(content)));
                    }
                >
                    {move || t::diff::accept_result(locale.get())}
                </button>
            </Show>
        </div>
    }
}

fn button_class(mobile: bool) -> &'static str {
    if mobile {
        "h-9 px-3 rounded border border-[var(--diff-border)] text-xs text-[var(--diff-fg)] active:bg-[var(--diff-btn-hover)]"
    } else {
        "px-3 py-1 rounded border border-[var(--diff-border)] text-xs text-[var(--diff-fg)] hover:bg-[var(--diff-btn-hover)]"
    }
}

pub(crate) fn accept_both_content(current: &str, incoming: &str) -> String {
    if current.is_empty() || incoming.is_empty() || current.ends_with('\n') {
        format!("{current}{incoming}")
    } else {
        format!("{current}\n{incoming}")
    }
}

#[cfg(test)]
mod tests {
    use super::accept_both_content;

    #[test]
    fn accept_both_joins_current_and_incoming_with_single_line_break() {
        assert_eq!(accept_both_content("local", "remote"), "local\nremote");
        assert_eq!(accept_both_content("local\n", "remote"), "local\nremote");
    }

    #[test]
    fn accept_both_handles_empty_sides_without_extra_separator() {
        assert_eq!(accept_both_content("", "remote"), "remote");
        assert_eq!(accept_both_content("local", ""), "local");
    }
}
