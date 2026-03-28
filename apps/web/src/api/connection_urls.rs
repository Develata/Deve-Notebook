use super::connection::DEV_WS_PORT;

pub(super) fn build_same_origin_ws_url() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return format!("ws://localhost:{DEV_WS_PORT}/ws"),
    };
    let location = window.location();
    let host = location
        .host()
        .unwrap_or_else(|_| "localhost:3001".to_string());
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    format!("{}://{}/ws", ws_scheme, host)
}

pub(super) fn build_ws_urls() -> Vec<String> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return vec![format!("ws://localhost:{DEV_WS_PORT}/ws")],
    };
    let location = window.location();
    let hostname = normalize_hostname(
        location
            .hostname()
            .unwrap_or_else(|_| "localhost".to_string()),
    );
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    let mut urls = Vec::new();

    if let Some(port) = query_port() {
        push_ws_url(
            &mut urls,
            format!("{}://{}:{}/ws", ws_scheme, hostname, port),
        );
    }

    push_ws_url(&mut urls, build_same_origin_ws_url());

    if cfg!(debug_assertions) {
        push_ws_url(
            &mut urls,
            format!("{}://{}:{}/ws", ws_scheme, hostname, DEV_WS_PORT),
        );
        push_ws_url(
            &mut urls,
            format!("{}://localhost:{}/ws", ws_scheme, DEV_WS_PORT),
        );
        push_ws_url(
            &mut urls,
            format!("{}://127.0.0.1:{}/ws", ws_scheme, DEV_WS_PORT),
        );
    }

    urls
}

fn normalize_hostname(hostname: String) -> String {
    match hostname.as_str() {
        "" | "0.0.0.0" | "::" | "[::]" => "localhost".to_string(),
        _ => hostname,
    }
}

fn push_ws_url(urls: &mut Vec<String>, url: String) {
    if !urls.iter().any(|current| current == &url) {
        urls.push(url);
    }
}

fn query_port() -> Option<u16> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    if search.is_empty() {
        return None;
    }
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let val = params.get("ws_port")?;
    val.parse::<u16>().ok()
}
