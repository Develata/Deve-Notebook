//! Command types for the command palette.

use leptos::prelude::*;

/// A command that can be executed from the palette.
#[derive(Clone, Debug)]
pub struct Command {
    /// Unique identifier for the command.
    pub id: String,
    /// Display title shown in the palette.
    pub title: String,
    /// Action to execute when the command is selected.
    pub action: Callback<()>,
    /// Whether this command represents a file/document.
    pub is_file: bool,
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
