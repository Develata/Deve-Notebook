// crates\core\src\protocol
//! 客户端消息的浏览器 scope gate 元数据。
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 09_web_thin_client_ledger#document-create-intent

use super::client::ClientMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientMessageScopeGate {
    pub scope_nonce: Option<u64>,
    pub scope_name: &'static str,
}

impl ClientMessageScopeGate {
    const fn new(scope_nonce: Option<u64>, scope_name: &'static str) -> Self {
        Self {
            scope_nonce,
            scope_name,
        }
    }
}

impl ClientMessage {
    pub fn core_scope_gate(&self) -> Option<ClientMessageScopeGate> {
        match self {
            Self::OpenDoc { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "open doc"))
            }
            Self::RequestHistory { scope_nonce, .. } => Some(ClientMessageScopeGate::new(
                *scope_nonce,
                "document history",
            )),
            Self::Edit { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "edit"))
            }
            Self::ListDocs { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "document list"))
            }
            Self::ListShadows { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "shadow list"))
            }
            Self::ListRepos { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "repo list"))
            }
            Self::Search { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "search"))
            }
            Self::DeletePeer { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "delete peer"))
            }
            Self::RequestKey { scope_nonce } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "request key"))
            }
            _ => None,
        }
    }

    pub fn document_scope_gate(&self) -> Option<ClientMessageScopeGate> {
        match self {
            Self::DocumentCreate(request) => Some(ClientMessageScopeGate::new(
                Some(request.scope_nonce.get()),
                "document create",
            )),
            Self::RenameDoc { scope_nonce, .. }
            | Self::DeleteDoc { scope_nonce, .. }
            | Self::CopyDoc { scope_nonce, .. }
            | Self::MoveDoc { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "document"))
            }
            _ => None,
        }
    }

    pub fn merge_control_scope_gate(&self) -> Option<ClientMessageScopeGate> {
        match self {
            Self::GetSyncMode { scope_nonce, .. }
            | Self::SetSyncMode { scope_nonce, .. }
            | Self::GetPendingOps { scope_nonce, .. }
            | Self::ConfirmMerge { scope_nonce }
            | Self::ResolveMergeConflict { scope_nonce, .. }
            | Self::DiscardPending { scope_nonce }
            | Self::MergePeer { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "merge control"))
            }
            _ => None,
        }
    }

    pub fn source_control_scope_gate(&self) -> Option<ClientMessageScopeGate> {
        match self {
            Self::GetChanges { scope_nonce, .. }
            | Self::StageFile { scope_nonce, .. }
            | Self::StageFiles { scope_nonce, .. }
            | Self::UnstageFile { scope_nonce, .. }
            | Self::UnstageFiles { scope_nonce, .. }
            | Self::DiscardFile { scope_nonce, .. }
            | Self::Commit { scope_nonce, .. }
            | Self::ApplyExternalChanges { scope_nonce, .. }
            | Self::GetCommitHistory { scope_nonce, .. }
            | Self::GetDocDiff { scope_nonce, .. }
            | Self::GetCommitDiff { scope_nonce, .. }
            | Self::GetCommitFileDiff { scope_nonce, .. }
            | Self::ComputeDiffProjection { scope_nonce, .. }
            | Self::ResolveConflict { scope_nonce, .. } => {
                Some(ClientMessageScopeGate::new(*scope_nonce, "source control"))
            }
            _ => None,
        }
    }

    pub fn remote_import_scope_gate(&self) -> Option<ClientMessageScopeGate> {
        match self {
            Self::RemoteImport(request) => Some(ClientMessageScopeGate::new(
                Some(request.context().scope_nonce.get()),
                "remote import",
            )),
            Self::RemoteProjectionPush(request) => Some(ClientMessageScopeGate::new(
                Some(request.scope_nonce.get()),
                "remote projection push",
            )),
            _ => None,
        }
    }
}
