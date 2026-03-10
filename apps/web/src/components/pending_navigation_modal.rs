use crate::hooks::use_core::navigation::{NavigationTarget, PendingNavigation};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn PendingNavigationModal(
    pending: ReadSignal<Option<PendingNavigation>>,
    set_pending: WriteSignal<Option<PendingNavigation>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let confirm = move |_| {
        let next = pending.get_untracked();
        set_pending.set(None);
        if let Some(next) = next {
            next.action.run(());
        }
    };
    let cancel = move |_| set_pending.set(None);

    view! {
        <Show when=move || pending.get().is_some()>
            <div class="fixed inset-0 z-[110] flex items-center justify-center bg-black/50 backdrop-blur-sm">
                <div class="w-full max-w-md rounded-xl border border-default bg-panel p-6 shadow-2xl">
                    <h2 class="text-xl font-semibold text-primary">
                        {move || t::common::pending_navigation_title(locale.get())}
                    </h2>
                    <p class="mt-3 text-sm text-secondary">
                        {move || t::common::pending_navigation_body(locale.get())}
                    </p>
                    <div class="mt-4 rounded-lg border border-amber-300 bg-amber-100 px-3 py-2 text-sm text-amber-950">
                        <span class="font-medium">
                            {move || t::common::pending_navigation_destination(locale.get())}
                        </span>
                        <span class="ml-2">
                            {move || pending.get().map(|next| target_text(locale.get(), next.target)).unwrap_or_default()}
                        </span>
                    </div>
                    <p class="mt-3 text-sm text-muted">
                        {move || t::common::pending_navigation_note(locale.get())}
                    </p>
                    <div class="mt-6 flex gap-3">
                        <button
                            class="flex-1 rounded-lg border border-default px-4 py-2 text-sm font-medium text-primary hover:bg-hover"
                            on:click=cancel
                        >
                            {move || t::common::pending_navigation_cancel(locale.get())}
                        </button>
                        <button
                            class="flex-1 rounded-lg bg-amber-600 px-4 py-2 text-sm font-medium text-white hover:bg-amber-700"
                            on:click=confirm
                        >
                            {move || t::common::pending_navigation_continue(locale.get())}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

fn target_text(locale: Locale, target: NavigationTarget) -> &'static str {
    match target {
        NavigationTarget::Doc => t::common::pending_navigation_doc(locale),
        NavigationTarget::Repo => t::common::pending_navigation_repo(locale),
        NavigationTarget::Branch => t::common::pending_navigation_branch(locale),
        NavigationTarget::Home => t::common::pending_navigation_home(locale),
    }
}
