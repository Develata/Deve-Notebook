// apps\web\src\api
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 08_auth#unauthorized-handling
//!   - 08_auth#unauthorized-disconnected-ui
//!
//! # WebSocket 连接管理器
//!
//! ## 职责
//! 1. 建立 WebSocket 连接
//! 2. 指数退避重连策略
//! 3. 读取服务器消息并更新信号
//! 4. 将连接会话委托给 `connection_session`

mod lifecycle;
mod session;

use futures::channel::mpsc::UnboundedReceiver;
use futures::{FutureExt, StreamExt};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::VecDeque;

pub(super) use self::lifecycle::ConnectionLifecycle;
use self::session::{ConnectedSessionOutcome, ConnectedSessionSignals};
use super::ConnectionStatus;
use super::auth_probe::{AuthProbe, probe_auth_status_with_http_base};
use super::backoff::BackoffStrategy;
use super::connection_role::{
    NodeRoleProbeContext, WatcherHealthSnapshot, fetch_node_role, fetch_node_role_for_http_base,
};
use super::connection_urls::{build_same_origin_ws_url, build_ws_urls_for_native_state};
use super::native_bootstrap::read_native_bootstrap;
use super::native_http::preferred_http_base;
use super::output::prepare_queue_for_new_connection;
use super::socket::BrowserSocket;
use super::write_gate::{WriterReadyResetSignals, set_status_and_revoke_writer_ready};

/// 本地开发后端默认端口；仅在 debug 构建中作为兜底候选。
pub(super) const DEV_WS_PORT: u16 = 3001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionControl {
    RebindNativeEndpoint,
    ReconnectForResync { connection_epoch: u64 },
}

#[derive(Clone)]
pub(super) struct ConnectionManagerSignals {
    pub lifecycle: ConnectionLifecycle,
    pub current_status: ReadSignal<ConnectionStatus>,
    pub set_status: WriteSignal<ConnectionStatus>,
    pub set_msg_seq: WriteSignal<u64>,
    pub set_msg_queue: WriteSignal<VecDeque<(u64, u64, deve_core::protocol::ServerMessage)>>,
    pub current_connection_epoch: ReadSignal<u64>,
    pub set_connection_epoch: WriteSignal<u64>,
    pub set_endpoint: WriteSignal<String>,
    pub set_node_role: WriteSignal<String>,
    pub set_source_control_authority: WriteSignal<String>,
    pub set_host_file_copy_absolute_path: WriteSignal<bool>,
    pub set_host_file_reveal_in_system_explorer: WriteSignal<bool>,
    pub set_watcher_health: WriteSignal<WatcherHealthSnapshot>,
    pub set_node_role_probe_failed: WriteSignal<bool>,
    pub writer_ready_reset: WriterReadyResetSignals,
}

