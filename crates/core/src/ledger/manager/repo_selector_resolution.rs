//! plan_ref:
//!   - 04_repository#repo-selector-resolution-contract
//!
//! # Repo Selector Resolution
//!
//! Pure selector arbitration rules shared by display and execution repo lookup.

use anyhow::Result;

#[derive(Default)]
pub(super) struct LocalRepoCandidates {
    pub(super) by_id: Option<String>,
    pub(super) by_name: Option<String>,
}

pub(super) fn select_repo_name(
    candidates: &LocalRepoCandidates,
    fallback_names: impl FnOnce() -> Result<Vec<String>>,
) -> Result<String> {
    if let Some(name) = selected_candidate_name(candidates)? {
        return Ok(name);
    }
    let fallback_names = fallback_names()?;
    match fallback_names.as_slice() {
        [repo] => Ok(repo.clone()),
        [] => anyhow::bail!("No local repositories available"),
        _ => anyhow::bail!("Active repository not selected: multiple local repos exist"),
    }
}

fn selected_candidate_name(candidates: &LocalRepoCandidates) -> Result<Option<String>> {
    if let (Some(from_id), Some(from_name)) = (&candidates.by_id, &candidates.by_name)
        && from_id != from_name
    {
        anyhow::bail!(
            "Repo selector mismatch: repo_id resolved to {}, repo_name resolved to {}",
            from_id,
            from_name
        );
    }
    Ok(candidates
        .by_id
        .clone()
        .or_else(|| candidates.by_name.clone()))
}

#[cfg(test)]
mod tests {
    use super::{LocalRepoCandidates, select_repo_name};

    #[test]
    fn explicit_uuid_candidate_wins_without_fallback() {
        let candidates = LocalRepoCandidates {
            by_id: Some("main".into()),
            by_name: None,
        };

        assert_eq!(
            select_repo_name(&candidates, || Ok(vec![])).expect("selected"),
            "main"
        );
    }

    #[test]
    fn matching_candidates_select_once() {
        let candidates = LocalRepoCandidates {
            by_id: Some("main".into()),
            by_name: Some("main".into()),
        };

        assert_eq!(
            select_repo_name(&candidates, || Ok(vec!["other".into()])).expect("selected"),
            "main"
        );
    }

    #[test]
    fn mismatched_candidates_fail_closed() {
        let candidates = LocalRepoCandidates {
            by_id: Some("main".into()),
            by_name: Some("other".into()),
        };

        let err = select_repo_name(&candidates, || Ok(vec![])).expect_err("mismatch");
        assert!(err.to_string().contains("Repo selector mismatch"));
    }

    #[test]
    fn single_fallback_is_selected_without_explicit_candidates() {
        let candidates = LocalRepoCandidates::default();

        assert_eq!(
            select_repo_name(&candidates, || Ok(vec!["main".into()])).expect("selected"),
            "main"
        );
    }

    #[test]
    fn multiple_fallbacks_require_explicit_selector() {
        let candidates = LocalRepoCandidates::default();

        let err = select_repo_name(&candidates, || Ok(vec!["main".into(), "wiki".into()]))
            .expect_err("ambiguous");
        assert!(
            err.to_string()
                .contains("Active repository not selected: multiple local repos exist")
        );
    }

    #[test]
    fn explicit_candidate_does_not_touch_fallback() {
        let candidates = LocalRepoCandidates {
            by_id: Some("main".into()),
            by_name: None,
        };

        assert_eq!(
            select_repo_name(&candidates, || anyhow::bail!("fallback should stay lazy"))
                .expect("selected"),
            "main"
        );
    }
}
