//! plan_ref:
//!   - 08_auth#localhost-dev-policy
//!   - 18_release#runtime-observability
//!
//! Effective runtime environment selection shared by server auth, CORS, and
//! public node-role reporting.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEnvironment {
    Production,
    Development,
}

impl RuntimeEnvironment {
    pub fn from_env() -> Self {
        Self::from_deve_env(std::env::var("DEVE_ENV").ok().as_deref())
    }

    pub fn from_deve_env(value: Option<&str>) -> Self {
        match value {
            Some(value) if value.trim().eq_ignore_ascii_case("development") => Self::Development,
            _ => Self::Production,
        }
    }

    pub fn for_serve(dev: bool) -> Self {
        if dev {
            Self::Development
        } else {
            Self::from_env()
        }
    }

    pub fn is_development(self) -> bool {
        matches!(self, Self::Development)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Development => "development",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeEnvironment;

    #[test]
    fn deve_env_only_treats_explicit_development_as_dev() {
        assert_eq!(
            RuntimeEnvironment::from_deve_env(Some("development")),
            RuntimeEnvironment::Development
        );
        assert_eq!(
            RuntimeEnvironment::from_deve_env(Some(" Development ")),
            RuntimeEnvironment::Development
        );
        assert_eq!(
            RuntimeEnvironment::from_deve_env(Some("production")),
            RuntimeEnvironment::Production
        );
        assert_eq!(
            RuntimeEnvironment::from_deve_env(None),
            RuntimeEnvironment::Production
        );
    }

    #[test]
    fn serve_dev_flag_overrides_deve_env_reader() {
        assert_eq!(
            RuntimeEnvironment::for_serve(true),
            RuntimeEnvironment::Development
        );
    }
}
