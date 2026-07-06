use crate::config::Config;

#[test]
fn from_toml_str_checked_runs_static_runtime_validation() {
    let err = Config::from_toml_str_checked(
        r#"
[p2p]
connect_interval_ms = 0
"#,
    )
    .expect_err("writer-side config validation must reject invalid runtime values");

    assert!(err.to_string().contains("p2p.connect_interval_ms"));
}
