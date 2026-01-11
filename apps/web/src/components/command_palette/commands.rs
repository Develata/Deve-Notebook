//! Static command definitions for the command palette.

use leptos::prelude::*;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use super::types::Command;

/// Creates the list of static (non-file) commands.
pub fn create_static_commands(
    locale: Locale,
    on_settings: Callback<()>,
    set_show: WriteSignal<bool>,
    locale_signal: RwSignal<Locale>,
) -> Vec<Command> {
    vec![
        Command {
            id: "settings".to_string(), 
            title: (t::command_palette::open_settings)(locale).to_string(),
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    on_settings.run(());
                    set_show.set(false);
                });
            }),
            is_file: false,
        },
        Command {
            id: "lang".to_string(),
            title: (t::command_palette::toggle_language)(locale).to_string(),
            action: Callback::new(move |_| {
                request_animation_frame(move || {
                    locale_signal.update(|l| *l = l.toggle());
                    set_show.set(false);
                });
            }),
            is_file: false,
        }
    ]
}

/// Creates file commands from the document list.
pub fn create_file_commands(
    docs: Vec<(DocId, String)>,
    query: &str,
    on_select_doc: Callback<DocId>,
    set_show: WriteSignal<bool>,
) -> Vec<Command> {
    let q = query.to_lowercase();
    
    docs.into_iter()
        .filter(|(_, path)| q.is_empty() || path.to_lowercase().contains(&q))
        .map(|(id, path)| {
            Command {
                id: format!("doc-{}", id),
                title: path,
                action: Callback::new(move |_| {
                    request_animation_frame(move || {
                        on_select_doc.run(id);
                        set_show.set(false);
                    });
                }),
                is_file: true,
            }
        })
        .collect()
}

/// Filters and combines all commands based on the query.
pub fn filter_commands(
    query: &str,
    static_commands: Vec<Command>,
    file_commands: Vec<Command>,
    max_results: usize,
) -> Vec<Command> {
    let q = query.to_lowercase();
    let mut results = Vec::new();
    
    // Filter static commands
    for cmd in static_commands {
        if q.is_empty() || cmd.title.to_lowercase().contains(&q) {
            results.push(cmd);
        }
    }
    
    // Add file commands (already filtered)
    results.extend(file_commands);
    
    // Limit results
    if results.len() > max_results {
        results.truncate(max_results);
    }
    
    results
}
