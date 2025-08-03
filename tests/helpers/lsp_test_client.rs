use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::time::timeout;
use tower_lsp::lsp_types::*;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static BINARY_BUILT: AtomicBool = AtomicBool::new(false);

pub struct LspTestClient {
    process: tokio::process::Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    pid: Option<u32>,
    default_timeout_ms: u64,
}

impl LspTestClient {
    pub async fn spawn() -> Result<Self> {
        Self::spawn_with_timeout(1000).await
    }

    pub async fn spawn_with_timeout(default_timeout_ms: u64) -> Result<Self> {
        // Build caching (Task 7): Only build once per test run
        if !BINARY_BUILT.load(Ordering::Acquire) {
            let build_output = std::process::Command::new("cargo")
                .args(&["build", "--bin", "gren-lsp"])
                .output()
                .map_err(|e| anyhow!("Failed to build LSP server: {}", e))?;

            if !build_output.status.success() {
                return Err(anyhow!(
                    "Failed to build LSP server: {}",
                    String::from_utf8_lossy(&build_output.stderr)
                ));
            }
            
            BINARY_BUILT.store(true, Ordering::Release);
        }

        // Now spawn the built binary directly
        let mut cmd = Command::new("./target/debug/gren-lsp");
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()); // Changed from null to piped for debugging

        let mut process = tokio::process::Command::from(cmd)
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn LSP server: {}", e))?;

