//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 14_commands#cli-commands
//!
//! Process-owned termination signal adapter for standalone `deve serve`.

#[cfg(any(unix, test))]
use std::future::Future;

pub(crate) async fn production_shutdown_signal() {
    wait_for_platform_shutdown().await;
}

#[cfg(unix)]
async fn wait_for_platform_shutdown() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = match signal(SignalKind::terminate()) {
        Ok(signal) => signal,
        Err(_) => {
            signal_listener_failed();
            return;
        }
    };
    let mut interrupt = match signal(SignalKind::interrupt()) {
        Ok(signal) => signal,
        Err(_) => {
            signal_listener_failed();
            return;
        }
    };
    if !first_shutdown_signal(
        async move { terminate.recv().await.is_some() },
        async move { interrupt.recv().await.is_some() },
    )
    .await
    {
        signal_listener_failed();
    }
}

#[cfg(not(unix))]
async fn wait_for_platform_shutdown() {
    if tokio::signal::ctrl_c().await.is_err() {
        signal_listener_failed();
    }
}

#[cfg(any(unix, test))]
async fn first_shutdown_signal<Terminate, Interrupt>(
    terminate: Terminate,
    interrupt: Interrupt,
) -> bool
where
    Terminate: Future<Output = bool>,
    Interrupt: Future<Output = bool>,
{
    tokio::pin!(terminate);
    tokio::pin!(interrupt);
    tokio::select! {
        received = &mut terminate => received,
        received = &mut interrupt => received,
    }
}

fn signal_listener_failed() {
    tracing::error!(
        category = "server_shutdown_signal_unavailable",
        "production shutdown signal listener failed; retiring the server runtime"
    );
}

#[cfg(test)]
mod tests {
    use super::first_shutdown_signal;

    #[tokio::test]
    async fn production_shutdown_signal_selects_either_termination_source() {
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_secs(1),
                first_shutdown_signal(std::future::pending(), std::future::ready(true)),
            )
            .await
            .expect("interrupt completes the selector")
        );
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_secs(1),
                first_shutdown_signal(std::future::ready(true), std::future::pending()),
            )
            .await
            .expect("terminate completes the selector")
        );
        assert!(
            !first_shutdown_signal(std::future::ready(false), std::future::pending()).await,
            "closed signal listener must be distinguished from a real signal",
        );
    }
}
