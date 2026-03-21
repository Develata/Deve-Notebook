use super::read_main_port_hint;

#[test]
fn missing_main_port_hint_is_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let port = read_main_port_hint(dir.path()).expect("missing hint should be allowed");
    assert_eq!(port, None);
}

#[test]
fn invalid_main_port_hint_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let host = dir.path().join(".host");
    std::fs::create_dir_all(&host).expect("host dir");
    std::fs::write(host.join("main_port"), "not-a-port").expect("write hint");

    let err = read_main_port_hint(dir.path()).expect_err("invalid hint must fail closed");
    assert!(err.to_string().contains("Invalid main port hint"));
}
