use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::types::ChatMessage;

pub(super) fn accepts_unscoped_update(
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    pending_branch_switch.is_none() && pending_repo_switch.is_none()
}

pub(super) fn contains_request_id(req_id: &str, request_ids: &[String]) -> bool {
    request_ids.iter().any(|id| id == req_id)
}

pub(super) fn contains_chat_message(req_id: &str, messages: &[ChatMessage]) -> bool {
    messages
        .iter()
        .any(|message| message.req_id.as_deref() == Some(req_id))
}
