use super::*;

#[test]
fn nonzero_taskkill_is_fail_closed_after_direct_fallback() {
    let error = validate_windows_tree_termination(42, Ok(false), Ok(())).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("descendant cleanup is unverified")
    );
}

#[test]
fn missing_taskkill_is_fail_closed_after_direct_fallback() {
    let error = validate_windows_tree_termination(
        42,
        Err(io::Error::new(io::ErrorKind::NotFound, "missing")),
        Ok(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("failed to start taskkill /T"));
}
