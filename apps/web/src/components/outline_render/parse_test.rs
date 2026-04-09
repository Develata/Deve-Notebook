// apps/web/src/components/outline_render/parse_test.rs
//! Tests for outline inline parsing.

use super::parse::{SegmentKind, split_inline_segments};

#[test]
fn outline_parser_keeps_highlight_syntax_as_plain_text() {
    let segments = split_inline_segments("**bold** ==highlight== $a^2$");
    let kinds: Vec<_> = segments.iter().map(|seg| seg.kind).collect();
    let texts: Vec<_> = segments.iter().map(|seg| seg.text.as_str()).collect();

    assert_eq!(
        kinds,
        vec![SegmentKind::Strong, SegmentKind::Text, SegmentKind::Math,]
    );
    assert_eq!(texts, vec!["bold", " ==highlight== ", "a^2"]);
}

#[test]
fn outline_parser_still_renders_emphasis_and_strike() {
    let segments = split_inline_segments("*em* ~~del~~");
    let kinds: Vec<_> = segments.iter().map(|seg| seg.kind).collect();
    let texts: Vec<_> = segments.iter().map(|seg| seg.text.as_str()).collect();

    assert_eq!(
        kinds,
        vec![SegmentKind::Em, SegmentKind::Text, SegmentKind::Del]
    );
    assert_eq!(texts, vec!["em", " ", "del"]);
}
