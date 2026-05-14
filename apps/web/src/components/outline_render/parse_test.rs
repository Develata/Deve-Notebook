// apps/web/src/components/outline_render/parse_test.rs
//! Tests for outline inline parsing.

use super::parse::{SegmentKind, split_inline_segments};
use super::scan::{find_math_close, find_next_char, find_style_close};

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

#[test]
fn outline_scan_helpers_fail_soft_on_non_char_boundary_start() {
    let text = "é$ok$";
    assert_eq!(find_next_char(text, 1, '$'), None);
    assert_eq!(find_math_close(text, 1), None);
    assert_eq!(find_style_close("é**bold**", 1, "**"), None);
}

#[test]
fn outline_parser_handles_multibyte_escape_without_panic() {
    let segments = split_inline_segments("\\é *em*");
    let kinds: Vec<_> = segments.iter().map(|seg| seg.kind).collect();
    let texts: Vec<_> = segments.iter().map(|seg| seg.text.as_str()).collect();

    assert_eq!(kinds, vec![SegmentKind::Text, SegmentKind::Em]);
    assert_eq!(texts, vec!["\\é ", "em"]);
}
