//! plan_ref: infra

use anyhow::{Context, Result, bail};

pub(super) fn require_text(content: &str, expected: &str, scope: &str) -> Result<()> {
    if content.contains(expected) {
        Ok(())
    } else {
        bail!("release-baseline-check: missing '{expected}' in {scope}")
    }
}

pub(super) fn require_ordered_text(content: &str, expected: &[&str], scope: &str) -> Result<()> {
    let mut cursor = 0;
    for needle in expected {
        let Some(offset) = content[cursor..].find(needle) else {
            bail!("release-baseline-check: missing ordered '{needle}' in {scope}");
        };
        cursor += offset + needle.len();
    }
    Ok(())
}

pub(super) fn require_exact_mapping_keys(
    content: &str,
    indent: usize,
    expected: &[&str],
    scope: &str,
) -> Result<()> {
    let mut actual = content
        .lines()
        .filter(|line| leading_spaces(line) == indent)
        .filter_map(|line| line.trim().split_once(':').map(|(key, _)| key.to_owned()))
        .collect::<Vec<_>>();
    actual.sort();
    let mut expected = expected
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    expected.sort();
    if actual != expected {
        bail!(
            "release-baseline-check: expected exact keys {:?} in {scope}, found {:?}",
            expected,
            actual
        );
    }
    Ok(())
}

pub(super) fn has_v_tag_trigger(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(suffix) = trimmed.strip_prefix("tags:") else {
            continue;
        };
        if suffix.contains("v*") {
            return true;
        }
        let indent = leading_spaces(line);
        for nested in &lines[index + 1..] {
            if nested.trim().is_empty() || nested.trim_start().starts_with('#') {
                continue;
            }
            if leading_spaces(nested) <= indent {
                break;
            }
            if nested.contains("v*") {
                return true;
            }
        }
    }
    false
}

pub(super) fn yaml_mapping_block(content: &str, indent: usize, key: &str) -> Result<String> {
    let marker = format!("{}{key}:", " ".repeat(indent));
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == marker)
        .with_context(|| format!("release-baseline-check: missing YAML block {key}"))?;
    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            !line.trim().is_empty()
                && !line.trim_start().starts_with('#')
                && leading_spaces(line) <= indent
        })
        .map_or(lines.len(), |offset| start + 1 + offset);
    Ok(lines[start..end].join("\n"))
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

#[cfg(test)]
mod tests {
    use super::{
        has_v_tag_trigger, require_exact_mapping_keys, require_ordered_text, yaml_mapping_block,
    };

    #[test]
    fn detects_inline_and_multiline_v_tag_triggers() {
        assert!(has_v_tag_trigger("on:\n  push:\n    tags: ['v*']\n"));
        assert!(has_v_tag_trigger("on:\n  push:\n    tags:\n      - 'v*'\n"));
        assert!(!has_v_tag_trigger("on:\n  push:\n    branches: [main]\n"));
    }

    #[test]
    fn yaml_mapping_block_stops_at_sibling_job() {
        let workflow = "jobs:\n  first:\n    needs: test\n  second:\n    runs-on: ubuntu-latest\n";
        let block = yaml_mapping_block(workflow, 2, "first").expect("first job");
        assert!(block.contains("needs: test"));
        assert!(!block.contains("second"));
    }

    #[test]
    fn ordered_text_rejects_reversed_publish_steps() {
        assert!(
            require_ordered_text(
                "draft\nverify\npublish",
                &["draft", "verify", "publish"],
                "fixture"
            )
            .is_ok()
        );
        assert!(
            require_ordered_text(
                "publish\nverify\ndraft",
                &["draft", "verify", "publish"],
                "fixture"
            )
            .is_err()
        );
    }

    #[test]
    fn exact_mapping_keys_rejects_extra_secret() {
        let expected = ["FIRST", "SECOND"];
        let exact = "    secrets:\n      FIRST: one\n      SECOND: two\n";
        let extra = "    secrets:\n      FIRST: one\n      SECOND: two\n      THIRD: three\n";
        assert!(require_exact_mapping_keys(exact, 6, &expected, "fixture").is_ok());
        assert!(require_exact_mapping_keys(extra, 6, &expected, "fixture").is_err());
    }
}
