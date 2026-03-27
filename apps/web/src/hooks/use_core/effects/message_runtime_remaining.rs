use crate::hooks::use_core::contexts::SystemMetricsData;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::protocol::ServerMessage;
use leptos::prelude::Set;

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
            signals.set_system_metrics.set(Some(SystemMetricsData {
                cpu_usage_percent,
                memory_used_mb,
                active_connections,
                ops_processed,
                uptime_secs,
                db_size_bytes,
                doc_count,
            }));
        }
        other => {
            leptos::logging::log!("未处理的服务端消息: {:?}", other);
        }
    }
}
