use super::*;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_dev_default() {
    let _lock = ENV_LOCK.lock().expect("env test lock");
    let _env = EnvGuard::set(&[("AUTH_ALLOW_ANONYMOUS_LOCALHOST", None)]);
    let cfg = AuthConfig::dev_default().unwrap();
    assert_eq!(cfg.username, "admin");
    assert!(!cfg.allow_anonymous_localhost);
    assert!(cfg.secret.len() >= 32);
}

#[test]
fn anonymous_localhost_requires_development_env() {
    let _lock = ENV_LOCK.lock().expect("env test lock");
    let env = EnvGuard::set(&[
        ("DEVE_ENV", Some("production")),
        (
            "AUTH_SECRET",
            Some("test_secret_key_at_least_32_bytes_long!"),
        ),
        ("AUTH_PASS", Some(valid_argon2_hash())),
        ("AUTH_USER", Some("alice")),
        ("AUTH_ALLOW_ANONYMOUS_LOCALHOST", Some("true")),
        ("AUTH_TOKEN_VERSION", None),
    ]);

    let err = AuthConfig::from_env().expect_err("anonymous localhost must stay development-only");
    assert!(
        err.to_string()
            .contains("AUTH_ALLOW_ANONYMOUS_LOCALHOST requires DEVE_ENV=development")
    );
    drop(env);
}

#[test]
fn anonymous_localhost_is_allowed_in_development_env() {
    let _lock = ENV_LOCK.lock().expect("env test lock");
    let env = EnvGuard::set(&[
        ("DEVE_ENV", Some("development")),
        (
            "AUTH_SECRET",
            Some("test_secret_key_at_least_32_bytes_long!"),
        ),
        ("AUTH_PASS", Some(valid_argon2_hash())),
        ("AUTH_USER", Some("alice")),
        ("AUTH_ALLOW_ANONYMOUS_LOCALHOST", Some("true")),
        ("AUTH_TOKEN_VERSION", None),
    ]);

    let config = AuthConfig::from_env().expect("development anonymous localhost should load");
    assert!(config.allow_anonymous_localhost);
    drop(env);
}

#[test]
fn invalid_auth_pass_phc_fails_closed_at_config_load() {
    let _lock = ENV_LOCK.lock().expect("env test lock");
    let env = EnvGuard::set(&[
        ("DEVE_ENV", Some("production")),
        (
            "AUTH_SECRET",
            Some("test_secret_key_at_least_32_bytes_long!"),
        ),
        ("AUTH_PASS", Some("not-a-valid-phc-hash")),
        ("AUTH_USER", Some("alice")),
        ("AUTH_ALLOW_ANONYMOUS_LOCALHOST", None),
        ("AUTH_TOKEN_VERSION", None),
    ]);

    let err = AuthConfig::from_env().expect_err("invalid PHC must fail closed");
    assert!(
        err.to_string()
            .contains("AUTH_PASS must be a valid Argon2 PHC hash")
    );
    drop(env);
}

#[test]
fn missing_secret_or_password_fails_closed_in_production() {
    let _lock = ENV_LOCK.lock().expect("env test lock");
    let env = EnvGuard::set(&[
        ("DEVE_ENV", Some("production")),
        ("AUTH_SECRET", None),
        ("AUTH_PASS", None),
        ("AUTH_USER", None),
        ("AUTH_ALLOW_ANONYMOUS_LOCALHOST", None),
        ("AUTH_TOKEN_VERSION", None),
    ]);

    let err = AuthConfig::from_env().expect_err("missing production auth must fail closed");
    assert!(
        err.to_string()
            .contains("Production mode requires AUTH_SECRET and AUTH_PASS")
    );
    drop(env);
}

#[test]
fn invalid_auth_token_version_fails_closed() {
    let _lock = ENV_LOCK.lock().expect("env test lock");
    let env = EnvGuard::set(&[
        ("DEVE_ENV", Some("production")),
        (
            "AUTH_SECRET",
            Some("test_secret_key_at_least_32_bytes_long!"),
        ),
        ("AUTH_PASS", Some(valid_argon2_hash())),
        ("AUTH_USER", Some("alice")),
        ("AUTH_ALLOW_ANONYMOUS_LOCALHOST", None),
        ("AUTH_TOKEN_VERSION", Some("not-a-number")),
    ]);

    let err = AuthConfig::from_env().expect_err("invalid token version must fail closed");
    assert!(err.to_string().contains("AUTH_TOKEN_VERSION"));
    drop(env);
}

fn valid_argon2_hash() -> &'static str {
    "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$MLt1KZB+74lpz3bB5FzWzWgfz8Q1nXWJ7HfLqF6QL0M"
}

struct EnvGuard {
    old: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(vars: &[(&'static str, Option<&str>)]) -> Self {
        let old = vars
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in vars {
            // SAFETY: this test holds ENV_LOCK while mutating process env.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
        Self { old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.old.drain(..) {
            // SAFETY: EnvGuard restores only keys it changed while ENV_LOCK is held.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
