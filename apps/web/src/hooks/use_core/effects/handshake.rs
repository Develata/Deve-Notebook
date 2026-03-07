use crate::api::{ConnectionStatus, WsService};
use crate::storage::DegradedSyncMode;
use crate::storage::identity::{
    StoredPeerIdentity, note_handshake, save_repo_vector, sign_sync_hello,
};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// 设置握手 Effect。
pub fn setup(
    ws: &WsService,
    identity: ReadSignal<Option<StoredPeerIdentity>>,
    repo_vector: ReadSignal<VersionVector>,
    degraded: ReadSignal<Option<DegradedSyncMode>>,
) {
    let ws_clone = ws.clone();
    let status_signal = ws.status;
    let endpoint_signal = ws.endpoint;
    let last_mode = Rc::new(RefCell::new(None::<String>));

    Effect::new(move |_| {
        if status_signal.get() != ConnectionStatus::Connected {
            *last_mode.borrow_mut() = None;
            return;
        }

        let Some(mode_key) = degraded
            .get()
            .as_ref()
            .map(|_| format!("{}::degraded", endpoint_signal.get()))
            .or_else(|| {
                identity
                    .get()
                    .as_ref()
                    .map(|id| format!("{}::{}", endpoint_signal.get(), id.repo_id))
            })
        else {
            return;
        };
        if last_mode.borrow().as_deref() == Some(mode_key.as_str()) {
            return;
        }
        *last_mode.borrow_mut() = Some(mode_key);

        let ws = ws_clone.clone();
        let maybe_mode = degraded.get();
        let maybe_identity = identity.get();
        let vector = repo_vector.get();
        spawn_local(async move {
            if let Some(mode) = maybe_mode {
                leptos::logging::warn!("{}", mode.banner_text());
                ws.send(ClientMessage::ListDocs);
                ws.send(ClientMessage::ListRepos);
                return;
            }
            let Some(identity) = maybe_identity else {
                return;
            };

            leptos::logging::log!("已连接! 发送 SyncHello...");
            let sorted_map: BTreeMap<_, _> = vector.iter().collect();
            let vec_bytes = serde_json::to_vec(&sorted_map).unwrap_or_default();
            let mut msg = Vec::new();
            msg.extend_from_slice(b"deve-handshake");
            msg.extend_from_slice(identity.peer_id.as_bytes());
            msg.extend_from_slice(&vec_bytes);

            match sign_sync_hello(&identity, &msg).await {
                Ok(signature) => {
                    let peer_id = PeerId::new(&identity.peer_id);
                    let vector_json = serde_json::to_string(&vector).unwrap_or_default();
                    let _ = save_repo_vector(&identity.repo_id, &vector_json).await;
                    let _ = note_handshake(&identity.repo_id).await;
                    let repo_id = uuid::Uuid::parse_str(&identity.repo_id)
                        .unwrap_or_else(|_| uuid::Uuid::nil());
                    ws.send(ClientMessage::SyncHello {
                        peer_id,
                        pub_key: identity.public_key.clone(),
                        signature,
                        vector,
                        repo_id,
                    });
                }
                Err(err) => leptos::logging::error!("WebCrypto 握手签名失败: {}", err),
            }
            ws.send(ClientMessage::ListDocs);
            ws.send(ClientMessage::ListRepos);
        });
    });
}
