use axum::{routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use deve_core::ledger::Ledger;
use tower_http::cors::CorsLayer;

pub mod ws;

pub async fn start_server(ledger: Ledger, port: u16) -> anyhow::Result<()> {
    let state = Arc::new(ws::AppState {
        ledger: Arc::new(ledger),
    });

    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("Server running on ws://{}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
