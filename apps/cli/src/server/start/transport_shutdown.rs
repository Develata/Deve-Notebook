//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 14_commands#cli-commands
//!
//! Per-transport and process-final shutdown deadline coordination.

use std::future::{Future, IntoFuture};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
pub(crate) struct RuntimeShutdownDeadline {
    deadline: Arc<Mutex<Option<tokio::time::Instant>>>,
}

impl RuntimeShutdownDeadline {
    pub(crate) fn begin(&self, timeout: Duration) -> tokio::time::Instant {
        self.begin_at(tokio::time::Instant::now(), timeout)
    }

    fn begin_at(&self, now: tokio::time::Instant, timeout: Duration) -> tokio::time::Instant {
        let proposed = now.checked_add(timeout).unwrap_or(now);
        let mut deadline = self
            .deadline
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match *deadline {
            Some(current) if current <= proposed => current,
            _ => {
                *deadline = Some(proposed);
                proposed
            }
        }
    }
}

pub(super) fn deadline_after(timeout: Duration) -> tokio::time::Instant {
    tokio::time::Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(tokio::time::Instant::now)
}

pub(super) fn remaining_shutdown_budget(
    deadline: tokio::time::Instant,
    now: tokio::time::Instant,
) -> Duration {
    deadline.saturating_duration_since(now)
}

#[cfg(test)]
async fn serve_router_until_shutdown<F>(
    listener: tokio::net::TcpListener,
    app: axum::Router,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    serve_router_until_shutdown_with_timeout(
        listener,
        app,
        shutdown,
        super::SERVER_RUNTIME_SHUTDOWN_TIMEOUT,
    )
    .await
}

#[cfg(test)]
async fn serve_router_until_shutdown_with_timeout<F>(
    listener: tokio::net::TcpListener,
    app: axum::Router,
    shutdown: F,
    shutdown_timeout: Duration,
) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    serve_router_until_shutdown_with_deadline(listener, app, async move {
        shutdown.await;
        deadline_after(shutdown_timeout)
    })
    .await
}

pub(super) async fn serve_router_until_shutdown_with_deadline<F>(
    listener: tokio::net::TcpListener,
    app: axum::Router,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: Future<Output = tokio::time::Instant> + Send + 'static,
{
    let (shutdown_started_tx, shutdown_started_rx) = tokio::sync::oneshot::channel();
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let deadline = shutdown.await;
        let _ = shutdown_started_tx.send(deadline);
    })
    .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => {
            result?;
            Ok(())
        }
        deadline = shutdown_started_rx => {
            let deadline = deadline
                .map_err(|_| anyhow::anyhow!("server transport shutdown marker unavailable"))?;
            match tokio::time::timeout_at(deadline, &mut server).await {
                Ok(result) => {
                    result?;
                    Ok(())
                }
                Err(_) => Err(anyhow::anyhow!("server transport graceful shutdown deadline exceeded")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use tokio::sync::oneshot;

    #[test]
    fn server_runtime_remaining_shutdown_budget_never_extends_deadline() {
        let now = tokio::time::Instant::now();
        let deadline = now + Duration::from_secs(5);
        assert_eq!(
            remaining_shutdown_budget(deadline, now + Duration::from_secs(3)),
            Duration::from_secs(2)
        );
        assert_eq!(
            remaining_shutdown_budget(deadline, now + Duration::from_secs(6)),
            Duration::ZERO
        );
    }

    #[test]
    fn server_shutdown_deadline_is_shared_across_transport_and_owner_cleanup() {
        let budget = RuntimeShutdownDeadline::default();
        let now = tokio::time::Instant::now();
        let transport_deadline = budget.begin_at(now, Duration::from_secs(5));

        assert_eq!(
            budget.begin_at(now + Duration::from_secs(3), Duration::from_secs(5)),
            transport_deadline,
            "owner cleanup must not restart the transport deadline",
        );
        let shortened = budget.begin_at(now + Duration::from_secs(1), Duration::from_secs(1));
        assert_eq!(shortened, now + Duration::from_secs(2));
        assert_eq!(
            remaining_shutdown_budget(shortened, now + Duration::from_secs(3)),
            Duration::ZERO,
        );
    }

    #[test]
    fn transport_generations_receive_independent_shutdown_deadlines() {
        let now = tokio::time::Instant::now();
        let first_generation = RuntimeShutdownDeadline::default();
        assert_eq!(
            first_generation.begin_at(now, Duration::from_secs(5)),
            now + Duration::from_secs(5),
        );

        let second_generation = RuntimeShutdownDeadline::default();
        assert_eq!(
            second_generation.begin_at(now + Duration::from_secs(6), Duration::from_secs(5)),
            now + Duration::from_secs(11),
        );
    }

    #[tokio::test]
    async fn native_loopback_graceful_shutdown_stops_bound_server() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("listener");
        let addr = listener.local_addr().expect("addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(serve_router_until_shutdown(
            listener,
            Router::new().route("/health", get(|| async { "ok" })),
            async move {
                let _ = shutdown_rx.await;
            },
        ));

        tokio::net::TcpStream::connect(addr)
            .await
            .expect("server accepts connections");
        shutdown_tx.send(()).expect("signal shutdown");
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("bounded shutdown")
            .expect("join")
            .expect("server result");
    }

    #[tokio::test]
    async fn server_transport_shutdown_deadline_preempts_in_flight_handler() {
        use tokio::io::AsyncWriteExt;

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("listener");
        let addr = listener.local_addr().expect("addr");
        let entered = Arc::new(tokio::sync::Notify::new());
        let entered_handler = entered.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(serve_router_until_shutdown_with_timeout(
            listener,
            Router::new().route(
                "/slow",
                get(move || {
                    let entered = entered_handler.clone();
                    async move {
                        entered.notify_one();
                        std::future::pending::<&'static str>().await
                    }
                }),
            ),
            async move {
                let _ = shutdown_rx.await;
            },
            Duration::from_millis(20),
        ));
        let mut stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
        stream
            .write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .expect("request");
        tokio::time::timeout(Duration::from_secs(1), entered.notified())
            .await
            .expect("handler entered");
        shutdown_tx.send(()).expect("signal shutdown");
        let error = tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("bounded server task")
            .expect("join")
            .expect_err("in-flight handler must hit graceful deadline");
        assert!(
            error
                .to_string()
                .contains("server transport graceful shutdown deadline exceeded")
        );
    }
}
