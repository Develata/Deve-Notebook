use crate::{DesktopBootstrap, DesktopNativeSessionCookie};

mod init_script;
mod local_service;

fn success_bootstrap() -> DesktopBootstrap {
    DesktopBootstrap {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound: true,
        capabilities: deve_core::native_adapter::NativeShellCapabilities::local_backend(),
    }
}

fn native_session_cookie() -> DesktopNativeSessionCookie {
    DesktopNativeSessionCookie::from_set_cookie(
        "token=abc.def; Path=/; HttpOnly; SameSite=None; Secure",
        "127.0.0.1",
    )
    .expect("cookie")
}