        let pid = process.id();

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stdin"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stdout"))?;

        Ok(Self {
            process,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
            pid,
            default_timeout_ms,
        })
    }

    pub async fn send_request_with_timeout<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &mut self,
        method: &str,
        params: T,
        timeout_ms: u64,
    ) -> Result<R> {
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        self.send_message(&message).await?;

        // Keep reading messages until we find the response with our ID
        let response = timeout(
            Duration::from_millis(timeout_ms), 
            self.read_response_with_id(id)
        )
        .await
        .map_err(|_| anyhow!("Request timed out after {}ms", timeout_ms))??;

        if let Some(error) = response.get("error") {
            return Err(anyhow!("LSP error: {}", error));
        }

        let result = response
            .get("result")
            .ok_or_else(|| anyhow!("No result in response"))?;

        serde_json::from_value(result.clone())
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))
    }

    pub async fn send_notification<T: serde::Serialize>(
        &mut self,
        method: &str,
        params: T,
    ) -> Result<()> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        self.send_message(&message).await
    }

    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(Url::parse("file:///tmp/test-workspace").unwrap()),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    hover: Some(HoverClientCapabilities::default()),
                    completion: Some(CompletionClientCapabilities::default()),
                    definition: Some(GotoCapability::default()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let result: InitializeResult = self
            .send_request_with_timeout("initialize", params, 1000)
            .await?;

        // Send initialized notification
        self.send_notification("initialized", InitializedParams {})
            .await?;

        Ok(result)
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        // Send shutdown request without params field entirely
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "shutdown"
        });

        self.send_message(&message).await?;

        let response = timeout(
            Duration::from_millis(1000), 
            self.read_response_with_id(id)
        )
        .await
        .map_err(|_| anyhow!("Shutdown request timed out after 1000ms"))??;

        if let Some(error) = response.get("error") {
            return Err(anyhow!("LSP error during shutdown: {}", error));
        }

        // Send exit notification without params
        let exit_message = json!({
            "jsonrpc": "2.0",
            "method": "exit"
        });
        self.send_message(&exit_message).await?;

        // Wait a bit for the process to exit - don't fail if it takes time
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        Ok(())
    }

    async fn send_message(&mut self, message: &Value) -> Result<()> {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        
        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(content.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    async fn read_response(&mut self) -> Result<Value> {
        // Read the Content-Length header
        let mut header_line = String::new();
        self.stdout.read_line(&mut header_line).await?;

        if !header_line.starts_with("Content-Length:") {
            return Err(anyhow!("Invalid header: {}", header_line));
        }

        let content_length: usize = header_line
            .trim()
            .strip_prefix("Content-Length:")
            .unwrap()
            .trim()
            .parse()
            .map_err(|e| anyhow!("Invalid content length: {}", e))?;

        // Read the empty line
        let mut empty_line = String::new();
        self.stdout.read_line(&mut empty_line).await?;

        // Read the message content
        let mut buffer = vec![0u8; content_length];
        self.stdout.read_exact(&mut buffer).await?;

        let content = String::from_utf8(buffer)?;
        serde_json::from_str(&content).map_err(|e| anyhow!("Invalid JSON: {}", e))
    }

    async fn read_response_with_id(&mut self, expected_id: u64) -> Result<Value> {
        loop {
            let message = self.read_response().await?;
            
            // If this message has an ID and it matches ours, return it
            if let Some(id) = message.get("id") {
                if let Some(id_value) = id.as_u64() {
                    if id_value == expected_id {
                        return Ok(message);
                    }
                }
            }
            
            // Otherwise, this is a notification or response to another request
            // Continue reading (silently ignore notifications)
        }
    }

    /// Send a raw protocol message string (for error testing)
    pub async fn send_raw_message(&mut self, raw_message: &str) -> Result<()> {
        self.stdin.write_all(raw_message.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Send a JSON message without validation (for error testing)
    pub async fn send_raw_json_message(&mut self, message: &Value) -> Result<Value> {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        
        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(content.as_bytes()).await?;
        self.stdin.flush().await?;

        // Try to read a response - may fail for invalid messages
        self.read_response().await
    }

    /// Send a raw request and wait for response (for error testing)
    pub async fn send_request_raw(&mut self, message: &Value, timeout_ms: u64) -> Result<Value> {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        
        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(content.as_bytes()).await?;
        self.stdin.flush().await?;

        // Extract ID from message for response correlation
        let expected_id = message.get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Message missing valid id field"))?;

        timeout(
            Duration::from_millis(timeout_ms),
            self.read_response_with_id(expected_id)
        )
        .await
        .map_err(|_| anyhow!("Request timed out after {}ms", timeout_ms))?
    }

    /// Check if server is still responsive by sending a simple request
    pub async fn is_server_responsive(&mut self) -> Result<bool> {
        // Try a simple, valid request that should work
        match timeout(
            Duration::from_millis(1000),
            self.send_request_with_timeout::<_, Value>("textDocument/hover", json!({
                "textDocument": {"uri": "file:///test.gren"},
                "position": {"line": 0, "character": 0}
            }), 500)
        ).await {
            Ok(Ok(_)) => Ok(true),        // Got successful response
            Ok(Err(_)) => Ok(true),       // Got error response (but server is responsive)
            Err(_) => Ok(false),          // Timeout means not responsive
        }
    }

    /// Set default timeout for operations (Task 7: Framework Robustness)
    pub fn set_default_timeout(&mut self, timeout_ms: u64) {
        self.default_timeout_ms = timeout_ms;
    }

    /// Get the process ID (Task 3: Process Lifecycle)
    pub fn get_pid(&self) -> Option<u32> {
        self.pid
    }

    /// Verify process has actually exited (Task 3: Process Lifecycle)
    pub async fn verify_process_exit(&mut self, timeout_duration: Duration) -> Result<()> {
        // Wait for the process to exit
        let exit_status = timeout(timeout_duration, self.process.wait())
            .await
            .map_err(|_| anyhow!("Process did not exit within {:?}", timeout_duration))?
            .map_err(|e| anyhow!("Failed to wait for process: {}", e))?;

        // Check exit status
        if !exit_status.success() {
            return Err(anyhow!("Process exited with non-zero status: {}", exit_status));
        }

        // On Unix systems, verify process is not a zombie
        #[cfg(unix)]
        {
            if let Some(pid) = self.pid {
                use std::fs;
                let stat_path = format!("/proc/{}/stat", pid);
                
                // If the stat file still exists, check if it's a zombie
                if let Ok(stat_content) = fs::read_to_string(&stat_path) {
                    let fields: Vec<&str> = stat_content.split_whitespace().collect();
                    if fields.len() > 2 && fields[2] == "Z" {
                        return Err(anyhow!("Process {} is a zombie", pid));
                    }
                }
            }
        }

        Ok(())
    }

    /// Force kill the process if still running (Task 8: Test Isolation)
    pub async fn force_cleanup(&mut self) -> Result<()> {
        // Try graceful shutdown first
        if let Err(_) = self.process.try_wait() {
            // Process still running, force kill
            self.process.kill().await.map_err(|e| anyhow!("Failed to kill process: {}", e))?;
            
            // Wait for termination
            let _ = timeout(Duration::from_secs(2), self.process.wait()).await;
        }
        Ok(())
    }

    /// Get stderr output for debugging (Task 7: Framework Robustness)
    pub async fn get_stderr(&mut self) -> Result<String> {
        if let Some(mut stderr) = self.process.stderr.take() {
            let mut output = Vec::new();
            
            // Try to read available stderr data with timeout
            match timeout(Duration::from_millis(100), stderr.read_to_end(&mut output)).await {
                Ok(Ok(_)) => {
                    return Ok(String::from_utf8_lossy(&output).to_string());
                },
                _ => {
                    // Put stderr back
                    self.process.stderr = Some(stderr);
                }
            }
        }
        Ok(String::new())
    }

    /// Check if process is still running
    pub fn is_process_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Send multiple requests concurrently (for Task 6: Concurrent Testing)
    pub async fn send_concurrent_requests(&mut self, requests: Vec<Value>) -> Result<Vec<Value>> {
        let mut request_ids = Vec::new();
        
        // Send all requests
        for request in &requests {
            self.send_message(request).await?;
            if let Some(id) = request.get("id").and_then(|v| v.as_u64()) {
                request_ids.push(id);
            }
        }

        // Collect responses
        let mut responses = Vec::new();
        for id in request_ids {
            let response = timeout(
                Duration::from_millis(self.default_timeout_ms),
                self.read_response_with_id(id)
            )
            .await
            .map_err(|_| anyhow!("Request {} timed out after {}ms", id, self.default_timeout_ms))??;
            responses.push(response);
        }

        Ok(responses)
    }

    /// Test-only method: Send a custom test request for document state inspection
    pub async fn send_test_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": format!("test/{}", method),
            "params": params
        });

        self.send_message(&message).await?;

        let response = timeout(
            Duration::from_millis(self.default_timeout_ms), 
            self.read_response_with_id(id)
        )
        .await
        .map_err(|_| anyhow!("Test request timed out after {}ms", self.default_timeout_ms))??;

        if let Some(error) = response.get("error") {
            return Err(anyhow!("LSP error: {}", error));
        }

        response
            .get("result")
            .ok_or_else(|| anyhow!("No result in response"))
            .map(|v| v.clone())
    }
}

/// Task 8: Test Isolation - Ensure cleanup on drop
impl Drop for LspTestClient {
    fn drop(&mut self) {
        // Force kill the process if it's still running
        if self.is_process_running() {
            let _ = self.process.start_kill();
        }
    }
}