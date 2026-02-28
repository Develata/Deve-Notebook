// apps/web/src/components/dashboard/mod.rs
//! # Dashboard (仪表盘)
//!
//! 当没有文档被选中时，在主内容区显示服务器运行指标。
//!
//! **Invariant**: 所有指标仅存于 RAM 信号中，不持久化到 IndexedDB。
//! 当 WebSocket 断开时，指标冻结并显示 "Waiting for server..." 提示。

mod actions_card;
mod health_card;
mod storage_card;
mod sync_card;

use crate::hooks::use_core::DashboardContext;
use leptos::prelude::*;

use self::actions_card::ActionsCard;
use self::health_card::HealthCard;
use self::storage_card::StorageCard;
use self::sync_card::SyncCard;

#[component]
pub fn Dashboard() -> impl IntoView {
    let ctx = expect_context::<DashboardContext>();

    view! {
        <div class="flex items-center justify-center h-full p-6">
            <div class="w-full max-w-md space-y-4">
                <h2 class="text-lg font-bold text-primary text-center mb-4">
                    "Server Dashboard"
                </h2>
                {move || match ctx.metrics.get() {
                    Some(m) => view! {
                        <div class="space-y-3">
                            <HealthCard metrics=m.clone() />
                            <SyncCard metrics=m.clone() />
                            <StorageCard metrics=m.clone() />
                            <ActionsCard />
                        </div>
                    }.into_any(),
                    None => view! {
                        <div class="text-center text-muted text-sm py-8">
                            "Waiting for server metrics..."
                        </div>
                        <ActionsCard />
                    }.into_any(),
                }}
            </div>
        </div>
    }
}
