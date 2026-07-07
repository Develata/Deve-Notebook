//! plan_ref:
//!   - 11_ui_design/01_web#single-binary-distribution
//!
//! Embedded SPA static asset fallback.

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Request, Response, StatusCode, header};
use std::convert::Infallible;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_static.rs"));
}

pub(super) fn fallback<S: Clone + Send + Sync + 'static>() -> Option<Router<S>> {
    valid_index_asset()?;
    tracing::info!("Serving embedded frontend static assets");
    Some(Router::new().fallback_service(tower::service_fn(serve_asset)))
}

pub(super) fn has_index_asset() -> bool {
    valid_index_asset().is_some()
}

fn valid_index_asset() -> Option<&'static embedded::EmbeddedAsset> {
    valid_index_asset_in(embedded::EMBEDDED_ASSETS)
}

fn valid_index_asset_in(assets: &[embedded::EmbeddedAsset]) -> Option<&embedded::EmbeddedAsset> {
    let asset = asset_for_path_in("/index.html", assets)?;
    if super::static_files::index_html_contains_trunk_dev_ws(asset.bytes) {
        tracing::warn!(
            "Embedded frontend index.html contains Trunk development live-reload markers; run `trunk build --release` before rebuilding deve_cli"
        );
        return None;
    }
    Some(asset)
}

async fn serve_asset(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let response = asset_for_request_path(req.uri().path())
        .map(asset_response)
        .unwrap_or_else(not_found_response);
    Ok(response)
}

fn asset_for_request_path(path: &str) -> Option<&'static embedded::EmbeddedAsset> {
    asset_for_request_path_in(path, embedded::EMBEDDED_ASSETS)
}

fn asset_for_request_path_in<'a>(
    path: &str,
    assets: &'a [embedded::EmbeddedAsset],
) -> Option<&'a embedded::EmbeddedAsset> {
    if !super::static_files::is_spa_fallback_path(path) {
        return None;
    }
    asset_for_path_in(path, assets)
}

fn asset_for_path_in<'a>(
    path: &str,
    assets: &'a [embedded::EmbeddedAsset],
) -> Option<&'a embedded::EmbeddedAsset> {
    let requested = normalize_request_path(path);
    assets
        .iter()
        .find(|asset| asset.path == requested)
        .or_else(|| assets.iter().find(|asset| asset.path == "index.html"))
}

fn normalize_request_path(path: &str) -> String {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return "index.html".into();
    }
    trimmed.to_string()
}

fn asset_response(asset: &embedded::EmbeddedAsset) -> Response<Body> {
    let mut response = Response::new(Body::from(asset.bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(mime_for_path(asset.path)),
    );
    response
}

fn not_found_response() -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

fn mime_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_lookup_serves_root_index() {
        let assets = [embedded::EmbeddedAsset {
            path: "index.html",
            bytes: b"index",
        }];

        let found = asset_for_path_in("/", &assets).expect("index asset");

        assert_eq!(found.path, "index.html");
    }

    #[test]
    fn embedded_lookup_rejects_trunk_dev_index() {
        let assets = [embedded::EmbeddedAsset {
            path: "index.html",
            bytes: b"<script>const address = '{{__TRUNK_ADDRESS__}}'; const url = '.well-known/trunk/ws';</script>",
        }];

        let found = valid_index_asset_in(&assets);

        assert!(found.is_none());
    }

    #[test]
    fn embedded_lookup_serves_nested_asset() {
        let assets = [
            embedded::EmbeddedAsset {
                path: "index.html",
                bytes: b"index",
            },
            embedded::EmbeddedAsset {
                path: "assets/app.js",
                bytes: b"js",
            },
        ];

        let found = asset_for_path_in("/assets/app.js", &assets).expect("js asset");

        assert_eq!(found.path, "assets/app.js");
    }

    #[test]
    fn embedded_lookup_falls_back_to_index_for_spa_route() {
        let assets = [embedded::EmbeddedAsset {
            path: "index.html",
            bytes: b"index",
        }];

        let found = asset_for_path_in("/docs/123", &assets).expect("spa fallback");

        assert_eq!(found.path, "index.html");
    }

    #[test]
    fn embedded_lookup_rejects_api_route_before_spa_fallback() {
        let assets = [embedded::EmbeddedAsset {
            path: "index.html",
            bytes: b"index",
        }];

        let found = asset_for_request_path_in("/api/missing", &assets);

        assert!(found.is_none());
    }

    #[test]
    fn embedded_lookup_rejects_ws_route_before_spa_fallback() {
        let assets = [embedded::EmbeddedAsset {
            path: "index.html",
            bytes: b"index",
        }];

        let found = asset_for_request_path_in("/ws/missing", &assets);

        assert!(found.is_none());
    }

    #[test]
    fn mime_for_path_covers_common_frontend_assets() {
        assert_eq!(mime_for_path("index.html"), "text/html; charset=utf-8");
        assert_eq!(mime_for_path("app.css"), "text/css; charset=utf-8");
        assert_eq!(mime_for_path("app.js"), "text/javascript; charset=utf-8");
        assert_eq!(mime_for_path("app.wasm"), "application/wasm");
    }
}
