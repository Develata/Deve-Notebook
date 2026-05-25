//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 04_repository#repo-scope-runtime
//!
use super::open_scope::OpenRequestKey;
use crate::hooks::use_core::EditorContext;
use deve_core::protocol::ConfirmedOp;
use deve_core::security::{EncryptedOp, RepoKey};
use leptos::prelude::*;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

pub(super) struct EditorRuntime {
    pub content: ReadSignal<String>,
    pub set_content: WriteSignal<String>,
    pub local_version: ReadSignal<u64>,
    pub set_local_version: WriteSignal<u64>,
    pub open_request_id: ReadSignal<u64>,
    pub set_open_request_id: WriteSignal<u64>,
    pub last_open_request_key: ReadSignal<Option<OpenRequestKey>>,
    pub set_last_open_request_key: WriteSignal<Option<OpenRequestKey>>,
    pub session_generation: Arc<AtomicU64>,
    pub ready_generation: Arc<AtomicU64>,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
    pub history: ReadSignal<Vec<(u64, deve_core::models::Op)>>,
    pub set_history: WriteSignal<Vec<(u64, deve_core::models::Op)>>,
    pub playback_version: ReadSignal<u64>,
    pub set_playback_version: WriteSignal<u64>,
    pub is_playback: ReadSignal<bool>,
    pub set_is_playback: WriteSignal<bool>,
    pub repo_key: ReadSignal<Option<RepoKey>>,
    pub set_repo_key: WriteSignal<Option<RepoKey>>,
    pub set_doc_version: WriteSignal<u64>,
}

pub(super) fn build_editor_runtime(core: &EditorContext) -> EditorRuntime {
    let (content, set_content) = signal("".to_string());
    let (local_version, set_local_version) = signal(0u64);
    let (open_request_id, set_open_request_id) = signal(0u64);
    let (last_open_request_key, set_last_open_request_key) = signal(None::<OpenRequestKey>);
    let session_generation = Arc::new(AtomicU64::new(0));
    let ready_generation = Arc::new(AtomicU64::new(0));
    let buffered_live_ops = Arc::new(Mutex::new(Vec::new()));
    let buffered_encrypted_ops = Arc::new(Mutex::new(Vec::<EncryptedOp>::new()));
    let (history, set_history) = signal(Vec::<(u64, deve_core::models::Op)>::new());
    let (is_playback, set_is_playback) = signal(false);
    let (repo_key, set_repo_key) = signal(None::<RepoKey>);

    EditorRuntime {
        content,
        set_content,
        local_version,
        set_local_version,
        open_request_id,
        set_open_request_id,
        last_open_request_key,
        set_last_open_request_key,
        session_generation,
        ready_generation,
        buffered_live_ops,
        buffered_encrypted_ops,
        history,
        set_history,
        playback_version: core.playback_version,
        set_playback_version: core.set_playback_version,
        is_playback,
        set_is_playback,
        repo_key,
        set_repo_key,
        set_doc_version: core.set_doc_version,
    }
}
