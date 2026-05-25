//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::hooks::use_core::contexts::SystemMetricsData;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::protocol::ServerMessage;
use leptos::prelude::{Set, Update};

pub fn handle_remaining(msg: ServerMessage, signals: CoreSignals) {
    match msg {
        ServerMessage::Pong => {}
        ServerMessage::Snapshot { .. }
        | ServerMessage::History { .. }
        | ServerMessage::NewOp { .. }
        | ServerMessage::SyncPush { .. }
        | ServerMessage::SyncPushSnapshot { .. }
        | ServerMessage::KeyProvide { .. }
        | ServerMessage::KeyDenied { .. } => {}
        ServerMessage::SystemMetrics {
            cpu_usage_percent,
            memory_used_mb,
            active_connections,
            ops_processed,
            uptime_secs,
            db_size_bytes,
            doc_count,
        } => {
            signals.set_system_metrics.update(|current| {
                let sample_seq = current
                    .as_ref()
                    .map_or(1, |metrics| metrics.sample_seq.saturating_add(1));
                *current = Some(SystemMetricsData {
                    sample_seq,
                    cpu_usage_percent,
                    memory_used_mb,
                    active_connections,
                    ops_processed,
                    uptime_secs,
                    db_size_bytes,
                    doc_count,
                });
            });
            signals.set_system_metrics_live.set(true);
        }
        other => {
            leptos::logging::log!("未处理的服务端消息: {:?}", other);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::handle_remaining;
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::state::init_signals;
    use deve_core::protocol::ServerMessage;
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;

    fn metrics_message(uptime_secs: u64, active_connections: u32) -> ServerMessage {
        ServerMessage::SystemMetrics {
            cpu_usage_percent: 12.5,
            memory_used_mb: 128,
            active_connections,
            ops_processed: uptime_secs.saturating_mul(2),
            uptime_secs,
            db_size_bytes: 4096,
            doc_count: 7,
        }
    }

    #[test]
    fn dashboard_metrics_ws_refresh_increments_sample_seq() {
        let runtime = Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);

        handle_remaining(metrics_message(5, 1), signals);
        let first = signals
            .system_metrics
            .get_untracked()
            .expect("first metrics");
        assert_eq!(first.sample_seq, 1);
        assert_eq!(first.uptime_secs, 5);
        assert_eq!(first.active_connections, 1);
        assert!(signals.system_metrics_live.get_untracked());

        handle_remaining(metrics_message(10, 2), signals);
        let second = signals
            .system_metrics
            .get_untracked()
            .expect("second metrics");
        assert_eq!(second.sample_seq, 2);
        assert_eq!(second.uptime_secs, 10);
        assert_eq!(second.active_connections, 2);
    }
}
