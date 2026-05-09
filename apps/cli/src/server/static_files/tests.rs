use super::*;
use axum::body;
use std::sync::{Mutex, OnceLock};
use tower::ServiceExt;

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock")
}

#[test]
fn test_default_static_dir() {
    let _guard = env_guard();
    // 确保默认路径不 panic
    unsafe { std::env::remove_var(ENV_STATIC_DIR) };
    let dir = resolve_static_dir();
    assert_eq!(dir, PathBuf::from(DEFAULT_STATIC_DIR));
}

#[test]
fn test_env_override() {
    let _guard = env_guard();
    unsafe { std::env::set_var(ENV_STATIC_DIR, "/custom/path") };
    let dir = resolve_static_dir();
    assert_eq!(dir, PathBuf::from("/custom/path"));
    unsafe { std::env::remove_var(ENV_STATIC_DIR) };
}

#[test]
fn validate_static_dir_override_fails_closed_when_dir_missing() {
    let _guard = env_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("missing-static");
    unsafe { std::env::set_var(ENV_STATIC_DIR, &missing) };

    let err =
        validate_static_dir_override().expect_err("configured missing static dir must fail closed");

    unsafe { std::env::remove_var(ENV_STATIC_DIR) };
    assert!(err.to_string().contains("Configured static dir missing"));
}

#[test]
fn validate_static_dir_override_fails_closed_when_index_missing() {
    let _guard = env_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let static_dir = dir.path().join("static");
    std::fs::create_dir_all(&static_dir).expect("mkdir static");
    unsafe { std::env::set_var(ENV_STATIC_DIR, &static_dir) };

    let err = validate_static_dir_override()
        .expect_err("configured static dir without index must fail closed");

    unsafe { std::env::remove_var(ENV_STATIC_DIR) };
    assert!(
        err.to_string().contains("Configured static index missing")
            || err.to_string().contains("index.html")
    );
}

#[test]
fn delivery_shape_reports_static_override_when_valid() {
    let _guard = env_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("index.html"), "<html></html>").expect("write index");
    unsafe { std::env::set_var(ENV_STATIC_DIR, dir.path()) };

    assert_eq!(delivery_shape(), "static-dir-override");

    unsafe { std::env::remove_var(ENV_STATIC_DIR) };
}

#[tokio::test]
async fn static_dir_spa_route_returns_index_with_ok_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("index.html"),
        "<html><body>deve-spa</body></html>",
    )
    .expect("write index");
    let app = static_fallback_from_dir::<()>(dir.path().to_path_buf());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/any/path")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.starts_with("text/html"));
    let body = body::to_bytes(response.into_body(), 4096)
        .await
        .expect("body");
    assert!(String::from_utf8_lossy(&body).contains("deve-spa"));
}

#[tokio::test]
async fn static_dir_unknown_api_route_does_not_fallback_to_index() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("index.html"),
        "<html><body>deve-spa</body></html>",
    )
    .expect("write index");
    let app = static_fallback_from_dir::<()>(dir.path().to_path_buf());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/missing")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body::to_bytes(response.into_body(), 4096)
        .await
        .expect("body");
    assert!(body.is_empty());
}

#[tokio::test]
async fn static_dir_unknown_ws_route_does_not_fallback_to_index() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("index.html"),
        "<html><body>deve-spa</body></html>",
    )
    .expect("write index");
    let app = static_fallback_from_dir::<()>(dir.path().to_path_buf());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws/missing")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body::to_bytes(response.into_body(), 4096)
        .await
        .expect("body");
    assert!(body.is_empty());
}

#[tokio::test]
async fn static_dir_serves_existing_asset_without_spa_fallback() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("index.html"),
        "<html><body>deve-spa</body></html>",
    )
    .expect("write index");
    let assets = dir.path().join("assets");
    std::fs::create_dir_all(&assets).expect("mkdir assets");
    std::fs::write(assets.join("app.js"), "console.log('deve');").expect("write js");
    let app = static_fallback_from_dir::<()>(dir.path().to_path_buf());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets/app.js")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body::to_bytes(response.into_body(), 4096)
        .await
        .expect("body");
    assert!(String::from_utf8_lossy(&body).contains("console.log"));
}

#[test]
fn spa_fallback_path_excludes_api_and_ws_runtime_paths() {
    assert!(is_spa_fallback_path("/any/path"));
    assert!(is_spa_fallback_path("/"));
    assert!(!is_spa_fallback_path("/api"));
    assert!(!is_spa_fallback_path("/api/missing"));
    assert!(!is_spa_fallback_path("/ws"));
    assert!(!is_spa_fallback_path("/ws/extra"));
}
