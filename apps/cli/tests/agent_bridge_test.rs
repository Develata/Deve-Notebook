// apps/cli/tests/agent_bridge_test.rs
//! Agent Bridge 子进程管道测试
//!
//! 使用 `echo` 模拟外部 CLI，验证 stdout 管道捕获逻辑。

use tokio::io::AsyncBufReadExt;

#[tokio::test]
async fn test_subprocess_stdout_pipe() -> anyhow::Result<()> {
    let (cmd, args) = if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", "echo hello agent bridge"])
    } else {
        ("sh", vec!["-c", "echo hello agent bridge"])
    };

    let mut child = tokio::process::Command::new(cmd)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("stdout");
    let mut reader = tokio::io::BufReader::new(stdout);

    let mut line = String::new();
    let n = reader.read_line(&mut line).await?;
    assert!(n > 0, "Should read at least one line");
    assert!(
        line.contains("hello agent bridge"),
        "Output should contain echo text, got: {}",
        line
    );

    let status = child.wait().await?;
    assert!(status.success());
    Ok(())
}

#[tokio::test]
async fn test_subprocess_not_found_returns_error() {
    let result = tokio::process::Command::new("__nonexistent_cli_tool__")
        .arg("test")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .spawn();

    assert!(result.is_err(), "Spawning nonexistent CLI should fail");
}
