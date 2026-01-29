// apps\web\src\components\search_box
use crate::components::command_palette::Command;
use deve_core::models::DocId;

#[derive(Clone, Debug, PartialEq)]
pub enum SearchAction {
    OpenDoc(DocId),
    RunCommand(Command),
    SwitchBranch(String),
    CreateDoc(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub detail: Option<String>,
    pub score: f32,
    pub action: SearchAction,
}

pub trait SearchProvider {
    fn trigger_char(&self) -> Option<char>;
    fn search(&self, query: &str) -> Vec<SearchResult>;
    fn execute(&self, action: &SearchAction);
}
