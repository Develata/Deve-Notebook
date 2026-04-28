//! plan_ref:
//!   - 04_storage#browser-storage-layering
//!   - 06_repository#repo-scope-runtime
//!
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::VersionVector;
use leptos::prelude::{Set, WriteSignal};
use std::cell::RefCell;
use std::rc::Rc;

/// 验证并提取有效的 UUID 格式的 repo_id。
/// 如果不是有效的 UUID，返回 None 表示应等待服务端提供真实 ID。
pub(super) fn repo_scope(repo_id: Option<String>) -> Option<String> {
    repo_id.filter(|value| !value.is_empty() && uuid::Uuid::parse_str(value).is_ok())
}

pub(super) fn reset_repo_runtime(
    last_repo: &Rc<RefCell<Option<String>>>,
    set_identity: WriteSignal<Option<StoredPeerIdentity>>,
    set_repo_vector: WriteSignal<VersionVector>,
) {
    last_repo.borrow_mut().take();
    set_identity.set(None);
    set_repo_vector.set(VersionVector::new());
}

#[cfg(test)]
mod tests {
    use super::repo_scope;

    #[test]
    fn repo_scope_rejects_non_uuid_strings() {
        assert_eq!(repo_scope(Some("default".into())), None);
        assert_eq!(repo_scope(Some(String::new())), None);
    }

    #[test]
    fn repo_scope_accepts_uuid_strings() {
        let repo_id = uuid::Uuid::new_v4().to_string();
        assert_eq!(repo_scope(Some(repo_id.clone())), Some(repo_id));
    }
}
