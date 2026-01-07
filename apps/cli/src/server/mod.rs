use axum::{routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use deve_core::ledger::Ledger;
use tower_http::cors::{Any, CorsLayer};
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use std::net::SocketAddr;

pub mod ws;

pub struct AppState {
    pub ledger: Arc<Ledger>,
    pub tx: broadcast::Sender<ServerMessage>,
}

pub async fn start_server(ledger: Ledger, port: u16) -> anyhow::Result<()> {
    // Create broadcast channel for WS server
    let (tx, _rx) = broadcast::channel(100);
    
    let app_state = Arc::new(AppState { 
        ledger: Arc::new(ledger),
        tx,
    });

    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .with_state(app_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server running on ws://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
