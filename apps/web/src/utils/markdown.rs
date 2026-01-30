// apps/web/src/utils/markdown.rs
//! Lightweight Markdown renderer with HTML filtering.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub fn render_markdown(source: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut out = String::new();
    let mut buffer: Vec<Event> = Vec::new();
    let mut iter = Parser::new_ext(source, options)
        .filter(|event| match event {
            Event::Html(tag) | Event::InlineHtml(tag) => {
                let t = tag.trim();
                t.eq_ignore_ascii_case("<br>")
                    || t.eq_ignore_ascii_case("<br/>")
                    || t.eq_ignore_ascii_case("<br />")
            }
            _ => true,
        })
        .peekable();

    while let Some(event) = iter.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                if !buffer.is_empty() {
                    html::push_html(&mut out, buffer.drain(..));
                }
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                let mut code = String::new();
                for ev in iter.by_ref() {
                    match ev {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(t) | Event::Code(t) => code.push_str(&t),
                        Event::SoftBreak | Event::HardBreak => code.push('\n'),
                        _ => {}
                    }
                }
                out.push_str(&render_code_block(&code, &lang));
            }
            other => buffer.push(other),
        }
    }

    if !buffer.is_empty() {
        html::push_html(&mut out, buffer.drain(..));
    }

    out
}

fn render_code_block(code: &str, lang: &str) -> String {
    let escaped = escape_html(code);
    let encoded = STANDARD.encode(code.as_bytes());
    let lang_class = if lang.is_empty() {
        "".to_string()
    } else {
        format!("language-{}", lang)
    };

    format!(
        "<div class=\"markdown-code-block\"><div class=\"code-toolbar\"><button class=\"apply-code\" data-code=\"{}\">Apply</button></div><pre><code class=\"{}\">{}</code></pre></div>",
        encoded,
        lang_class,
        escaped
    )
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
