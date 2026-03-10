use gloo_net::http::Request;

pub enum AuthProbe {
    Valid,
    Invalid,
    Unknown,
}

pub async fn probe_auth_status() -> AuthProbe {
    match Request::get("/api/auth/me").send().await {
        Ok(response) if response.ok() => AuthProbe::Valid,
        Ok(response) if response.status() == 401 => AuthProbe::Invalid,
        Ok(_) => AuthProbe::Unknown,
        Err(_) => AuthProbe::Unknown,
    }
}
