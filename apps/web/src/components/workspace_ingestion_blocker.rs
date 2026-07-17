//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 13_i18n#i18n-keys-reference
//!
//! Thin rendering projection for a backend-issued workspace-ingestion blocker.

use crate::i18n::{Locale, t};
use crate::runtime::{scope_client::ScopeClient, session_client::SessionClient};
use leptos::prelude::*;

#[component]
pub fn WorkspaceIngestionBlockerBanner() -> impl IntoView {
    let session = expect_context::<SessionClient>();
    let scope = expect_context::<ScopeClient>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let blocked = Signal::derive(move || {
        let repo_id = scope.current_repo_id.get();
        session.ws.workspace_ingestion_blocked_for(
            repo_id.as_deref(),
            Some(scope.current_scope_nonce.get()),
        )
    });

    view! {
        <Show when=move || blocked.get()>
            <div
                class="mx-3 mt-2 rounded-lg border border-amber-400 bg-amber-100 px-3 py-2 text-xs font-medium text-amber-950 sm:mx-4"
                data-deve-workspace-ingestion-blocker="true"
            >
                {move || t::workspace_ingestion::blocker(locale.get())}
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_ingestion_blocker_uses_only_fixed_localized_copy() {
        for locale in [Locale::En, Locale::Zh] {
            let text = t::workspace_ingestion::blocker(locale);
            assert!(text.contains(t::workspace_ingestion::unavailable(locale)));
            assert!(text.contains(t::workspace_ingestion::restart_service(locale)));
            assert!(!text.contains("CANARY_PRIVATE_BACKEND_DETAIL"));
        }
    }
}
