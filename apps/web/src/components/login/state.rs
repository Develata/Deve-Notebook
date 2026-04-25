//! plan_ref:
//!   - 09_auth#unauthorized-disconnected-ui
//!

#[derive(Clone, PartialEq)]
pub enum AuthState {
    Checking,
    Unavailable,
    Unauthenticated,
    Authenticating,
    Authenticated,
    Failed(String),
}
