#[derive(Clone, PartialEq)]
pub enum AuthState {
    Checking,
    Unavailable,
    Unauthenticated,
    Authenticating,
    Authenticated,
    Failed(String),
}
