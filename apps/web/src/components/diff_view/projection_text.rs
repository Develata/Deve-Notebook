//! UTF-16 wire-range to UTF-8 DOM text adapter.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract

use deve_core::source_control::diff_projection::DiffTextRange;

pub(crate) fn highlighted_parts(
    text: &str,
    ranges: &[DiffTextRange],
) -> Result<Vec<(String, bool)>, &'static str> {
    if ranges.is_empty() {
        return Ok(vec![(text.to_string(), false)]);
    }
    let mut wanted = Vec::with_capacity(ranges.len() * 2);
    let mut previous_end = 0u32;
    for range in ranges {
        if range.start > range.end || range.start < previous_end {
            return Err("reversed or overlapping UTF-16 range");
        }
        wanted.push(range.start);
        wanted.push(range.end);
        previous_end = range.end;
    }
    let mut boundaries = Vec::with_capacity(wanted.len());
    let mut wanted_index = 0usize;
    let mut utf16 = 0u32;
    for (byte, ch) in text.char_indices() {
        while wanted_index < wanted.len() && wanted[wanted_index] == utf16 {
            boundaries.push(byte);
            wanted_index += 1;
        }
        if wanted_index < wanted.len() && wanted[wanted_index] < utf16 {
            return Err("UTF-16 range is not on a scalar boundary");
        }
        utf16 = utf16.saturating_add(ch.len_utf16() as u32);
    }
    while wanted_index < wanted.len() && wanted[wanted_index] == utf16 {
        boundaries.push(text.len());
        wanted_index += 1;
    }
    if wanted_index != wanted.len() {
        return Err("UTF-16 range outside cell text or inside surrogate pair");
    }
    let mut parts = Vec::with_capacity(ranges.len() * 2 + 1);
    let mut cursor = 0usize;
    for pair in boundaries.chunks_exact(2) {
        let start = pair[0];
        let end = pair[1];
        if start > cursor {
            parts.push((text[cursor..start].to_string(), false));
        }
        if end > start {
            parts.push((text[start..end].to_string(), true));
        }
        cursor = end;
    }
    if cursor < text.len() {
        parts.push((text[cursor..].to_string(), false));
    }
    Ok(parts)
}

pub(crate) fn validate_highlight_ranges(
    text: &str,
    ranges: &[DiffTextRange],
) -> Result<(), &'static str> {
    let mut previous_end = 0u32;
    let mut endpoint_index = 0usize;
    for range in ranges {
        if range.start > range.end || range.start < previous_end {
            return Err("reversed or overlapping UTF-16 range");
        }
        previous_end = range.end;
    }
    let endpoint_count = ranges.len().saturating_mul(2);
    let endpoint = |index: usize| {
        let range = &ranges[index / 2];
        if index.is_multiple_of(2) {
            range.start
        } else {
            range.end
        }
    };
    let mut utf16 = 0u32;
    for ch in text.chars() {
        while endpoint_index < endpoint_count && endpoint(endpoint_index) == utf16 {
            endpoint_index += 1;
        }
        if endpoint_index < endpoint_count && endpoint(endpoint_index) < utf16 {
            return Err("UTF-16 range is not on a scalar boundary");
        }
        utf16 = utf16.saturating_add(ch.len_utf16() as u32);
    }
    while endpoint_index < endpoint_count && endpoint(endpoint_index) == utf16 {
        endpoint_index += 1;
    }
    if endpoint_index == endpoint_count {
        Ok(())
    } else {
        Err("UTF-16 range outside cell text or inside surrogate pair")
    }
}

#[cfg(test)]
mod tests {
    use super::highlighted_parts;
    use deve_core::source_control::diff_projection::DiffTextRange;

    #[test]
    fn converts_emoji_and_cjk_utf16_ranges_without_splitting_utf8() {
        let parts = highlighted_parts(
            "a\u{1f600}\u{4e2d}\u{6587}z",
            &[DiffTextRange { start: 1, end: 5 }],
        )
        .unwrap();
        assert_eq!(
            parts,
            vec![
                ("a".into(), false),
                ("\u{1f600}\u{4e2d}\u{6587}".into(), true),
                ("z".into(), false)
            ]
        );
    }

    #[test]
    fn rejects_range_inside_surrogate_pair() {
        assert!(highlighted_parts("\u{1f600}", &[DiffTextRange { start: 1, end: 2 }]).is_err());
    }
}
