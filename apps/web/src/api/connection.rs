// apps\web\src\api
//! # WebSocket 连接管理器
//!
//! ## 职责
//! 1. 建立 WebSocket 连接
//! 2. 指数退避重连策略
//! 3. 读取服务器消息并更新信号
//! 4. 通过 link_tx 将写入端传递给输出管理器

use deve_core::protocol::ServerMessage;
use futures::StreamExt;
use futures::channel::mpsc::UnboundedSender;
use futures::stream::SplitSink;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::ConnectionStatus;
use super::backoff::BackoffStrategy;

/// 启动连接管理器任务
pub fn spawn_connection_manager(
    set_status: WriteSignal<ConnectionStatus>,
    set_msg: WriteSignal<Option<ServerMessage>>,
    link_tx: UnboundedSender<SplitSink<WebSocket, Message>>,
) {
    spawn_local(async move {
        let url = build_ws_url();
        let mut backoff = BackoffStrategy::new();

        loop {
            set_status.set(ConnectionStatus::Connecting);
            leptos::logging::log!("WS: Connecting to {}...", url);

            match WebSocket::open(&url) {
                Ok(ws) => {
                    leptos::logging::log!("WS: Socket opened, waiting for first message...");
                    backoff.reset();

                    let (write, read) = ws.split();

                    // Hand over the writer to the Output Manager
                    if let Err(e) = link_tx.unbounded_send(write) {
                        leptos::logging::error!("Failed to send sink to output loop: {:?}", e);
                    }

                    // Block on reading until disconnect
                    // Pass set_status to confirm connection after first successful message
                    process_incoming_messages(read, set_msg.clone(), set_status.clone()).await;

                    leptos::logging::log!("WS: Connection Lost (Reader ended)");
                }
                Err(e) => {
                    leptos::logging::error!("WS Open Error: {:?}", e);
                }
            }

            set_status.set(ConnectionStatus::Disconnected);
            backoff.wait().await;
        }
    });
}

/// 从 WebSocket 读取消息直到连接关闭。
/// 在收到第一条成功消息后将状态设置为 Connected。
async fn process_incoming_messages(
    mut read: futures::stream::SplitStream<WebSocket>,
    set_msg: WriteSignal<Option<ServerMessage>>,
    set_status: WriteSignal<ConnectionStatus>,
) {
    let mut confirmed_connected = false;

    while let Some(result) = read.next().await {
        match result {
            Ok(Message::Text(txt)) => {
                // First successful message confirms the connection
                if !confirmed_connected {
                    leptos::logging::log!("WS: First message received, connection confirmed!");
                    set_status.set(ConnectionStatus::Connected);
                    confirmed_connected = true;
                }

                match serde_json::from_str::<ServerMessage>(&txt) {
                    Ok(server_msg) => set_msg.set(Some(server_msg)),
                    Err(e) => leptos::logging::error!("Parse Error: {:?}", e),
                }
            }
            Ok(_) => {} // Ignore binary messages
            Err(e) => {
                leptos::logging::error!("WS Read Error: {:?}", e);
                break;
            }
        }
    }
}

/// 根据当前主机名构建 WebSocket URL
fn build_ws_url() -> String {
    let hostname = web_sys::window()
        .expect("Window 对象不存在 (非浏览器环境?)")
        .location()
        .hostname()
        .unwrap_or_else(|_| "localhost".to_string());
    format!("ws://{}:3001/ws", hostname)
}
