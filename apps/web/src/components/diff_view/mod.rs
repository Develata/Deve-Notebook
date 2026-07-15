//! Backend-projected diff renderer.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 10_rendering#large-document-runtime

mod conflict_actions;
mod projection;
mod projection_model;
mod projection_row;
mod projection_text;
mod surface;
mod title;

use self::conflict_actions::MergeConflictActions;
use self::surface::ProjectionSurface;
use self::title::diff_title;
use crate::components::icons::X;
use crate::i18n::{Locale, t};
use crate::runtime::source_control_client::diff_session::{
    DiffProjectionIntent, DiffProjectionStatus, DiffSessionWire, next_diff_revision,
};
use deve_core::protocol::{MergeConflictAction, ServerError, ServerErrorCode};
use gloo_timers::callback::Timeout;
use leptos::prelude::*;

const DIFF_EDIT_DEBOUNCE_MS: u32 = 150;
const DIFF_CLOSE_BUTTON_CLASS: &str = "diff-close-button min-h-11 min-w-11 rounded p-1 text-[var(--diff-muted)] hover:bg-[var(--diff-btn-hover)]";

pub(crate) fn diff_view_class(mobile: bool) -> &'static str {
    if mobile {
        "diff-view-mobile h-full w-full bg-[var(--diff-bg)] flex flex-col font-mono text-[13px]"
    } else {
        "h-full w-full bg-[var(--diff-bg)] flex flex-col font-mono text-[13px]"
    }
}

