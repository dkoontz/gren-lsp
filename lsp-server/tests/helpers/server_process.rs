use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub struct ServerProcess {
    process: tokio::process::Child,
    pid: u32,
    start_time: Instant,
}

impl ServerProcess {
    pub async fn spawn() -> Result<Self> {
        // Ensure binary is built
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

        let mut cmd = Command::new("./target/debug/gren-lsp");
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let process = tokio::process::Command::from(cmd)
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn LSP server: {}", e))?;

        let pid = process.id().ok_or_else(|| anyhow!("Failed to get process ID"))?;

        Ok(Self {
            process,
            pid,
            start_time: Instant::now(),
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub async fn terminate(&mut self) -> Result<()> {
        self.process.kill().await.map_err(|e| anyhow!("Failed to kill process: {}", e))
    }

    pub async fn wait_for_exit(&mut self, timeout_duration: Duration) -> Result<std::process::ExitStatus> {
        timeout(timeout_duration, self.process.wait())
            .await
            .map_err(|_| anyhow!("Process did not exit within timeout"))?
            .map_err(|e| anyhow!("Failed to wait for process: {}", e))
    }

    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    pub async fn verify_clean_exit(&mut self, timeout_duration: Duration) -> Result<()> {
        let exit_status = self.wait_for_exit(timeout_duration).await?;
        
        if !exit_status.success() {
            return Err(anyhow!("Process exited with non-zero status: {}", exit_status));
        }

        // Verify process is actually gone
        if self.is_running() {
            return Err(anyhow!("Process still running after reported exit"));
        }

        Ok(())
    }

    /// Check if process has become a zombie
    pub fn is_zombie(&self) -> Result<bool> {
        #[cfg(unix)]
        {
            use std::fs;
            let stat_path = format!("/proc/{}/stat", self.pid);
            if let Ok(stat_content) = fs::read_to_string(&stat_path) {
                let fields: Vec<&str> = stat_content.split_whitespace().collect();
                if fields.len() > 2 {
                    // Third field is process state
                    return Ok(fields[2] == "Z");
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, assume not zombie
            return Ok(false);
        }
        
        Ok(false)
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        // Ensure process is cleaned up
        let _ = self.process.start_kill();
    }
}