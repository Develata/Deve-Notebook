//! plan_ref:
//!   - 16_ai_agent#trusted-agent-bridge
//!
use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tokio::process::ChildStdout;
use tokio::time::Instant;

use super::process_tree::{ContainedChild, trusted_cli_command};

const MAX_AGENT_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
enum BoundedLine {
    Eof,
    Line(usize),
    OutputLimit,
}

pub(super) async fn spawn_and_stream(
    cli_path: &str,
    query: &str,
    timeout_ms: u64,
    ch: &DualChannel,
    req_id: &str,
) -> anyhow::Result<()> {
    let mut command = trusted_cli_command(cli_path, query);
    let mut child = ContainedChild::spawn(&mut command).map_err(|err| {
            anyhow::anyhow!(
                "Failed to spawn '{}': {}. Check AGENT_CLI_PATH points to an existing absolute executable path.",
                cli_path,
                err
            )
        })?;

    let stdout = child
        .take_stdout()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
    stream_child(child, stdout, timeout_ms, ch, req_id).await
}

async fn stream_child(
    mut child: ContainedChild,
    stdout: ChildStdout,
    timeout_ms: u64,
    ch: &DualChannel,
    req_id: &str,
) -> anyhow::Result<()> {
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut line = Vec::with_capacity(MAX_AGENT_OUTPUT_BYTES);
    let deadline_at = Instant::now() + Duration::from_millis(timeout_ms.max(1));
    let deadline = tokio::time::sleep_until(deadline_at);
    tokio::pin!(deadline);
    let mut output_bytes = 0usize;

    loop {
        tokio::select! {
            _ = &mut deadline => {
                return fail_after_cleanup(
                    anyhow::anyhow!("Agent CLI timeout"),
                    &mut child,
                ).await;
            }
            read = read_line_bounded(&mut reader, &mut line) => {
                match read {
                    Ok(BoundedLine::Eof) => break,
                    Ok(BoundedLine::OutputLimit) => {
                        return fail_after_cleanup(
                            anyhow::anyhow!("Agent CLI output limit exceeded"),
                            &mut child,
                        ).await;
                    }
                    Ok(BoundedLine::Line(raw_bytes)) => {
                        if output_bytes
                            .checked_add(raw_bytes)
                            .is_none_or(|total| total > MAX_AGENT_OUTPUT_BYTES)
                        {
                            return fail_after_cleanup(
                                anyhow::anyhow!("Agent CLI output limit exceeded"),
                                &mut child,
                            )
                            .await;
                        }
                        output_bytes += raw_bytes;
                        let text = match std::str::from_utf8(&line) {
                            Ok(text) => text,
                            Err(err) => {
                                let primary =
                                    anyhow::anyhow!("Agent CLI stdout read error: {}", err);
                                return fail_after_cleanup(primary, &mut child).await;
                            }
                        };
                        let clean = strip_ansi(text);
                        if !clean.trim().is_empty() {
                            ch.unicast(ServerMessage::ChatChunk {
                                req_id: req_id.to_string(),
                                delta: Some(clean),
                                finish_reason: None,
                            });
                        }
                    }
                    Err(err) => {
                        let primary = anyhow::anyhow!("Agent CLI stdout read error: {}", err);
                        return fail_after_cleanup(primary, &mut child).await;
                    }
                }
            }
        }
    }

    let status = match tokio::time::timeout_at(deadline_at, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(err)) => {
            let primary = anyhow::anyhow!("Agent CLI wait error: {}", err);
            return fail_after_cleanup(primary, &mut child).await;
        }
        Err(_) => {
            return fail_after_cleanup(anyhow::anyhow!("Agent CLI timeout"), &mut child).await;
        }
    };
    if !status.success() {
        let primary = anyhow::anyhow!("Agent CLI exited with status: {}", status);
        return match child.retire_tree_after_wait() {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(primary.context(format!("Agent CLI cleanup failed: {cleanup}"))),
        };
    }
    child
        .retire_tree_after_wait()
        .map_err(|cleanup| anyhow::anyhow!("Agent CLI cleanup failed: {cleanup}"))?;

    ch.unicast(ServerMessage::ChatChunk {
        req_id: req_id.to_string(),
        delta: None,
        finish_reason: Some("stop".to_string()),
    });
    Ok(())
}

async fn read_line_bounded<R>(reader: &mut R, line: &mut Vec<u8>) -> io::Result<BoundedLine>
where
    R: AsyncBufRead + Unpin,
{
    line.clear();
    loop {
        let (take, terminated) = {
            let buffer = reader.fill_buf().await?;
            if buffer.is_empty() {
                return Ok(if line.is_empty() {
                    BoundedLine::Eof
                } else {
                    BoundedLine::Line(line.len())
                });
            }

            let newline = buffer.iter().position(|byte| *byte == b'\n');
            let take = newline.map_or(buffer.len(), |index| index + 1);
            let Some(new_len) = line.len().checked_add(take) else {
                return Ok(BoundedLine::OutputLimit);
            };
            if new_len > MAX_AGENT_OUTPUT_BYTES {
                return Ok(BoundedLine::OutputLimit);
            }
            line.extend_from_slice(&buffer[..take]);
            (take, newline.is_some())
        };
        reader.consume(take);
        if terminated {
            return Ok(BoundedLine::Line(line.len()));
        }
    }
}

async fn fail_after_cleanup(
    primary: anyhow::Error,
    child: &mut ContainedChild,
) -> anyhow::Result<()> {
    match child.retire_tree().await {
        Ok(()) => Err(primary),
        Err(cleanup) => Err(primary.context(format!("Agent CLI cleanup failed: {cleanup}"))),
    }
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.next() == Some('[') {
            for esc in chars.by_ref() {
                if esc.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
