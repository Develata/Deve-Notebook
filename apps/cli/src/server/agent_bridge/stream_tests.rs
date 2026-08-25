//! plan_ref:
//!   - 16_ai_agent#trusted-agent-bridge
//!
#[cfg(unix)]
use super::{super::process_tree::ContainedChild, stream_child};
use super::{BoundedLine, MAX_AGENT_OUTPUT_BYTES, read_line_bounded};
#[cfg(unix)]
use crate::server::channel::DualChannel;
#[cfg(unix)]
use std::process::Stdio;
#[cfg(unix)]
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
#[cfg(unix)]
use tokio::process::Command;
#[cfg(unix)]
use tokio::sync::{broadcast, mpsc};

#[tokio::test]
async fn no_newline_output_limit_uses_raw_bytes_before_utf8() -> anyhow::Result<()> {
    let (mut writer, reader) = tokio::io::duplex(1024);
    let writer_task = tokio::spawn(async move {
        let payload = vec![0xff; MAX_AGENT_OUTPUT_BYTES + 1];
        writer.write_all(&payload).await
    });

    let mut reader = tokio::io::BufReader::new(reader);
    let mut line = Vec::with_capacity(MAX_AGENT_OUTPUT_BYTES);
    let result = read_line_bounded(&mut reader, &mut line).await?;

    assert_eq!(result, BoundedLine::OutputLimit);
    assert!(line.len() <= MAX_AGENT_OUTPUT_BYTES);
    writer_task.abort();
    let _ = writer_task.await;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn stdout_eof_does_not_bypass_overall_deadline() -> anyhow::Result<()> {
    let mut command = closing_stdout_then_sleep_command();
    command.stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = ContainedChild::spawn(&mut command)?;
    let stdout = child
        .take_stdout()
        .ok_or_else(|| anyhow::anyhow!("test child stdout was not piped"))?;

    let (broadcast_tx, _) = broadcast::channel(8);
    let (unicast_tx, _) = mpsc::channel(8);
    let channel = DualChannel::new(broadcast_tx, unicast_tx);
    let started = Instant::now();

    let error = stream_child(child, stdout, 50, &channel, "req-eof")
        .await
        .expect_err("child must outlive the overall deadline");

    assert!(error.to_string().contains("Agent CLI timeout"));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "cleanup exceeded its bound: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[cfg(unix)]
fn closing_stdout_then_sleep_command() -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", "exec 1>&-; exec sleep 5"]);
    command
}
