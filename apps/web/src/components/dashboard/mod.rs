// apps/web/src/components/dashboard/mod.rs
//! plan_ref:
//!   - 18_release#runtime-observability
//!
//! # Dashboard (仪表盘)
//!
//! 当没有文档被选中时，在主内容区显示服务器运行指标。
//!
//! **Invariant**: 所有指标仅存于 RAM 信号中，不持久化到 IndexedDB。
//! 当 WebSocket 断开时，指标冻结并显示 disconnected snapshot 提示。

mod actions_card;
mod health_card;
mod runtime_card;
mod storage_card;
mod sync_card;

use crate::api::ConnectionStatus;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::DashboardContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use self::actions_card::ActionsCard;
use self::health_card::HealthCard;
use self::runtime_card::RuntimeCard;
use self::storage_card::StorageCard;
use self::sync_card::SyncCard;

#[component]
pub fn Dashboard() -> impl IntoView {
    let ctx = expect_context::<DashboardContext>();
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let ws_status = core.ws.status;
    let runtime_summary = core.ws.node_role;
    let repo_name = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| t::dashboard::no_repo_selected(locale.get()).to_string())
    });
    let doc_count = Signal::derive(move || core.docs.get().len());
    let metrics_state = Signal::derive(move || {
        dashboard_metrics_state(
            ctx.metrics.get().is_some(),
            ctx.metrics_live.get(),
            ws_status.get(),
        )
    });
    let metrics_status = Signal::derive(move || {
        let locale = locale.get();
        match metrics_state.get() {
            DashboardMetricsState::Waiting => t::dashboard::waiting_metrics(locale),
            DashboardMetricsState::Live => t::dashboard::metrics_live(locale),
            DashboardMetricsState::FrozenDisconnected => t::dashboard::metrics_disconnected(locale),
        }
    });

    view! {
        <div
            class="w-full h-full min-h-0 overflow-auto p-6"
            data-deve-dashboard="server"
            data-deve-dashboard-metrics-state=move || metrics_state.get().as_attr()
        >
            <div class="mx-auto flex min-h-full w-full max-w-xl items-center justify-center">
                <div class="w-full space-y-4">
                    <div class="rounded-2xl border border-default bg-app px-6 py-6 shadow-sm">
                        <div class="text-center">
                            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-muted">
                                {move || t::dashboard::title(locale.get())}
                            </div>
                            <h2 class="mt-2 text-xl font-bold text-primary">
                                {move || repo_name.get()}
                            </h2>
                            <p class="mt-2 text-sm text-secondary">
                                {move || t::dashboard::docs_in_current_repo(locale.get(), doc_count.get())}
                            </p>
                            <p
                                class="mt-3 text-sm text-muted"
                                data-deve-dashboard-metrics-copy=move || metrics_state.get().as_attr()
                            >
                                {move || metrics_status.get()}
                            </p>
                        </div>
                    </div>
                    {move || match ctx.metrics.get() {
                        Some(m) => view! {
                            <div class="space-y-3">
                                <RuntimeCard runtime_summary=runtime_summary />
                                <HealthCard metrics=m.clone() />
                                <SyncCard metrics=m.clone() />
                                <StorageCard metrics=m.clone() />
                                <ActionsCard />
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="space-y-3">
                                <RuntimeCard runtime_summary=runtime_summary />
                                <ActionsCard />
                            </div>
                        }.into_any(),
                    }}
                </div>
            </div>
        </div>
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DashboardMetricsState {
    Waiting,
    Live,
    FrozenDisconnected,
}

impl DashboardMetricsState {
    fn as_attr(self) -> &'static str {
        match self {
            DashboardMetricsState::Waiting => "waiting",
            DashboardMetricsState::Live => "live",
            DashboardMetricsState::FrozenDisconnected => "frozen-disconnected",
        }
    }
}

fn dashboard_metrics_state(
    metrics_available: bool,
    metrics_live: bool,
    connection_status: ConnectionStatus,
) -> DashboardMetricsState {
    if !metrics_available {
        DashboardMetricsState::Waiting
    } else if connection_status == ConnectionStatus::Connected && metrics_live {
        DashboardMetricsState::Live
    } else {
        DashboardMetricsState::FrozenDisconnected
    }
}

#[cfg(test)]
mod tests {
    use super::{DashboardMetricsState, dashboard_metrics_state};
    use crate::api::ConnectionStatus;

    #[test]
    fn dashboard_metrics_state_tracks_ws_refresh_and_disconnect_freeze() {
        assert_eq!(
            dashboard_metrics_state(false, false, ConnectionStatus::Connected),
            DashboardMetricsState::Waiting
        );
        assert_eq!(
            dashboard_metrics_state(true, true, ConnectionStatus::Connected),
            DashboardMetricsState::Live
        );
        assert_eq!(
            dashboard_metrics_state(true, false, ConnectionStatus::Connected),
            DashboardMetricsState::FrozenDisconnected
        );
        assert_eq!(
            dashboard_metrics_state(true, true, ConnectionStatus::Disconnected),
            DashboardMetricsState::FrozenDisconnected
        );
        assert_eq!(
            dashboard_metrics_state(true, true, ConnectionStatus::Connecting),
            DashboardMetricsState::FrozenDisconnected
        );
    }
}
