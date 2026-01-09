use axum::{routing::get, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use deve_core::ledger::Ledger;
use tower_http::cors::{Any, CorsLayer};
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use std::net::SocketAddr;

pub mod ws;
pub mod handlers;

pub struct AppState {
    pub ledger: Arc<Ledger>,
    pub sync_manager: Arc<deve_core::sync::SyncManager>,
    pub tx: broadcast::Sender<ServerMessage>,
    pub vault_path: std::path::PathBuf,
}

pub async fn start_server(ledger: Arc<Ledger>, vault_path: std::path::PathBuf, port: u16) -> anyhow::Result<()> {
    // Create broadcast channel for WS server
    let (tx, _rx) = broadcast::channel(100);
    
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(ledger.clone(), vault_path.clone()));

    // SPAWN WATCHER
    let tx_for_watcher = tx.clone();
    let sm_for_watcher = sync_manager.clone();
    let vp_for_watcher = vault_path.clone();
    
    tokio::task::spawn_blocking(move || {
        let watcher = deve_core::watcher::Watcher::new(sm_for_watcher, vp_for_watcher)
            .with_callback(move |msgs| {
                for msg in msgs {
                    let _ = tx_for_watcher.send(msg);
                }
            });
            
        if let Err(e) = watcher.watch() {
            tracing::error!("Watcher failed: {:?}", e);
        }
    });

    let app_state = Arc::new(AppState { 
        ledger: ledger.clone(), // Clone the Arc
        sync_manager,
        tx,
        vault_path,
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
