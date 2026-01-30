// apps\web\src\components\search_box
use crate::components::command_palette::Command;
use deve_core::models::DocId;

#[derive(Clone, Debug, PartialEq)]
pub enum SearchAction {
    OpenDoc(DocId),
    RunCommand(Command),
    SwitchBranch(String),
    CreateDoc(String),
    FileOp(FileOpAction),
    InsertQuery(InsertQuery),
    Noop,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileOpKind {
    Move,
    Copy,
    Remove,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileOpAction {
    pub kind: FileOpKind,
    pub src: String,
    pub dst: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InsertQuery {
    pub query: String,
    pub cursor: usize,
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
