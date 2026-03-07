mod api;
mod page;
mod state;

pub use api::check_auth_status;
pub use page::LoginPage;
pub use state::AuthState;

#[allow(dead_code)]
pub async fn logout() -> Result<(), String> {
    api::logout().await
}
