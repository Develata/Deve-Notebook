//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 04_repository#repo-scope-runtime
//!
use crate::components::search_box::score::score_desc;
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
use crate::i18n::{Locale, t};

pub const LOCAL_BRANCH_LABEL: &str = "Local";

pub struct BranchProvider {
    branches: Vec<String>,
    current_branch: Option<String>,
    locale: Locale,
}

impl BranchProvider {
    pub fn new(shadows: Vec<String>, current: Option<String>, locale: Locale) -> Self {
        let mut branches = vec![LOCAL_BRANCH_LABEL.to_string()];
        branches.extend(shadows);
        Self {
            branches,
            current_branch: current,
            locale,
        }
    }
}

impl SearchProvider for BranchProvider {
    fn search(&self, query: &str) -> Vec<SearchResult> {
        let clean_query = query.strip_prefix('@').unwrap_or(query).trim();

        let mut results: Vec<SearchResult> = self
            .branches
            .iter()
            .map(|name| {
                let score = if clean_query.is_empty() {
                    1.0
                } else {
                    sublime_fuzzy::best_match(clean_query, name)
                        .map(|m| m.score() as f32)
                        .unwrap_or(0.0)
                };
                (name, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .map(|(name, score)| SearchResult {
                id: name.clone(),
                title: name.clone(),
                detail: branch_detail(name, self.current_branch.as_deref(), self.locale),
                score,
                action: SearchAction::SwitchBranch(name.clone()),
            })
            .collect();

        results.sort_by(|a, b| score_desc(a.score, b.score));
        results
    }
}

fn branch_detail(name: &str, current_branch: Option<&str>, locale: Locale) -> Option<String> {
    if current_branch == Some(name) {
        Some(t::search::current_branch(locale).to_string())
    } else if name == LOCAL_BRANCH_LABEL {
        None
    } else {
        Some(t::search::remote_branch(locale).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{BranchProvider, branch_detail};
    use crate::components::search_box::types::SearchProvider;
    use crate::i18n::{Locale, t};

    #[test]
    fn branch_detail_keeps_local_entry_local_when_remote_is_current() {
        assert_eq!(branch_detail("Local", Some("peer-a"), Locale::En), None);
    }

    #[test]
    fn branch_provider_marks_local_entry_as_local_when_viewing_remote() {
        let provider =
            BranchProvider::new(vec!["peer-a".into()], Some("peer-a".into()), Locale::En);
        let results = provider.search("@");
        let local = results
            .iter()
            .find(|result| result.title == "Local")
            .expect("missing local branch entry");
        assert_eq!(local.detail, None);
    }

    #[test]
    fn branch_provider_marks_remote_entry_as_remote_when_not_current() {
        let provider = BranchProvider::new(vec!["peer-a".into()], Some("Local".into()), Locale::En);
        let results = provider.search("@");
        let remote = results
            .iter()
            .find(|result| result.title == "peer-a")
            .expect("missing remote branch entry");
        assert_eq!(remote.detail.as_deref(), Some("Remote Branch"));
    }

    #[test]
    fn branch_provider_localizes_remote_detail() {
        let provider = BranchProvider::new(vec!["peer-a".into()], Some("Local".into()), Locale::Zh);
        let results = provider.search("@");
        let remote = results
            .iter()
            .find(|result| result.title == "peer-a")
            .expect("missing remote branch entry");
        assert_eq!(
            remote.detail.as_deref(),
            Some(t::search::remote_branch(Locale::Zh))
        );
    }
}
