//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use anyhow::{Context, Result};

pub(super) fn build_client(base_url: &str) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if is_loopback_url(base_url) {
        builder = builder.no_proxy();
    }
    builder
        .build()
        .context("Failed to build source control HTTP client")
}

fn is_loopback_url(base_url: &str) -> bool {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .is_some_and(|host| {
            let host = host.trim_start_matches('[').trim_end_matches(']');
            host == "localhost" || is_loopback_ip(host)
        })
}

fn is_loopback_ip(host: &str) -> bool {
    host.parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
}

#[cfg(test)]
mod tests {
    use super::{build_client, is_loopback_url};

    #[test]
    fn detects_loopback_base_urls() {
        assert!(is_loopback_url("http://127.0.0.1:3000"));
        assert!(is_loopback_url("http://[::1]:3000"));
        assert!(is_loopback_url("http://localhost:3000"));
    }

    #[test]
    fn leaves_remote_urls_proxy_eligible() {
        assert!(!is_loopback_url("https://example.com"));
        assert!(!is_loopback_url("http://10.0.0.2:3000"));
    }

    #[test]
    fn builds_source_control_http_client_without_panicking() {
        assert!(build_client("http://127.0.0.1:3000").is_ok());
        assert!(build_client("https://example.com").is_ok());
    }
}
