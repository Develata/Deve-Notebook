use gloo_net::http::Request;
use leptos::prelude::*;

pub(super) async fn fetch_node_role(ws_url: String, set_node_role: WriteSignal<String>) {
    let http_url = ws_url
        .replace("wss://", "https://")
        .replace("ws://", "http://")
        .replace("/ws", "");
    let url = format!("{}/api/node/role", http_url);
    let res = Request::get(&url).send().await;
    if let Ok(resp) = res
        && let Ok(json) = resp.json::<serde_json::Value>().await
    {
        let role = json
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let main_port = json.get("main_port").and_then(|v| v.as_u64()).unwrap_or(0);
        let ws_port = json.get("ws_port").and_then(|v| v.as_u64()).unwrap_or(0);
        let text = if role == "proxy" && main_port > 0 {
            format!("proxy → {} (ws:{})", main_port, ws_port)
        } else if ws_port > 0 {
            format!("{} (ws:{})", role, ws_port)
        } else {
            role.to_string()
        };
        set_node_role.set(text);
    }
}
