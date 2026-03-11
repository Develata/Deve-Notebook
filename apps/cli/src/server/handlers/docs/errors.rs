use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    ch.send_protocol_error(ServerError::with_detail(code, detail));
}

pub(super) fn request_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::RequestFailed, detail);
}

pub(super) fn remote_branch_readonly(ch: &DualChannel) {
    ch.send_protocol_error(ServerError::new(ServerErrorCode::ScRemoteBranchReadonly));
}

pub(super) fn storage_not_found(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StorageNotFound, detail);
}

pub(super) fn storage_conflict(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StorageConflict, detail);
}

pub(super) fn storage_persist_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StoragePersistFailed, detail);
}
