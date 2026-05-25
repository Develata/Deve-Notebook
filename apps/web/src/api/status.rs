//! plan_ref:
//!   - 11_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 08_auth#unauthorized-disconnected-ui
//!

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Unauthorized,
    NativeBootstrapInvalid,
    NativeSessionPending,
    NativeServiceOffline,
    NativeReprobeRequired,
    Connected,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting..."),
            ConnectionStatus::Unauthorized => write!(f, "Unauthorized"),
            ConnectionStatus::NativeBootstrapInvalid => write!(f, "Native Bootstrap Invalid"),
            ConnectionStatus::NativeSessionPending => write!(f, "Native Session Pending"),
            ConnectionStatus::NativeServiceOffline => write!(f, "Native Service Offline"),
            ConnectionStatus::NativeReprobeRequired => write!(f, "Native Reprobe Required"),
            ConnectionStatus::Connected => write!(f, "Connected"),
        }
    }
}