/// 启动连接管理器任务
pub fn spawn_connection_manager(
    mut rx: UnboundedReceiver<deve_core::protocol::ClientMessage>,
    mut control_rx: UnboundedReceiver<ConnectionControl>,
    signals: ConnectionManagerSignals,
) {
    spawn_local(async move {
        let mut backoff = BackoffStrategy::new();
        let mut queue = VecDeque::new();
        let mut connection_epoch = 0u64;

        'bootstrap: loop {
            if !signals.lifecycle.is_active() {
                return;
            }
            let native_bootstrap = read_native_bootstrap();
            let auth_http_base = preferred_http_base();
            if let Some(blocked_status) = native_bootstrap.blocked_status() {
                leptos::logging::error!(
                    "Native bootstrap is present but not ready; refusing fallback"
                );
                if !try_set_connection_status(&signals, blocked_status) {
                    return;
                }
                if wait_for_rebind(&mut control_rx).await {
                    backoff.reset();
                    continue;
                }
                return;
            }

            let urls = build_ws_urls_for_native_state(&native_bootstrap);
            let mut url_idx = 0usize;
            loop {
                if !signals.lifecycle.is_active() {
                    return;
                }
                let url = urls
                    .get(url_idx)
                    .cloned()
                    .unwrap_or_else(build_same_origin_ws_url);
                connection_epoch = connection_epoch.saturating_add(1);
                if !signals
                    .lifecycle
                    .try_set(signals.set_connection_epoch, connection_epoch)
                {
                    return;
                }
                if !try_set_connection_status(&signals, ConnectionStatus::Connecting) {
                    return;
                }
                leptos::logging::log!("WS: Connecting to {}...", url);

                match BrowserSocket::connect(&url) {
                    Ok((socket, events)) => {
                        if !signals.lifecycle.try_set(signals.set_endpoint, url.clone())
                            || !signals
                                .lifecycle
                                .try_set(signals.set_node_role, String::new())
                            || !signals.lifecycle.try_set(
                                signals.set_source_control_authority,
                                "unknown".to_string(),
                            )
                            || !signals
                                .lifecycle
                                .try_set(signals.set_host_file_copy_absolute_path, false)
                            || !signals
                                .lifecycle
                                .try_set(signals.set_host_file_reveal_in_system_explorer, false)
                            || !signals.lifecycle.try_set(
                                signals.set_watcher_health,
                                WatcherHealthSnapshot::default(),
                            )
                            || !signals
                                .lifecycle
                                .try_set(signals.set_node_role_probe_failed, false)
                        {
                            return;
                        }
                        if let Some(http_base) = auth_http_base.clone() {
                            spawn_local(fetch_node_role_for_http_base(
                                signals.lifecycle.clone(),
                                http_base,
                                NodeRoleProbeContext {
                                    set_node_role: signals.set_node_role,
                                    set_source_control_authority: signals
                                        .set_source_control_authority,
                                    set_host_file_copy_absolute_path: signals
                                        .set_host_file_copy_absolute_path,
                                    set_host_file_reveal_in_system_explorer: signals
                                        .set_host_file_reveal_in_system_explorer,
                                    set_watcher_health: signals.set_watcher_health,
                                    set_node_role_probe_failed: signals.set_node_role_probe_failed,
                                    current_connection_epoch: signals.current_connection_epoch,
                                    probe_connection_epoch: connection_epoch,
                                },
                            ));
                        } else {
                            spawn_local(fetch_node_role(
                                signals.lifecycle.clone(),
                                url.clone(),
                                NodeRoleProbeContext {
                                    set_node_role: signals.set_node_role,
                                    set_source_control_authority: signals
                                        .set_source_control_authority,
                                    set_host_file_copy_absolute_path: signals
                                        .set_host_file_copy_absolute_path,
                                    set_host_file_reveal_in_system_explorer: signals
                                        .set_host_file_reveal_in_system_explorer,
                                    set_watcher_health: signals.set_watcher_health,
                                    set_node_role_probe_failed: signals.set_node_role_probe_failed,
                                    current_connection_epoch: signals.current_connection_epoch,
                                    probe_connection_epoch: connection_epoch,
                                },
                            ));
                        }
                        backoff.reset();
                        prepare_queue_for_new_connection(&mut queue);
                        let outcome = session::run_connected_session(
                            socket,
                            events,
                            &mut rx,
                            &mut control_rx,
                            &mut queue,
                            ConnectedSessionSignals {
                                lifecycle: signals.lifecycle.clone(),
                                set_status: signals.set_status,
                                set_msg_seq: signals.set_msg_seq,
                                set_msg_queue: signals.set_msg_queue,
                                set_node_role: signals.set_node_role,
                                set_source_control_authority: signals.set_source_control_authority,
                                set_host_file_copy_absolute_path: signals
                                    .set_host_file_copy_absolute_path,
                                set_host_file_reveal_in_system_explorer: signals
                                    .set_host_file_reveal_in_system_explorer,
                                set_watcher_health: signals.set_watcher_health,
                                set_node_role_probe_failed: signals.set_node_role_probe_failed,
                                writer_ready_reset: signals.writer_ready_reset,
                                connection_epoch,
                            },
                        )
                        .await;

                        if outcome == ConnectedSessionOutcome::RebindRequested {
                            backoff.reset();
                            continue 'bootstrap;
                        }
                        if outcome == ConnectedSessionOutcome::ResyncRequested {
                            backoff.reset();
                            if !try_set_connection_status(&signals, ConnectionStatus::Disconnected)
                            {
                                return;
                            }
                            url_idx = 0;
                            continue;
                        }

                        if !signals.lifecycle.is_active() {
                            return;
                        }
                        leptos::logging::log!("WS: Connection Lost");

                        if native_rebind_pending(&signals) {
                            if wait_for_rebind(&mut control_rx).await {
                                backoff.reset();
                                continue 'bootstrap;
                            }
                            return;
                        }
                        if matches!(
                            probe_auth_status_with_http_base(auth_http_base.as_deref()).await,
                            AuthProbe::Invalid
                        ) {
                            let _ =
                                try_set_connection_status(&signals, ConnectionStatus::Unauthorized);
                            return;
                        }
                    }
                    Err(e) => {
                        leptos::logging::error!("WS Open Error: {:?}", e);
                        if !signals.lifecycle.is_active() {
                            return;
                        }
                        if native_rebind_pending(&signals) {
                            if wait_for_rebind(&mut control_rx).await {
                                backoff.reset();
                                continue 'bootstrap;
                            }
                            return;
                        }
                        if matches!(
                            probe_auth_status_with_http_base(auth_http_base.as_deref()).await,
                            AuthProbe::Invalid
                        ) {
                            let _ =
                                try_set_connection_status(&signals, ConnectionStatus::Unauthorized);
                            return;
                        }
                        if url_idx + 1 < urls.len() {
                            url_idx += 1;
                            continue;
                        }
                    }
                }

                if !signals.lifecycle.is_active() {
                    return;
                }
                if native_rebind_pending(&signals) {
                    if wait_for_rebind(&mut control_rx).await {
                        backoff.reset();
                        continue 'bootstrap;
                    }
                    return;
                }
                if matches!(
                    probe_auth_status_with_http_base(auth_http_base.as_deref()).await,
                    AuthProbe::Invalid
                ) {
                    let _ = try_set_connection_status(&signals, ConnectionStatus::Unauthorized);
                    return;
                }
                if !try_set_connection_status(&signals, ConnectionStatus::Disconnected) {
                    return;
                }
                match wait_for_backoff_or_rebind(&mut backoff, &mut control_rx).await {
                    BackoffWait::Elapsed => url_idx = 0,
                    BackoffWait::Rebind => {
                        backoff.reset();
                        continue 'bootstrap;
                    }
                    BackoffWait::Closed => return,
                }
            }
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackoffWait {
    Elapsed,
    Rebind,
    Closed,
}

async fn wait_for_rebind(control_rx: &mut UnboundedReceiver<ConnectionControl>) -> bool {
    loop {
        match control_rx.next().await {
            Some(ConnectionControl::RebindNativeEndpoint) => return true,
            Some(ConnectionControl::ReconnectForResync { .. }) => continue,
            None => return false,
        }
    }
}

async fn wait_for_backoff_or_rebind(
    backoff: &mut BackoffStrategy,
    control_rx: &mut UnboundedReceiver<ConnectionControl>,
) -> BackoffWait {
    let delay = backoff.take_delay_ms();
    leptos::logging::log!("WS: Reconnecting in {}ms...", delay);
    let timer = TimeoutFuture::new(delay).fuse();
    let control = control_rx.next().fuse();
    futures::pin_mut!(timer, control);
    futures::select! {
        _ = timer => BackoffWait::Elapsed,
        command = control => match command {
            Some(ConnectionControl::RebindNativeEndpoint) => BackoffWait::Rebind,
            Some(ConnectionControl::ReconnectForResync { .. }) => BackoffWait::Elapsed,
            None => BackoffWait::Closed,
        },
    }
}

fn native_rebind_pending(signals: &ConnectionManagerSignals) -> bool {
    signals.current_status.get_untracked() == ConnectionStatus::NativeReprobeRequired
}

fn try_set_connection_status(signals: &ConnectionManagerSignals, status: ConnectionStatus) -> bool {
    if !signals.lifecycle.is_active() {
        return false;
    }
    if status != ConnectionStatus::Connected && !reset_node_role_runtime(signals) {
        return false;
    }
    set_status_and_revoke_writer_ready(signals.set_status, signals.writer_ready_reset, status)
}

fn reset_node_role_runtime(signals: &ConnectionManagerSignals) -> bool {
    signals
        .lifecycle
        .try_set(signals.set_node_role, String::new())
        && signals
            .lifecycle
            .try_set(signals.set_source_control_authority, "unknown".to_string())
        && signals
            .lifecycle
            .try_set(signals.set_host_file_copy_absolute_path, false)
        && signals
            .lifecycle
            .try_set(signals.set_host_file_reveal_in_system_explorer, false)
        && signals
            .lifecycle
            .try_set(signals.set_watcher_health, WatcherHealthSnapshot::default())
        && signals
            .lifecycle
            .try_set(signals.set_node_role_probe_failed, false)
}

#[cfg(test)]
mod tests;
