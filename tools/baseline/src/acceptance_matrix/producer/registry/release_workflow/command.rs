//! Canonical candidate producer command parsing and shell-settlement checks.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;

pub(super) fn parse_candidate_command(
    run: &str,
    shell: Option<&str>,
    path: &str,
) -> Result<Option<Vec<String>>> {
    let command_count = run.match_indices("acceptance-run").count();
    if command_count == 0 {
        return Ok(None);
    }
    if command_count != 1 {
        bail!("acceptance producers: {path} must execute exactly one acceptance-run command");
    }
    if !matches!(shell, None | Some("pwsh")) {
        bail!("acceptance producers: {path} uses an uncontrolled shell");
    }
    let commands = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let [command] = commands.as_slice() else {
        bail!(
            "acceptance producers: {path} producer must occupy one dedicated canonical run command"
        );
    };
    parse_argv(command, path)
}

fn parse_argv(command: &str, path: &str) -> Result<Option<Vec<String>>> {
    let tokens = tokenize_command(command, path)?
        .into_iter()
        .filter(|token| !matches!(token.as_str(), "\\" | "`"))
        .collect::<Vec<_>>();
    let mut index = 0usize;
    if tokens.first().map(String::as_str) == Some("cargo") {
        for expected in ["cargo", "run", "--locked"] {
            expect_token(&tokens, &mut index, expected, path)?;
        }
        if tokens.get(index).map(String::as_str) == Some("--quiet") {
            index += 1;
        }
        for expected in ["-p", "deve_baseline", "--", "acceptance-run", "--tier"] {
            expect_token(&tokens, &mut index, expected, path)?;
        }
    } else {
        for expected in [
            "$GITHUB_WORKSPACE/target/android-candidate-harness/debug/deve_baseline",
            "acceptance-run",
            "--tier",
        ] {
            expect_token(&tokens, &mut index, expected, path)?;
        }
    }
    let tier = tokens
        .get(index)
        .with_context(|| format!("acceptance producers: {path} is missing its tier"))?;
    if !matches!(tier.as_str(), "full" | "tag-ready" | "target-host") {
        bail!("acceptance producers: {path} has invalid candidate tier {tier}");
    }
    index += 1;
    if tokens.get(index).map(String::as_str) == Some("--plan") {
        bail!("acceptance producers: {path} must execute receipts, not plan them");
    }
    let mut producers = Vec::new();
    while tokens.get(index).map(String::as_str) == Some("--producer") {
        index += 1;
        let producer = tokens.get(index).with_context(|| {
            format!("acceptance producers: {path} has --producer without an ID")
        })?;
        producers.push(producer.to_owned());
        index += 1;
    }
    if producers.is_empty() {
        bail!(
            "acceptance producers: {path} candidate acceptance-run must bind at least one --producer"
        );
    }
    expect_token(&tokens, &mut index, "--receipt-dir", path)?;
    let receipt_dir = tokens.get(index).with_context(|| {
        format!("acceptance producers: {path} is missing its receipt directory")
    })?;
    if !valid_receipt_dir(receipt_dir) {
        bail!("acceptance producers: {path} has an invalid receipt directory");
    }
    index += 1;
    if index != tokens.len() {
        bail!("acceptance producers: {path} mixes the candidate command with trailing shell text");
    }
    let mut unique = BTreeSet::new();
    for producer in &producers {
        if !valid_id(producer) {
            bail!("acceptance producers: {path} has invalid producer ID {producer}");
        }
        if !unique.insert(producer.as_str()) {
            bail!("acceptance producers: {path} repeats producer {producer}");
        }
    }
    Ok(Some(producers))
}

fn tokenize_command(command: &str, path: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in command.chars() {
        if let Some(expected) = quote {
            if ch == expected {
                quote = None;
            } else {
                current.push(ch);
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if quote.is_some() {
        bail!("acceptance producers: {path} has an unterminated quoted argument");
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn expect_token(tokens: &[String], index: &mut usize, expected: &str, path: &str) -> Result<()> {
    if tokens.get(*index).map(String::as_str) != Some(expected) {
        bail!(
            "acceptance producers: {path} must use canonical candidate argv; expected {expected} at token {}",
            *index
        );
    }
    *index += 1;
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
}

fn valid_receipt_dir(value: &str) -> bool {
    let suffix = value
        .strip_prefix("$RUNNER_TEMP/")
        .or_else(|| value.strip_prefix("${{ runner.temp }}/"))
        .or_else(|| value.strip_prefix("/tmp/"));
    suffix.is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    })
}
