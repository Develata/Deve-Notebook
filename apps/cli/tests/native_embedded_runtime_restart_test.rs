//! plan_ref:
//!   - 07_network#native-full-peer-runtime
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract

use std::time::Duration;

use deve_cli::native_runtime::{
    NativeEmbeddedServerRuntime, NativeLocalBackendOptions, bind_native_loopback_listener,
    bind_native_loopback_listener_exact,
};
use deve_cli::server::NativeLoopbackAuthMaterial;
use deve_core::native_adapter::native_tauri_allowed_origins;
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_TAURI_HTTP_LOCALHOST_ORIGIN,
};
use deve_core::security::auth::password;
use futures::StreamExt;
use tempfile::tempdir;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{COOKIE, ORIGIN};

fn generation_options(
    data_root: &std::path::Path,
    port: u16,
    generation: u64,
) -> NativeLocalBackendOptions {
    let password_hash =
        password::hash_password(&format!("password-{generation}")).expect("hash test password");
    let session_secret = format!("{generation:064x}");
    let auth_secret = format!("{:064x}", generation.saturating_add(1));
    let auth = NativeLoopbackAuthMaterial::new(
        session_secret,
        auth_secret,
        "native",
        password_hash,
        native_tauri_allowed_origins(),
    );
    NativeLocalBackendOptions::new(data_root, port).with_auth_material(auth)
}

async fn wait_for_listener(port: u16) {
    tokio::time::timeout(Duration::from_secs(5), async move {
        loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("transport listener became reachable");
}

async fn connect_generation_websocket(
    port: u16,
    generation: u64,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let secret = format!("{generation:064x}");
    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/api/auth/native-session"))
        .header(NATIVE_SESSION_BOOTSTRAP_HEADER, secret)
        .send()
        .await
        .expect("issue native session cookie");
    let cookie = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("session cookie");
    let mut request = format!("ws://127.0.0.1:{port}/ws")
        .into_client_request()
        .expect("WS request");
    request
        .headers_mut()
        .insert(COOKIE, cookie.parse().expect("cookie header"));
    request.headers_mut().insert(
        ORIGIN,
        NATIVE_TAURI_HTTP_LOCALHOST_ORIGIN
            .parse()
            .expect("origin header"),
    );
    tokio_tungstenite::connect_async(request)
        .await
        .expect("authenticated WS")
        .0
}

async fn assert_websocket_closed(
    mut socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    tokio::time::timeout(Duration::from_secs(5), async move {
        while let Some(message) = socket.next().await {
            match message {
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => return,
                Ok(_) => {}
            }
        }
    })
    .await
    .expect("retired transport closes upgraded WS session");
}

#[tokio::test]
async fn embedded_runtime_reuses_authority_across_transport_generations() {
    let data_root = tempdir().expect("temp data root");
    let first_listener = bind_native_loopback_listener(None).expect("first listener");
    let first_port = first_listener.port();
    let first_options = generation_options(data_root.path(), first_port, 1);
    let runtime = NativeEmbeddedServerRuntime::initialize(&first_options)
        .await
        .expect("initialize one authority runtime");
    let transport = runtime.transport();

    let (first_shutdown, first_shutdown_rx) = oneshot::channel();
    let first_task = tokio::spawn({
        let transport = transport.clone();
        async move {
            transport
                .serve_with_listener_until_shutdown(first_options, first_listener, async move {
                    let _ = first_shutdown_rx.await;
                })
                .await
        }
    });
    wait_for_listener(first_port).await;
    let first_socket = connect_generation_websocket(first_port, 1).await;
    let _ = first_shutdown.send(());
    assert_websocket_closed(first_socket).await;
    first_task
        .await
        .expect("join first transport")
        .expect("stop first transport");

    let _old_port_guard =
        bind_native_loopback_listener_exact(first_port).expect("reserve old port");
    let second_listener = bind_native_loopback_listener(None).expect("second listener");
    let second_port = second_listener.port();
    assert_ne!(second_port, first_port);
    let second_options = generation_options(data_root.path(), second_port, 2);
    let (second_shutdown, second_shutdown_rx) = oneshot::channel();
    let second_task = tokio::spawn(async move {
        transport
            .serve_with_listener_until_shutdown(second_options, second_listener, async move {
                let _ = second_shutdown_rx.await;
            })
            .await
    });
    wait_for_listener(second_port).await;
    let _ = second_shutdown.send(());
    second_task
        .await
        .expect("join second transport")
        .expect("stop second transport");

    runtime
        .shutdown(Duration::from_secs(5))
        .await
        .expect("shutdown owned authority runtime tasks");
}
