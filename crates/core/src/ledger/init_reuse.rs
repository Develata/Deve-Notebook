//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-catalog-contract

use super::RepoInfo;

/// 判定 `init()` 在遇到同名 `.redb` 时是否可以安全复用现有物理库。
///
/// Invariants:
/// - 显式 `repo_url` 只允许与完全相同的 URL 复用。
/// - 缺失 `repo_url` 时，只允许复用本地隐式仓库（`urn:*` 或缺失 URL）。
/// - 不得在缺失 URL 的调用下静默复用显式外部 URL 仓库。
pub(super) fn should_reuse_existing_repo(requested_url: Option<&str>, existing: &RepoInfo) -> bool {
    match requested_url {
        Some(url) => existing.url.as_deref() == Some(url),
        None => existing
            .url
            .as_deref()
            .is_none_or(|url| url.starts_with("urn:")),
    }
}

#[cfg(test)]
mod tests {
    use super::should_reuse_existing_repo;
    use crate::ledger::RepoInfo;

    fn repo(url: Option<&str>) -> RepoInfo {
        RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: url.map(str::to_string),
        }
    }

    #[test]
    fn explicit_url_must_match_exactly() {
        assert!(should_reuse_existing_repo(
            Some("urn:test:wiki"),
            &repo(Some("urn:test:wiki")),
        ));
        assert!(!should_reuse_existing_repo(
            Some("urn:test:wiki"),
            &repo(Some("urn:test:other")),
        ));
    }

    #[test]
    fn implicit_init_only_reuses_implicit_local_urls() {
        assert!(should_reuse_existing_repo(None, &repo(None)));
        assert!(should_reuse_existing_repo(
            None,
            &repo(Some("urn:uuid:test"))
        ));
        assert!(!should_reuse_existing_repo(
            None,
            &repo(Some("https://example.com/wiki.git")),
        ));
    }
}
