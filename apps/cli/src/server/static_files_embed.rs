//! plan_ref:
//!   - 08_ui_design_01_web#single-binary-distribution
//!
//! Embedded SPA static asset fallback.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode, header};
use std::convert::Infallible;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_static.rs"));
}

pub(super) fn fallback<S: Clone + Send + Sync + 'static>() -> Option<Router<S>> {
    asset_for_path("/index.html")?;
    tracing::info!("Serving embedded frontend static assets");
    Some(Router::new().fallback_service(tower::service_fn(serve_asset)))
}

async fn serve_asset(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let response = asset_for_path(req.uri().path())
        .map(asset_response)
        .unwrap_or_else(not_found_response);
    Ok(response)
}

fn asset_for_path(path: &str) -> Option<&'static embedded::EmbeddedAsset> {
    asset_for_path_in(path, embedded::EMBEDDED_ASSETS)
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
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_for_path(asset.path))
        .body(Body::from(asset.bytes))
        .expect("embedded asset response")
}

fn not_found_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .expect("not found response")
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
    fn mime_for_path_covers_common_frontend_assets() {
        assert_eq!(mime_for_path("index.html"), "text/html; charset=utf-8");
        assert_eq!(mime_for_path("app.css"), "text/css; charset=utf-8");
        assert_eq!(mime_for_path("app.js"), "text/javascript; charset=utf-8");
        assert_eq!(mime_for_path("app.wasm"), "application/wasm");
    }
}
