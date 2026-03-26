// apps\web\src\components\search_box
use crate::components::command_palette::Command;
use crate::components::search_box::types::{SearchAction, SearchProvider, SearchResult};
use deve_core::models::DocId;

pub const LOCAL_BRANCH_LABEL: &str = "Local";

// --- File Provider ---
pub struct FileProvider {
    docs: Vec<(DocId, String)>,
}

impl FileProvider {
    pub fn new(docs: Vec<(DocId, String)>) -> Self {
        Self { docs }
    }
}

impl SearchProvider for FileProvider {
    fn trigger_char(&self) -> Option<char> {
        None
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        // Use a HashSet to deduplicate by path (title)
        let mut seen_paths = std::collections::HashSet::new();

        if query.is_empty() {
            return self
                .docs
                .iter()
                .filter(|(_, path)| seen_paths.insert(path.clone()))
                .take(20)
                .map(|(id, path)| SearchResult {
                    id: id.to_string(),
                    title: path.clone(),
                    detail: None,
                    score: 1.0,
                    action: SearchAction::OpenDoc(*id),
                })
                .collect();
        }

        let mut results: Vec<SearchResult> = self
            .docs
            .iter()
            .map(|(id, path)| {
                let score = sublime_fuzzy::best_match(query, path)
                    .map(|m| m.score() as f32)
                    .unwrap_or(0.0);
                (id, path, score)
            })
            .filter(|(_, _, score)| *score > 0.0)
            .filter(|(_, path, _)| seen_paths.insert((*path).clone()))
            .map(|(id, path, score)| SearchResult {
                id: id.to_string(),
                title: path.clone(),
                detail: None,
                score,
                action: SearchAction::OpenDoc(*id),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);

        // 如果没有完全匹配，添加创建选项
        if !query.is_empty() && !results.iter().any(|r| r.title == query) {
            results.push(SearchResult {
                id: "create-doc".to_string(),
                title: format!("Create/Open '{}'", query),
                detail: Some("New File".to_string()),
                score: 0.1, // Low score to keep at bottom unless filtered out
                action: SearchAction::CreateDoc(query.to_string()),
            });
        }

        results
    }

    fn execute(&self, _action: &SearchAction) {
        // Validation only, execution handled by component
    }
}

// --- Command Provider ---
pub struct CommandProvider {
    commands: Vec<Command>,
}

impl CommandProvider {
    pub fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }
}

impl SearchProvider for CommandProvider {
    fn trigger_char(&self) -> Option<char> {
        Some('>')
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let clean_query = query.strip_prefix('>').unwrap_or(query);
        let clean_query = clean_query.trim();

        if clean_query.is_empty() {
            return self
                .commands
                .iter()
                .take(20)
                .map(|cmd| SearchResult {
                    id: cmd.id.clone(),
                    title: cmd.title.clone(),
                    detail: Some("Command".to_string()),
                    score: 1.0,
                    action: SearchAction::RunCommand(cmd.clone()),
                })
                .collect();
        }

        let mut results: Vec<SearchResult> = self
            .commands
            .iter()
            .map(|cmd| {
                let score = sublime_fuzzy::best_match(clean_query, &cmd.title)
                    .map(|m| m.score() as f32)
                    .unwrap_or(0.0);
                (cmd, score)
            })
            .filter(|(_, score)| *score > 0.0)
            .map(|(cmd, score)| SearchResult {
                id: cmd.id.clone(),
                title: cmd.title.clone(),
                detail: Some("Command".to_string()),
                score,
                action: SearchAction::RunCommand(cmd.clone()),
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    fn execute(&self, _action: &SearchAction) {
        // Validation only
    }
}

// --- Branch Provider ---
pub struct BranchProvider {
    branches: Vec<String>,
    current_branch: Option<String>,
}

impl BranchProvider {
    pub fn new(shadows: Vec<String>, current: Option<String>) -> Self {
        // Collect all branches: "Local" + shadows
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
        let clean_query = query.strip_prefix('@').unwrap_or(query);
        let clean_query = clean_query.trim();

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

    fn execute(&self, _action: &SearchAction) {
        // Validation only
    }
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
