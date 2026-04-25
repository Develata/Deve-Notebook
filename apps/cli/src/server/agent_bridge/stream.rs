//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;
use std::time::Duration;

const MAX_AGENT_OUTPUT_BYTES: usize = 64 * 1024;

pub(super) async fn spawn_and_stream(
    cli_path: &str,
    query: &str,
    timeout_ms: u64,
    ch: &DualChannel,
    req_id: &str,
) -> anyhow::Result<()> {
    use std::process::Stdio;
    use tokio::io::AsyncBufReadExt;

    let mut child = tokio::process::Command::new(cli_path)
        .env_clear()
        .args(["run", query])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            anyhow::anyhow!(
                "Failed to spawn '{}': {}. Is it installed and in PATH?",
                cli_path,
                err
            )
        })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut line = String::new();
    let deadline = tokio::time::sleep(Duration::from_millis(timeout_ms.max(1)));
    tokio::pin!(deadline);
    let mut output_bytes = 0usize;

    loop {
        line.clear();
        tokio::select! {
            _ = &mut deadline => {
                let _ = child.kill().await;
                return Err(anyhow::anyhow!("Agent CLI timeout"));
            }
            read = reader.read_line(&mut line) => {
                match read {
                    Ok(0) => break,
                    Ok(_) => {
                        let clean = strip_ansi(&line);
                        output_bytes = output_bytes.saturating_add(clean.len());
                        if output_bytes > MAX_AGENT_OUTPUT_BYTES {
                            let _ = child.kill().await;
                            return Err(anyhow::anyhow!("Agent CLI output limit exceeded"));
                        }
                        if !clean.trim().is_empty() {
                            ch.unicast(ServerMessage::ChatChunk {
                                req_id: req_id.to_string(),
                                delta: Some(clean),
                                finish_reason: None,
                            });
                        }
                    }
                    Err(err) => {
                        tracing::warn!("Agent stdout read error: {:?}", err);
                        break;
                    }
                }
            }
        }
    }

    ch.unicast(ServerMessage::ChatChunk {
        req_id: req_id.to_string(),
        delta: None,
        finish_reason: Some("stop".to_string()),
    });

    let status = child.wait().await?;
    if !status.success() {
        tracing::warn!("Agent CLI exited with status: {}", status);
    }
    Ok(())
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
