//! plan_ref:
//!   - 08_auth#jwt-cookie-contract
//!
//! Shared auth cookie policy helpers.

pub(crate) fn secure_cookies_enabled() -> bool {
    match std::env::var("HTTPS_ENABLED") {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => {
                tracing::warn!(
                    "Invalid HTTPS_ENABLED value '{}'; keeping Secure cookies enabled",
                    value
                );
                true
            }
        },
        Err(std::env::VarError::NotPresent) => true,
        Err(err) => {
            tracing::warn!("Failed to read HTTPS_ENABLED: {err}; keeping Secure cookies enabled");
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::secure_cookies_enabled;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn https_enabled_invalid_value_fails_secure() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let _env = EnvGuard::set("HTTPS_ENABLED", Some("maybe"));

        assert!(secure_cookies_enabled());
    }

    #[test]
    fn https_enabled_explicit_false_disables_secure_cookie() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let _env = EnvGuard::set("HTTPS_ENABLED", Some("false"));

        assert!(!secure_cookies_enabled());
    }

    struct EnvGuard {
        key: &'static str,
        old: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let old = std::env::var(key).ok();
            // SAFETY: tests serialize environment mutation through ENV_LOCK.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            Self { key, old }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: EnvGuard restores only the key it changed while ENV_LOCK is held.
            unsafe {
                match &self.old {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
}
