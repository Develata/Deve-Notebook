#[derive(Clone, PartialEq)]
pub enum AuthState {
    Unauthenticated,
    Authenticating,
    Authenticated,
    Failed(String),
}
