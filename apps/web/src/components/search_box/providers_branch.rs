use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};

pub const LOCAL_BRANCH_LABEL: &str = "Local";

pub struct BranchProvider {
    branches: Vec<String>,
    current_branch: Option<String>,
}

impl BranchProvider {
    pub fn new(shadows: Vec<String>, current: Option<String>) -> Self {
        let mut branches = vec![LOCAL_BRANCH_LABEL.to_string()];
        branches.extend(shadows);
        Self {
            branches,
            current_branch: current,
        }
    }
}

impl SearchProvider for BranchProvider {
    fn trigger_char(&self) -> Option<char> {
        Some('@')
    }

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
                detail: branch_detail(name, self.current_branch.as_deref()),
                score,
                action: SearchAction::SwitchBranch(name.clone()),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results
    }

    fn execute(&self, _action: &SearchAction) {}
}

fn branch_detail(name: &str, current_branch: Option<&str>) -> Option<String> {
    if current_branch == Some(name) {
        Some("Current Branch".to_string())
    } else if name == LOCAL_BRANCH_LABEL {
        Some("Local Branch".to_string())
    } else {
        Some("Remote Branch".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{BranchProvider, branch_detail};
    use crate::components::search_box::types::SearchProvider;

    #[test]
    fn branch_detail_keeps_local_entry_local_when_remote_is_current() {
        assert_eq!(
            branch_detail("Local", Some("peer-a")),
            Some("Local Branch".to_string())
        );
    }

    #[test]
    fn branch_provider_marks_local_entry_as_local_when_viewing_remote() {
        let provider = BranchProvider::new(vec!["peer-a".into()], Some("peer-a".into()));
        let results = provider.search("@");
        let local = results
            .iter()
            .find(|result| result.title == "Local")
            .expect("missing local branch entry");
        assert_eq!(local.detail.as_deref(), Some("Local Branch"));
    }
}
