use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, info, warn};

/// Represents the Gren compiler integration layer
pub struct GrenCompiler {
    /// Path to the gren executable
    gren_path: PathBuf,
    /// Working directory for compilation
    working_dir: PathBuf,
    /// Cache of compilation results
    cache: std::collections::HashMap<PathBuf, CompilationResult>,
}

/// Result of a compilation attempt
#[derive(Debug, Clone)]
pub struct CompilationResult {
    /// Whether compilation succeeded
    pub success: bool,
    /// Compilation errors and warnings
    pub diagnostics: Vec<CompilerDiagnostic>,
    /// Compilation timestamp
    pub timestamp: SystemTime,
    /// Hash of the source content compiled
    pub content_hash: u64,
}

/// A diagnostic message from the Gren compiler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerDiagnostic {
    /// Type of diagnostic (error, warning, etc.)
    pub severity: DiagnosticSeverity,
    /// Error message title
    pub title: String,
    /// Detailed error message
    pub message: String,
    /// File path where error occurred
    pub path: Option<PathBuf>,
    /// Line and column information
    pub location: Option<DiagnosticLocation>,
}

/// Severity levels for compiler diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// Location information for a diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticLocation {
    pub line: u32,
    pub column: u32,
    pub end_line: Option<u32>,
    pub end_column: Option<u32>,
}

/// JSON structure returned by gren make --report=json
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum GrenCompilerOutput {
    Error {
        path: Option<String>,
        title: String,
        message: serde_json::Value,
    },
    Warning {
        path: Option<String>,
        title: String,
        message: serde_json::Value,
    },
}

impl GrenCompiler {
    /// Create a new Gren compiler integration
    pub fn new(working_dir: PathBuf) -> Result<Self> {
        let gren_path = Self::find_gren_executable()?;
        
        info!("Found Gren compiler at: {}", gren_path.display());
        
        Ok(Self {
            gren_path,
            working_dir,
            cache: std::collections::HashMap::new(),
        })
    }

    /// Find the gren executable in PATH
    fn find_gren_executable() -> Result<PathBuf> {
        // Try common locations
        let candidates = vec![
            "gren",
            "/usr/local/bin/gren",
            "/opt/homebrew/bin/gren",
        ];

        for candidate in candidates {
            if let Ok(output) = Command::new(candidate)
                .arg("--help")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                if output.success() {
                    return Ok(PathBuf::from(candidate));
                }
            }
        }

