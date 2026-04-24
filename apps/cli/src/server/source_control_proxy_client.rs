//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

pub(super) fn build_client(base_url: &str) -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if is_loopback_url(base_url) {
        builder = builder.no_proxy();
    }
    builder.build().expect("build source control HTTP client")
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
    use super::is_loopback_url;

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
}
