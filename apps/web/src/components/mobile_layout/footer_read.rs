//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 18_release#runtime-observability
//!
//! Local read helpers for mobile footer render closures.

use leptos::prelude::*;

pub(super) fn read_footer_signal<T>(signal: ReadSignal<T>, fallback: T) -> T
where
    T: Clone + Send + Sync + 'static,
{
    signal.try_get().unwrap_or(fallback)
}

pub(super) fn read_footer_value<T>(signal: Signal<T>, fallback: T) -> T
where
    T: Clone + Send + Sync + 'static,
{
    signal.try_get().unwrap_or(fallback)
}