#[component]
pub fn DiffView(
    session: DiffSessionWire,
    #[prop(default = false)] is_readonly: bool,
    #[prop(default = false)] force_unified: bool,
    #[prop(default = false)] mobile: bool,
    on_compute_projection: Option<Callback<DiffProjectionIntent>>,
    on_persist_draft: Option<Callback<String>>,
    on_resolve_merge_conflict: Option<Callback<(MergeConflictAction, Option<String>)>>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let filename = diff_title(&session.path, &session.display_path);
    let projection = session.projection.clone();
    let initial_draft = session
        .draft_content
        .clone()
        .or_else(|| projection.as_ref().map(|p| p.target_content.clone()))
        .unwrap_or_default();
    let (is_editing, set_is_editing) = signal(false);
    let (draft, set_draft) = signal(initial_draft);
    let (status, set_status) = signal(session.status.clone());
    let initial_revision = session.latest_revision;
    let debounce_timer = StoredValue::new_local(None::<Timeout>);

    let submit = {
        let projection = projection.clone();
        move |target_content: String| {
            let next_revision = next_diff_revision();
            let request_id = uuid::Uuid::new_v4().to_string();
            let intent = DiffProjectionIntent {
                request_id: request_id.clone(),
                revision: next_revision,
                base_content: projection
                    .as_ref()
                    .map(|projection| projection.base_content.clone())
                    .unwrap_or_default(),
                target_content,
            };
            if let Some(on_compute) = on_compute_projection {
                set_status.set(DiffProjectionStatus::Computing {
                    request_id,
                    revision: next_revision,
                });
                on_compute.run(intent);
            } else {
                set_status.set(DiffProjectionStatus::Unavailable(ServerError::new(
                    ServerErrorCode::DiffComputeFailed,
                )));
            }
        }
    };
    // The callback and timer are owner-scoped arena values. Closing a transient
    // diff view disposes both before a delayed callback can touch its signals.
    // Projection requests are driven by user input only; mounting a view must
    // not enqueue an unsolicited recomputation.
    let submit = StoredValue::new_local(submit);

    let retry = (projection.is_some() && on_compute_projection.is_some()).then(|| {
        Callback::new(move |_| {
            let Some(content) = draft.try_get_untracked() else {
                return;
            };
            let _ = submit.try_with_value(|submit| submit(content));
        })
    });
    let added = projection.as_ref().map_or(0, |p| p.added_lines);
    let deleted = projection.as_ref().map_or(0, |p| p.deleted_lines);
    let algorithm = projection.as_ref().map(|p| match p.algorithm {
        deve_core::source_control::diff_projection::DiffAlgorithm::Myers => "Myers".to_string(),
        deve_core::source_control::diff_projection::DiffAlgorithm::PatienceMyers => {
            "Patience+Myers".to_string()
        }
    });
    let compute_ms = projection.as_ref().map_or(0, |p| {
        p.compute_micros.div_ceil(1000).min(u32::MAX as u64) as u32
    });
    let merge_conflict = session.merge_conflict.clone();

    view! {
        <div class=move || diff_view_class(mobile) data-deve-diff-projection="backend-typed">
            <div class=move || if mobile {
                "flex-none border-b border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-3 py-2"
            } else {
                "flex-none h-10 border-b border-[var(--diff-border)] flex items-center justify-between px-4 bg-[var(--diff-header-bg)]"
            }>
                <div class="flex min-w-0 items-center gap-2">
                    <span class="font-semibold text-[var(--diff-fg)]">{move || format!("{}:", t::diff::title(locale.get()))}</span>
                    <span class="truncate text-[var(--diff-filename)]" title=filename.clone()>{filename.clone()}</span>
                    <span class="rounded bg-[var(--diff-line-add)] px-1.5 py-0.5 text-[11px]" title=move || t::diff::added(locale.get())>{format!("+{added}")}</span>
                    <span class="rounded bg-[var(--diff-line-del)] px-1.5 py-0.5 text-[11px]" title=move || t::diff::deleted(locale.get())>{format!("-{deleted}")}</span>
                    {algorithm.map(|algorithm| view! {
                        <span class="hidden text-[11px] text-[var(--diff-muted)] md:inline" title=move || t::diff::algorithm_help(locale.get())>{move || t::diff::algorithm(locale.get(), &algorithm)}</span>
                    })}
                    <span class="hidden text-[11px] text-[var(--diff-muted)] md:inline" title=move || t::diff::compute_ms_help(locale.get())>{move || t::diff::compute_ms(locale.get(), compute_ms)}</span>
                    <Show when=move || is_readonly>
                        <span class="rounded bg-[var(--diff-pill-bg)] px-2 py-0.5 text-xs text-[var(--diff-pill-fg)]">{move || t::diff::read_only(locale.get())}</span>
                    </Show>
                </div>
                <div class="flex items-center gap-2">
                    <Show when=move || !is_readonly>
                        <button
                            class="diff-edit-toggle rounded border border-[var(--diff-border)] px-3 py-1 text-xs text-[var(--diff-fg)] hover:bg-[var(--diff-btn-hover)]"
                            on:click=move |_| set_is_editing.update(|editing| *editing = !*editing)
                        >
                            {move || if is_editing.get() { t::diff::preview_diff(locale.get()) } else { t::diff::edit(locale.get()) }}
                        </button>
                    </Show>
                    <button
                        data-deve-mobile-diff-action="diff-close-button"
                        class=DIFF_CLOSE_BUTTON_CLASS
                        on:click=move |_| on_close.run(())
                        title=move || t::diff::close_diff_view(locale.get())
                    ><X class="h-5 w-5"/></button>
                </div>
            </div>

            {merge_conflict.and_then(|conflict| {
                on_resolve_merge_conflict.map(|on_resolve| view! {
                    <MergeConflictActions
                        mobile
                        conflict
                        resolved_content=draft
                        on_resolve
                    />
                })
            })}

            <div class="relative flex-1 min-h-0">
                <Show
                    when=move || is_editing.get()
                    fallback=move || view! {
                        <ProjectionSurface
                            projection=projection.clone()
                            status=status.get()
                            force_unified
                            on_retry=retry
                        />
                    }
                >
                    <textarea
                        name="diff-edit-draft"
                        data-deve-diff-draft="true"
                        class="h-full w-full resize-none border-0 bg-[var(--diff-bg)] p-3 font-mono text-[13px] text-[var(--diff-fg)] outline-none"
                        prop:value=move || draft.get()
                        on:input=move |event| {
                            let content = event_target_value(&event);
                            set_draft.set(content.clone());
                            let pending_revision = initial_revision.saturating_add(1);
                            if set_status
                                .try_set(DiffProjectionStatus::Debouncing {
                                    revision: pending_revision,
                                })
                                .is_some()
                            {
                                return;
                            }
                            let _ = debounce_timer.try_update_value(|slot| {
                                if let Some(timer) = slot.take() {
                                    timer.cancel();
                                }
                                *slot = Some(Timeout::new(DIFF_EDIT_DEBOUNCE_MS, move || {
                                    let _ = submit.try_with_value(|submit| submit(content));
                                }));
                            });
                        }
                        on:change=move |event| {
                            if let Some(on_persist) = on_persist_draft {
                                on_persist.run(event_target_value(&event));
                            }
                        }
                    ></textarea>
                </Show>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{DIFF_CLOSE_BUTTON_CLASS, DIFF_EDIT_DEBOUNCE_MS, diff_view_class};

    #[test]
    fn mobile_diff_uses_mobile_dom_marker() {
        assert!(diff_view_class(true).contains("diff-view-mobile"));
        assert!(!diff_view_class(false).contains("diff-view-mobile"));
    }

    #[test]
    fn draft_projection_debounce_is_contract_value() {
        assert_eq!(DIFF_EDIT_DEBOUNCE_MS, 150);
    }

    #[test]
    fn mobile_diff_close_button_is_touch_safe() {
        assert!(DIFF_CLOSE_BUTTON_CLASS.contains("min-h-11"));
        assert!(DIFF_CLOSE_BUTTON_CLASS.contains("min-w-11"));
    }
}
