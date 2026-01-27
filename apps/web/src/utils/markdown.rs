// apps/web/src/utils/markdown.rs
//! Lightweight Markdown renderer with HTML filtering.

use pulldown_cmark::{Event, Options, Parser, html};

pub fn render_markdown(source: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(source, options).filter(|event| {
        !matches!(event, Event::Html(_) | Event::InlineHtml(_))
    });

    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}
