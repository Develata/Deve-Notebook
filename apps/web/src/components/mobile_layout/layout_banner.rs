//! plan_ref:
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!
use crate::components::workspace_ingestion_blocker::WorkspaceIngestionBlockerBanner;
use crate::runtime::session_client::SessionClient;
use leptos::prelude::*;

#[component]
pub fn MobileSyncBanner() -> impl IntoView {
    let session = expect_context::<SessionClient>();

    view! {
        <WorkspaceIngestionBlockerBanner />
        <Show when=move || session.sync_banner.get().is_some()>
            <div class="mx-3 mt-2 rounded-lg border border-amber-300 bg-amber-100 px-3 py-2 text-[11px] font-medium text-amber-950">
                {move || session.sync_banner.get().unwrap_or_default()}
            </div>
        </Show>
    }
}
