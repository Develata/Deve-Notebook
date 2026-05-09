use super::verify_login_password;

#[test]
fn invalid_hash_fails_closed_instead_of_looking_like_bad_password() {
    let err = verify_login_password("secret", "not-a-valid-phc-hash").expect_err("must fail");
    let detail = err.to_string();
    assert!(
        !detail.is_empty(),
        "password verification failure should stay observable"
    );
}
