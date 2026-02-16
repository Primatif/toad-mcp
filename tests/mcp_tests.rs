use std::fs;
use std::process::Stdio;
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[tokio::test]
async fn test_mcp_full_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let config_dir = dir.path().join(".toad");
    fs::create_dir_all(&config_dir)?;

    let mut child = Command::new(env!("CARGO_BIN_EXE_toad-mcp"))
        .env("TOAD_CONFIG_DIR", &config_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // 1. Send initialize
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}"#;
    stdin.write_all(init_req.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // 2. Read initialize response
    let mut line = String::new();
    stdout.read_line(&mut line).await?;
    assert!(line.contains("toad-mcp"));

    // 3. Send initialized notification
    let init_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    stdin.write_all(init_notif.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // 4. Send tools/list
    let list_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    stdin.write_all(list_req.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // 5. Read tools/list response
    line.clear();
    stdout.read_line(&mut line).await?;
    assert!(line.contains("list_projects"));
    assert!(line.contains("get_project_detail"));
    assert!(line.contains("search_projects"));
    assert!(line.contains("get_ecosystem_summary"));
    assert!(line.contains("get_ecosystem_status"));
    assert!(line.contains("get_project_stats"));
    assert!(line.contains("get_atlas"));
    assert!(line.contains("get_manifest"));
    assert!(line.contains("get_project_context"));

    // 6. Shutdown
    drop(stdin);
    let _ = child.wait().await?;

    Ok(())
}

#[tokio::test]
async fn test_mcp_tool_call() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let config_dir = dir.path().join(".toad");
    fs::create_dir_all(&config_dir)?;

    let mut child = Command::new(env!("CARGO_BIN_EXE_toad-mcp"))
        .env("TOAD_CONFIG_DIR", &config_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Handshake
    stdin.write_all(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}"#.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;
    let mut line = String::new();
    stdout.read_line(&mut line).await?;

    stdin
        .write_all(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Call list_projects
    let call_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_projects","arguments":{}}}"#;
    stdin.write_all(call_req.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    line.clear();
    stdout.read_line(&mut line).await?;
    assert!(line.contains("result"));
    assert!(line.contains("content"));
    // Should be an empty list [] encoded in text
    assert!(line.contains("[]"));

    drop(stdin);
    let _ = child.wait().await?;

    Ok(())
}
