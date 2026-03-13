use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn error_code(err: &anyhow::Error) -> ServerErrorCode {
    if err.to_string().starts_with("Document not found:") {
        return ServerErrorCode::StorageNotFound;
    }
    ServerErrorCode::RequestFailed
}

pub(super) fn send_doc_error(ch: &DualChannel, context: &str, err: anyhow::Error) {
    ch.send_protocol_error(ServerError::with_detail(
        error_code(&err),
        format!("{}: {}", context, err),
    ));
}
