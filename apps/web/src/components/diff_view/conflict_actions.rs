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
                    on:click=move |_| on_resolve.run((MergeConflictAction::AcceptBoth, Some(resolved_content.get_untracked())))
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
