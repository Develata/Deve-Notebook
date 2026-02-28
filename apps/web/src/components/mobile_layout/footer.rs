// apps/web/src/components/mobile_layout/footer.rs
//! # Mobile Footer

use crate::api::ConnectionStatus;
use crate::components::branch_switcher::BranchSwitcher;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use web_sys::UiEvent;

#[component]
pub fn MobileFooter(core: CoreState) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let max_ver = core.doc_version;
    let curr_ver = core.playback_version;
    let set_ver = core.set_playback_version;
    let status = core.ws.status;
    let stats = core.stats;
    let load_state = core.load_state;
    let load_progress = core.load_progress;
    let load_eta_ms = core.load_eta_ms;
    let (is_narrow, set_is_narrow) = signal(false);
    let (expanded, set_expanded) = signal(false);

    let update_narrow = move || {
        let width = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(390.0);
        set_is_narrow.set(width <= 360.0);
    };
    update_narrow();
    window_event_listener(leptos::ev::resize, move |_ev: UiEvent| update_narrow());

    let status_view = move || {
        let (color, text) = match status.get() {
            ConnectionStatus::Connected => ("bg-green-500", t::bottom_bar::ready(locale.get())),
            ConnectionStatus::Connecting => ("bg-yellow-500", t::bottom_bar::syncing(locale.get())),
            ConnectionStatus::Disconnected => ("bg-red-500", t::bottom_bar::offline(locale.get())),
        };
        view! {
            <div class="flex items-center gap-1.5">
                <div class={format!("w-2 h-2 rounded-full {}", color)}></div>
                <span class="text-[11px] text-secondary font-medium">{text}</span>
            </div>
        }
    };

    let load_status = move || {
        if load_state.get() == "ready" {
            return view! {}.into_any();
        }
        let (done, total) = load_progress.get();
        let eta_ms = load_eta_ms.get();
        let text = if total > 0 {
            if eta_ms > 0 {
                if is_narrow.get() {
                    format!("L {}/{}", done, total)
                } else {
                    format!(
                        "{} {}/{} (~{}ms)",
                        t::bottom_bar::loading(locale.get()),
                        done,
                        total,
                        eta_ms
                    )
                }
            } else {
                format!("L {}/{}", done, total)
            }
        } else {
            t::bottom_bar::loading(locale.get()).to_string()
        };
        view! { <div class="text-[10px] text-muted font-mono">{text}</div> }.into_any()
    };

    let on_to_start = Callback::new(move |_| set_ver.set(0));
    let on_prev = Callback::new(move |_| {
        let next = curr_ver.get_untracked().saturating_sub(1);
        set_ver.set(next);
    });
    let on_next = Callback::new(move |_| {
        let cur = curr_ver.get_untracked();
        let max = max_ver.get_untracked();
        set_ver.set((cur + 1).min(max));
    });
    let on_to_end = Callback::new(move |_| set_ver.set(max_ver.get_untracked()));

    view! {
        <Show when=move || expanded.get()>
            <div class="fixed inset-0 z-40" on:click=move |_| set_expanded.set(false)></div>
        </Show>

        <footer
            class="relative z-50 bg-panel border-t border-default px-2 py-1.5 flex flex-col gap-1.5"
            style="padding-bottom: env(safe-area-inset-bottom);"
        >
            <div class="flex items-center gap-1.5">
                <div class="flex-1 min-w-0 flex items-center gap-1 whitespace-nowrap overflow-hidden">
                    <div class="shrink-0">
                        <BranchSwitcher compact=true />
                    </div>
                    <div class="shrink-0 px-1.5 h-6 rounded-md bg-sidebar border border-default flex items-center">
                        {status_view}
                    </div>
                    <div class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                        <span>{move || if is_narrow.get() { "W".to_string() } else { t::bottom_bar::words(locale.get()).to_string() }}</span>
                        <span class="font-mono text-primary">{move || stats.get().words}</span>
                    </div>
                    <div class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                        <span>{move || if is_narrow.get() { "L".to_string() } else { t::bottom_bar::lines(locale.get()).to_string() }}</span>
                        <span class="font-mono text-primary">{move || stats.get().lines}</span>
                    </div>
                    <div class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                        <span>{move || if is_narrow.get() { "C".to_string() } else { t::bottom_bar::col(locale.get()).to_string() }}</span>
                        <span class="font-mono text-primary">{move || stats.get().chars}</span>
                    </div>
                </div>

                <button
                    class="h-11 min-w-11 p-1.5 rounded-md active:bg-hover flex items-center justify-center"
                    title=move || t::bottom_bar::toggle_status_details(locale.get())
                    aria-label=move || t::bottom_bar::toggle_status_details(locale.get())
                    on:click=move |_| set_expanded.update(|v| *v = !*v)
                >
                    {move || if expanded.get() {
                        view! {
                            <span class="h-8 w-8 rounded-md border border-default bg-panel text-secondary flex items-center justify-center">
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M5 8l5 5 5-5" />
                                </svg>
                            </span>
                        }
                        .into_any()
                    } else {
                        view! {
                            <span class="h-8 w-8 rounded-md border border-default bg-panel text-secondary flex items-center justify-center">
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M5 12l5-5 5 5" />
                                </svg>
                            </span>
                        }
                        .into_any()
                    }}
                </button>
            </div>

            <Show when=move || expanded.get()>
                <div class="flex items-center gap-2 overflow-x-auto pb-0.5 scrollbar-none">
                    <Show when=move || load_state.get() != "ready">
                        <div class="shrink-0 px-2 h-7 rounded-md bg-sidebar border border-default flex items-center">
                            {load_status}
                        </div>
                    </Show>
                    <div class="shrink-0 text-[10px] text-muted font-mono px-1">
                        {move || format!("v{}/{}", curr_ver.get(), max_ver.get())}
                    </div>
                </div>

                <Show
                    when=move || is_narrow.get()
                    fallback=move || {
                        view! {
                            <div class="flex items-center gap-1.5 px-1 py-1 rounded-lg bg-sidebar border border-default">
                                <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::first(locale.get()) on:click=move |_| on_to_start.run(())>
                                    "«"
                                </button>
                                <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::prev(locale.get()) on:click=move |_| on_prev.run(())>
                                    "‹"
                                </button>
                                <input
                                    name="mobile-time-travel"
                                    type="range"
                                    min="0"
                                    max=move || max_ver.get().to_string()
                                    value=move || curr_ver.get().to_string()
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
                                        set_ver.set(val);
                                    }
                                    class="flex-1 min-w-16 h-1 bg-active rounded-lg appearance-none cursor-pointer accent-accent"
                                    title=move || t::bottom_bar::time_travel(locale.get())
                                />
                                <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::next(locale.get()) on:click=move |_| on_next.run(())>
                                    "›"
                                </button>
                                <button class="h-7 min-w-7 px-1 rounded border border-default bg-panel text-secondary active:bg-hover" title=move || t::bottom_bar::latest(locale.get()) on:click=move |_| on_to_end.run(())>
                                    "»"
                                </button>
                            </div>
                        }
                    }
                >
                    <div class="rounded-lg bg-sidebar border border-default px-1 py-1 flex flex-col gap-1">
                        <div class="flex items-center justify-between gap-1">
                            <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::first(locale.get()) on:click=move |_| on_to_start.run(())>
                                "«"
                            </button>
                            <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::prev(locale.get()) on:click=move |_| on_prev.run(())>
                                "‹"
                            </button>
                            <span class="text-[9px] text-muted font-mono px-0.5 min-w-12 text-center">
                                {move || format!("v{}/{}", curr_ver.get(), max_ver.get())}
                            </span>
                            <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::next(locale.get()) on:click=move |_| on_next.run(())>
                                "›"
                            </button>
                            <button class="h-6 min-w-6 px-1 rounded border border-default bg-panel text-secondary active:bg-hover text-[10px]" title=move || t::bottom_bar::latest(locale.get()) on:click=move |_| on_to_end.run(())>
                                "»"
                            </button>
                        </div>
                        <input
                            name="mobile-time-travel-narrow"
                            type="range"
                            min="0"
                            max=move || max_ver.get().to_string()
                            value=move || curr_ver.get().to_string()
                            on:input=move |ev| {
                                let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
                                set_ver.set(val);
                            }
                            class="w-full h-1 bg-active rounded-lg appearance-none cursor-pointer accent-accent"
                            title=move || t::bottom_bar::time_travel(locale.get())
                        />
                    </div>
                </Show>
            </Show>
        </footer>
    }
}
