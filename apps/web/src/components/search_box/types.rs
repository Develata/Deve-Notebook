use deve_core::models::DocId;
use crate::components::command_palette::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum SearchAction {
    OpenDoc(DocId),
    RunCommand(Command),
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
