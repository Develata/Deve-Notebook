//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 15_settings#native-host-local-backend-preference

use crate::i18n::Locale;

use super::native_backend_unavailable;

pub fn native_backend_error(locale: Locale, error: Option<&str>) -> String {
    let Some(error) = error.map(str::trim).filter(|error| !error.is_empty()) else {
        return native_backend_unavailable(locale).to_string();
    };
    if is_native_backend_bridge_unavailable(error) {
        return native_backend_unavailable(locale).to_string();
    }
    error.to_string()
}

fn is_native_backend_bridge_unavailable(error: &str) -> bool {
    error == "native backend bridge unavailable"
        || error == "native backend bridge call failed"
        || (error.starts_with("native backend bridge method ") && error.ends_with(" unavailable"))
}

#[cfg(test)]
mod tests {
    use super::native_backend_error;
    use crate::i18n::{Locale, t};

    #[test]
    fn native_backend_bridge_errors_map_to_localized_unavailable_copy() {
        assert_eq!(
            native_backend_error(Locale::Zh, Some("native backend bridge unavailable")),
            t::settings::native_backend_unavailable(Locale::Zh)
        );
        assert_eq!(
            native_backend_error(
                Locale::En,
                Some("native backend bridge method saveRemote unavailable"),
            ),
            t::settings::native_backend_unavailable(Locale::En)
        );
        assert_eq!(
            native_backend_error(Locale::Zh, Some("remote backend probe failed")),
            "remote backend probe failed"
        );
    }
}
