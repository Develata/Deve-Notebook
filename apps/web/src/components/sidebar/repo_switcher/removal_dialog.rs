//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract
//!   - 13_i18n#i18n-resource-management

use crate::components::focus_scope;
use crate::components::ui_back::{UiBackCoordinator, UiBackLayer};
use crate::hooks::use_core::BranchContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub(super) fn RepoRemovalDialog(
    fallback_focus_ref: NodeRef<leptos::html::Button>,
) -> impl IntoView {
    let core = expect_context::<BranchContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let cancel_button_ref = NodeRef::<leptos::html::Button>::new();
    let ui_back = expect_context::<UiBackCoordinator>();
    let core_for_back = core.clone();
    ui_back.register(UiBackLayer::Overlay, move || {
        if let Some(preview) = core_for_back.removal_preview.try_get_untracked().flatten() {
            core_for_back.on_cancel_remove_repo.run(preview.repo_id);
            return true;
        }
        false
    });
    focus_scope::attach_modal_focus_restore_effect_with_fallback(
        move || core.removal_preview.get().is_some(),
        cancel_button_ref,
        fallback_focus_ref,
    );

    view! {
        <Show when=move || core.removal_preview.get().is_some()>
            <div
                class="fixed inset-0 z-[var(--z-modal)] flex items-end justify-center bg-black/55 p-0 backdrop-blur-sm sm:items-center sm:p-6"
                data-deve-repo-removal-dialog="visible"
            >
                <div
                    node_ref=panel_ref
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="repo-removal-title"
                    tabindex="-1"
                    class="flex max-h-[92vh] w-full max-w-2xl flex-col overflow-hidden rounded-t-xl border border-default bg-panel shadow-2xl sm:max-h-[84vh] sm:rounded-xl"
                    on:keydown=move |event| {
                        if event.key() == "Escape"
                            && let Some(preview) = core.removal_preview.get_untracked()
                        {
                            core.on_cancel_remove_repo.run(preview.repo_id);
                            focus_scope::focus_button_next_frame(fallback_focus_ref);
                            return;
                        }
                        let _ = focus_scope::handle_focus_trap_keydown(&event, panel_ref);
                    }
                >
                    <div class="border-b border-default px-5 py-4 sm:px-6">
                        <p class="text-xs font-semibold uppercase tracking-[0.12em] text-red-600">
                            {move || t::repo_control::remove_title(locale.get())}
                        </p>
                        <h2 id="repo-removal-title" class="mt-1 text-lg font-semibold text-primary sm:text-xl">
                            {move || core.removal_preview.get().map(|preview| preview.display_alias).unwrap_or_default()}
                        </h2>
                        <p class="mt-1 text-sm text-secondary">
                            {move || core.removal_preview.get().map(|preview| {
                                t::repo_control::remove_subject(locale.get(), &preview.display_alias)
                            }).unwrap_or_default()}
                        </p>
                    </div>

                    <div class="overflow-y-auto px-5 py-4 sm:px-6">
                        <div class="border-l-4 border-red-600 bg-red-50 px-4 py-3 text-sm text-red-950">
                            {move || t::repo_control::irreversible_warning(locale.get())}
                        </div>

                        <div class="mt-5 grid gap-5 sm:grid-cols-2 sm:gap-6">
                            <section aria-labelledby="repo-removal-deleted-heading">
                                <h3 id="repo-removal-deleted-heading" class="text-sm font-semibold text-red-600">
                                    {move || t::repo_control::deleted_heading(locale.get())}
                                </h3>
                                <ul class="mt-2 space-y-2 text-sm text-primary">
                                    <For
                                        each=move || core.removal_preview.get().map(|value| value.preview.deleted).unwrap_or_default()
                                        key=|value| format!("{value:?}")
                                        children=move |value| view! {
                                            <li class="flex gap-2">
                                                <span aria-hidden="true" class="text-red-600">"−"</span>
                                                <span>{move || t::repo_control::deleted(locale.get(), value)}</span>
                                            </li>
                                        }
                                    />
                                </ul>
                            </section>

                            <section aria-labelledby="repo-removal-preserved-heading">
                                <h3 id="repo-removal-preserved-heading" class="text-sm font-semibold text-primary">
                                    {move || t::repo_control::preserved_heading(locale.get())}
                                </h3>
                                <ul class="mt-2 space-y-2 text-sm text-secondary">
                                    <For
                                        each=move || core.removal_preview.get().map(|value| value.preview.preserved).unwrap_or_default()
                                        key=|value| format!("{value:?}")
                                        children=move |value| view! {
                                            <li class="flex gap-2">
                                                <span aria-hidden="true" class="text-green-600">"✓"</span>
                                                <span>{move || t::repo_control::preserved(locale.get(), value)}</span>
                                            </li>
                                        }
                                    />
                                </ul>
                            </section>
                        </div>

                        <Show when=move || core.removal_preview.get().is_some_and(|value| !value.preview.warnings.is_empty())>
                            <section class="mt-5 border-t border-default pt-4" aria-labelledby="repo-removal-warnings-heading">
                                <h3 id="repo-removal-warnings-heading" class="text-sm font-semibold text-primary">
                                    {move || t::repo_control::warnings_heading(locale.get())}
                                </h3>
                                <ul class="mt-2 space-y-1.5 text-sm text-secondary">
                                    <For
                                        each=move || core.removal_preview.get().map(|value| value.preview.warnings).unwrap_or_default()
                                        key=|value| format!("{value:?}")
                                        children=move |value| view! {
                                            <li>{move || t::repo_control::warning(locale.get(), value)}</li>
                                        }
                                    />
                                </ul>
                            </section>
                        </Show>

                        <Show when=move || core.removal_preview.get().is_some_and(|value| !value.preview.blockers.is_empty())>
                            <section class="mt-5 border border-amber-400/70 bg-amber-100/80 px-4 py-3 text-amber-950" aria-labelledby="repo-removal-blockers-heading">
                                <h3 id="repo-removal-blockers-heading" class="text-sm font-semibold">
                                    {move || t::repo_control::blockers_heading(locale.get())}
                                </h3>
                                <p class="mt-1 text-sm">{move || t::repo_control::blocked(locale.get())}</p>
                                <ul class="mt-2 list-disc space-y-1 pl-5 text-sm">
                                    <For
                                        each=move || core.removal_preview.get().map(|value| value.preview.blockers).unwrap_or_default()
                                        key=|value| format!("{value:?}")
                                        children=move |value| view! {
                                            <li>{move || t::repo_control::blocker(locale.get(), value)}</li>
                                        }
                                    />
                                </ul>
                            </section>
                        </Show>
                    </div>

                    <div class="grid grid-cols-1 gap-3 border-t border-default px-5 pt-4 pb-[max(1rem,var(--deve-safe-area-bottom))] sm:grid-cols-2 sm:px-6 sm:pb-4">
                        <button
                            node_ref=cancel_button_ref
                            type="button"
                            class="min-h-[44px] rounded-lg border border-default px-4 py-2 text-sm font-medium text-primary hover:bg-hover focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
                            on:click=move |_| {
                                if let Some(preview) = core.removal_preview.get_untracked() {
                                    core.on_cancel_remove_repo.run(preview.repo_id);
                                    focus_scope::focus_button_next_frame(fallback_focus_ref);
                                }
                            }
                        >
                            {move || t::repo_control::cancel(locale.get())}
                        </button>
                        <button
                            type="button"
                            data-deve-repo-removal-confirm="true"
                            class="min-h-[44px] rounded-lg bg-red-600 px-4 py-2 text-sm font-semibold text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
                            disabled=move || !core.removal_preview.get().is_some_and(|value| value.can_execute)
                            on:click=move |_| {
                                if let Some(preview) = core.removal_preview.get_untracked()
                                    && preview.can_execute
                                {
                                    core.on_confirm_remove_repo.run(preview.repo_id);
                                }
                            }
                        >
                            {move || t::repo_control::confirm(locale.get())}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
