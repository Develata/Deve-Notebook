//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
pub(crate) fn cannot_action(action: &str, reason: &str) -> String {
    format!("Cannot {}: {}", action, reason)
}

pub(crate) fn cannot_send(action: &str, reason: &str) -> String {
    format!("Cannot send {}: {}", action, reason)
}

pub(crate) fn cannot_create_document(reason: &str) -> String {
    cannot_action("create document", reason)
}

#[cfg(test)]
mod tests {
    use super::{cannot_action, cannot_create_document, cannot_send};

    #[test]
    fn write_gate_banner_formats_messages() {
        assert_eq!(
            cannot_action("move document", "read-only"),
            "Cannot move document: read-only"
        );
        assert_eq!(
            cannot_send("DeleteDoc", "offline"),
            "Cannot send DeleteDoc: offline"
        );
        assert_eq!(
            cannot_create_document("snapshot loading"),
            "Cannot create document: snapshot loading"
        );
    }
}