        Err(anyhow!("Could not find gren executable in PATH"))
    }

    /// Compile a Gren file and return diagnostics
    pub async fn compile_file(&mut self, file_path: &Path) -> Result<CompilationResult> {
        let content_hash = self.calculate_content_hash(file_path)?;
        
        // Check cache first
        if let Some(cached) = self.cache.get(file_path) {
            if cached.content_hash == content_hash {
                debug!("Using cached compilation result for {}", file_path.display());
                return Ok(cached.clone());
            }
        }

        info!("Compiling {} with Gren compiler", file_path.display());

        let result = self.run_compiler(file_path).await?;
        
        // Cache the result
        self.cache.insert(file_path.to_path_buf(), result.clone());
        
        Ok(result)
    }

    /// Run the Gren compiler on a file
    async fn run_compiler(&self, file_path: &Path) -> Result<CompilationResult> {
        let mut cmd = AsyncCommand::new(&self.gren_path);
        cmd.arg("make")
            .arg(file_path)
            .arg("--output=/dev/null")
            .arg("--report=json")
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Running compiler command: {:?}", cmd);

        let output = cmd.output().await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        debug!("Compiler stdout: {}", stdout);
        debug!("Compiler stderr: {}", stderr);

        let success = output.status.success();
        let diagnostics = self.parse_compiler_output(&stderr)?;

        Ok(CompilationResult {
            success,
            diagnostics,
            timestamp: SystemTime::now(),
            content_hash: self.calculate_content_hash(file_path)?,
        })
    }

    /// Parse JSON output from Gren compiler
    fn parse_compiler_output(&self, output: &str) -> Result<Vec<CompilerDiagnostic>> {
        let mut diagnostics = Vec::new();

        // Handle empty output
        if output.trim().is_empty() {
            return Ok(diagnostics);
        }

        // Try to parse as JSON
        match serde_json::from_str::<GrenCompilerOutput>(output) {
            Ok(compiler_output) => {
                let diagnostic = match compiler_output {
                    GrenCompilerOutput::Error { path, title, message } => {
                        CompilerDiagnostic {
                            severity: DiagnosticSeverity::Error,
                            title,
                            message: self.extract_message_text(message),
                            path: path.map(PathBuf::from),
                            location: None, // TODO: Extract location from message
                        }
                    }
                    GrenCompilerOutput::Warning { path, title, message } => {
                        CompilerDiagnostic {
                            severity: DiagnosticSeverity::Warning,
                            title,
                            message: self.extract_message_text(message),
                            path: path.map(PathBuf::from),
                            location: None, // TODO: Extract location from message
                        }
                    }
                };
                diagnostics.push(diagnostic);
            }
            Err(e) => {
                warn!("Failed to parse compiler output as JSON: {}", e);
                // Fallback: treat as plain text error
                if !output.trim().is_empty() {
                    diagnostics.push(CompilerDiagnostic {
                        severity: DiagnosticSeverity::Error,
                        title: "Compiler Error".to_string(),
                        message: output.to_string(),
                        path: None,
                        location: None,
                    });
                }
            }
        }

        Ok(diagnostics)
    }

    /// Extract readable text from JSON message value
    fn extract_message_text(&self, message: serde_json::Value) -> String {
        match message {
            serde_json::Value::String(s) => s,
            serde_json::Value::Array(arr) => {
                arr.into_iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Object(obj) => {
                            obj.get("string")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string()
                        }
                        _ => format!("{}", v),
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }
            _ => format!("{}", message),
        }
    }

    /// Calculate a hash of the file content for caching
    fn calculate_content_hash(&self, file_path: &Path) -> Result<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = std::fs::read_to_string(file_path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        file_path.hash(&mut hasher);
        Ok(hasher.finish())
    }

    /// Clear the compilation cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        info!("Cleared compilation cache");
    }

    /// Remove a specific file from the cache
    pub fn invalidate_cache(&mut self, file_path: &Path) {
        self.cache.remove(file_path);
        debug!("Invalidated cache for {}", file_path.display());
    }

    /// Check if the compiler is available
    pub fn is_available(&self) -> bool {
        Command::new(&self.gren_path)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Get compiler version information
    pub async fn get_version(&self) -> Result<String> {
        let output = AsyncCommand::new(&self.gren_path)
            .arg("--help")
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Extract version from first line
        if let Some(first_line) = stdout.lines().next() {
            if first_line.contains("Gren") {
                return Ok(first_line.to_string());
            }
        }

        Ok("Unknown version".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_compiler_creation() {
        let temp_dir = TempDir::new().unwrap();
        let compiler = GrenCompiler::new(temp_dir.path().to_path_buf());
        
        // Should succeed if gren is available, or fail gracefully
        match compiler {
            Ok(compiler) => {
                assert!(compiler.is_available());
            }
            Err(_) => {
                // Gren not available, which is fine for testing
            }
        }
    }

    #[test]
    fn test_message_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let compiler = GrenCompiler::new(temp_dir.path().to_path_buf()).unwrap_or_else(|_| {
            // Create a mock compiler for testing
            GrenCompiler {
                gren_path: PathBuf::from("gren"),
                working_dir: temp_dir.path().to_path_buf(),
                cache: std::collections::HashMap::new(),
            }
        });

        // Test string message
        let msg = serde_json::Value::String("Simple error".to_string());
        assert_eq!(compiler.extract_message_text(msg), "Simple error");

        // Test array message
        let msg = serde_json::json!([
            "Error on line 5: ",
            {"string": "undefined variable", "color": "RED"},
            " 'foo'"
        ]);
        let result = compiler.extract_message_text(msg);
        assert!(result.contains("Error on line 5"));
        assert!(result.contains("undefined variable"));
        assert!(result.contains("foo"));
    }
}