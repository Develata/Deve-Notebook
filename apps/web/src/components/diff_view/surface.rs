//! Diff projection loading and fail-closed error surface.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 13_i18n#i18n-error-code-catalog

use std::sync::Arc;

use super::projection::ProjectionBody;
use crate::i18n::{Locale, t};
use crate::runtime::source_control_client::diff_session::DiffProjectionStatus;
use deve_core::source_control::diff_projection::DiffProjection;
use leptos::prelude::*;

#[component]
pub(super) fn ProjectionSurface(
    projection: Option<Arc<DiffProjection>>,
    status: DiffProjectionStatus,
    force_unified: bool,
    on_retry: Option<Callback<()>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    match status {
        DiffProjectionStatus::Ready => projection
            .map(|projection| view! { <ProjectionBody projection force_unified /> }.into_any())
            .unwrap_or_else(|| {
                unavailable_view(
                    t::diff::projection_unavailable(locale.get()),
                    t::diff::retry(locale.get()),
                    on_retry,
                )
            }),
        DiffProjectionStatus::Loading => status_view(t::diff::loading(locale.get())),
        DiffProjectionStatus::Debouncing { .. } => {
            status_view(t::diff::waiting_for_draft(locale.get()))
        }
        DiffProjectionStatus::Computing { .. } => status_view(t::diff::computing(locale.get())),
        DiffProjectionStatus::Unavailable(error) => {
            let message = t::server_error::message(locale.get(), error.code);
            unavailable_view(message, t::diff::retry(locale.get()), on_retry)
        }
    }
}

fn status_view(message: &'static str) -> AnyView {
    view! {
        <div class="flex h-full items-center justify-center text-sm text-[var(--diff-muted)]" data-deve-diff-status="loading">{message}</div>
    }
    .into_any()
}

fn unavailable_view(
    message: &str,
    retry_label: &'static str,
    on_retry: Option<Callback<()>>,
) -> AnyView {
    let message = message.to_string();
    view! {
        <div class="flex h-full flex-col items-center justify-center gap-3 p-6 text-center" data-deve-diff-status="unavailable">
            <p class="text-sm text-[var(--diff-muted)]">{message}</p>
            {on_retry.map(|on_retry| view! {
                <button class="rounded border border-[var(--diff-border)] px-3 py-1 text-xs" on:click=move |_| on_retry.run(())>{retry_label}</button>
            })}
        </div>
    }
    .into_any()
}
