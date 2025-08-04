use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, error, info, warn};
use tempfile::TempDir;

/// Configuration for the Gren compiler interface
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Path to the Gren compiler executable
    pub compiler_path: PathBuf,
    /// Timeout for compilation operations in milliseconds
    pub timeout_ms: u64,
    /// Maximum number of concurrent compilations
    pub max_concurrent: usize,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            compiler_path: PathBuf::from("gren"),
            timeout_ms: 10000, // 10 seconds
            max_concurrent: 4,
        }
    }
}

/// Represents a compilation request
#[derive(Debug, Clone)]
pub struct CompileRequest {
    /// The module name to compile (e.g., "Main", "Utils.Parser")
    pub module_name: String,
    /// The project root directory containing gren.json
    pub project_root: PathBuf,
    /// Whether to include source maps in output
    pub include_sourcemaps: bool,
    /// In-memory documents to be written to temporary files
    /// Key: file path relative to project root, Value: file content
    pub in_memory_documents: HashMap<PathBuf, String>,
}

/// Represents the result of a compilation
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Whether compilation was successful
    pub success: bool,
    /// Compiler output (JSON format for errors, empty for success)
    pub output: String,
    /// Stderr output from compiler
    pub stderr: String,
    /// Exit code from compiler process
    pub exit_code: Option<i32>,
}

/// JSON structure for Gren compiler error output
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum CompilerOutput {
    #[serde(rename = "compile-errors")]
    CompileErrors { errors: Vec<CompileError> },
    
    #[serde(rename = "error")]
    GeneralError {
        title: String,
        path: String,
        message: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompileError {
    pub path: String,
    pub name: String,
    pub problems: Vec<Problem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Problem {
    pub title: String,
    pub region: Region,
    pub message: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Region {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Temporary workspace for compilation
struct TempWorkspace {
    /// Temporary directory handle (kept alive for cleanup)
    _temp_dir: TempDir,
    /// Path to the temporary workspace
    workspace_path: PathBuf,
}

impl TempWorkspace {
    /// Create a new temporary workspace
    async fn new(project_root: &Path, in_memory_documents: &HashMap<PathBuf, String>) -> Result<Self> {
        let temp_dir = TempDir::new()
            .map_err(|e| anyhow!("Failed to create temporary directory: {}", e))?;
        
        let workspace_path = temp_dir.path().to_path_buf();
        debug!("Created temporary workspace at {:?}", workspace_path);

        // Copy the project structure
        Self::copy_project_structure(project_root, &workspace_path).await?;
        
        // Write in-memory documents
        Self::write_in_memory_documents(&workspace_path, in_memory_documents).await?;

        Ok(Self {
            _temp_dir: temp_dir,
            workspace_path,
        })
    }

    /// Copy the project structure (gren.json and other necessary files)
    async fn copy_project_structure(source: &Path, dest: &Path) -> Result<()> {
        // Copy gren.json
        let source_gren_json = source.join("gren.json");
        let dest_gren_json = dest.join("gren.json");
        
        if let Ok(content) = fs::read(&source_gren_json).await {
            fs::write(&dest_gren_json, content).await
                .map_err(|e| anyhow!("Failed to write gren.json to temporary workspace: {}", e))?;
            debug!("Copied gren.json to temporary workspace");
        } else {
            return Err(anyhow!("Source project missing gren.json at {:?}", source_gren_json));
        }

        // Copy gren_packages directory if it exists (downloaded dependencies)
        let source_packages = source.join("gren_packages");
        let dest_packages = dest.join("gren_packages");
        if fs::metadata(&source_packages).await.is_ok() {
            Self::copy_directory_recursive(&source_packages, &dest_packages).await?;
            debug!("Copied gren_packages directory to temporary workspace");
        }

        // Copy .gren directory if it exists (compiler cache/build artifacts)
        let source_gren_dir = source.join(".gren");
        let dest_gren_dir = dest.join(".gren");
        if fs::metadata(&source_gren_dir).await.is_ok() {
            Self::copy_directory_recursive(&source_gren_dir, &dest_gren_dir).await?;
            debug!("Copied .gren directory to temporary workspace");
        }

        // Create src directory structure
        let src_dir = dest.join("src");
        fs::create_dir_all(&src_dir).await
            .map_err(|e| anyhow!("Failed to create src directory: {}", e))?;

        // Copy any existing .gren files that aren't in memory
        // This ensures we have the full project context
        if let Ok(source_src) = fs::read_dir(source.join("src")).await {
            Self::copy_source_files(source_src, &src_dir, source).await?;
        }

        Ok(())
    }

    /// Recursively copy source files
    fn copy_source_files<'a>(entries: fs::ReadDir, dest_dir: &'a Path, source_root: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut entries = entries;
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| anyhow!("Failed to read directory entry: {}", e))? {
                let entry_path = entry.path();
                
                if entry_path.is_dir() {
                    let dir_name = entry_path.file_name()
                        .ok_or_else(|| anyhow!("Invalid directory name"))?;
                    let dest_subdir = dest_dir.join(dir_name);
                    fs::create_dir_all(&dest_subdir).await
                        .map_err(|e| anyhow!("Failed to create subdirectory: {}", e))?;
                    
                    let sub_entries = fs::read_dir(&entry_path).await
                        .map_err(|e| anyhow!("Failed to read subdirectory: {}", e))?;
                    Self::copy_source_files(sub_entries, &dest_subdir, source_root).await?;
                } else if entry_path.extension().map_or(false, |ext| ext == "gren") {
                    if let Ok(content) = fs::read(&entry_path).await {
                        let relative_path = entry_path.strip_prefix(source_root)
                            .map_err(|_| anyhow!("Failed to get relative path"))?;
                        let dest_file = dest_dir.parent().unwrap().join(relative_path);
                        
                        if let Some(parent) = dest_file.parent() {
                            fs::create_dir_all(parent).await
                                .map_err(|e| anyhow!("Failed to create parent directory: {}", e))?;
                        }
                        
                        fs::write(&dest_file, content).await
                            .map_err(|e| anyhow!("Failed to copy source file: {}", e))?;
                    }
                }
            }
            Ok(())
        })
    }

    /// Write in-memory documents to the temporary workspace
    async fn write_in_memory_documents(workspace: &Path, documents: &HashMap<PathBuf, String>) -> Result<()> {
        for (relative_path, content) in documents {
            let file_path = workspace.join(relative_path);
            
            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).await
                    .map_err(|e| anyhow!("Failed to create directory structure: {}", e))?;
            }
            
            fs::write(&file_path, content).await
                .map_err(|e| anyhow!("Failed to write in-memory document {:?}: {}", relative_path, e))?;
            
            debug!("Wrote in-memory document {:?} to temporary workspace", relative_path);
        }
        Ok(())
    }

    /// Recursively copy a directory and all its contents
    fn copy_directory_recursive<'a>(source: &'a Path, dest: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // Create destination directory
            fs::create_dir_all(dest).await
                .map_err(|e| anyhow!("Failed to create directory {:?}: {}", dest, e))?;

            let mut entries = fs::read_dir(source).await
                .map_err(|e| anyhow!("Failed to read directory {:?}: {}", source, e))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| anyhow!("Failed to read directory entry: {}", e))? {
                let entry_path = entry.path();
                let file_name = entry.file_name();
                let dest_path = dest.join(&file_name);

                if entry_path.is_dir() {
                    // Recursively copy subdirectory
                    Self::copy_directory_recursive(&entry_path, &dest_path).await?;
                } else {
                    // Copy file
                    fs::copy(&entry_path, &dest_path).await
                        .map_err(|e| anyhow!("Failed to copy file {:?} to {:?}: {}", entry_path, dest_path, e))?;
                }
            }

            Ok(())
        })
    }

    /// Get the workspace path
    fn path(&self) -> &Path {
        &self.workspace_path
    }
}

/// Interface for interacting with the Gren compiler
#[derive(Clone)]
pub struct GrenCompiler {
    config: CompilerConfig,
    /// Semaphore to limit concurrent compilations
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl GrenCompiler {
    /// Create a new Gren compiler interface
    pub fn new(config: CompilerConfig) -> Self {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrent));
        Self { config, semaphore }
    }

    /// Create with default configuration, optionally using GREN_COMPILER_PATH environment variable
    pub fn with_env() -> Result<Self> {
        let mut config = CompilerConfig::default();
        
        // Check for GREN_COMPILER_PATH environment variable
        if let Ok(compiler_path) = std::env::var("GREN_COMPILER_PATH") {
            config.compiler_path = PathBuf::from(compiler_path);
            info!("Using Gren compiler from GREN_COMPILER_PATH: {:?}", config.compiler_path);
        }
        
        Ok(Self::new(config))
    }

    /// Check if the Gren compiler is available and working
    pub async fn check_availability(&self) -> Result<()> {
        debug!("Checking Gren compiler availability at {:?}", self.config.compiler_path);
        
        let output = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            Command::new(&self.config.compiler_path)
                .arg("--help")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        ).await
        .map_err(|_| anyhow!("Compiler check timed out after {}ms", self.config.timeout_ms))?
        .map_err(|e| anyhow!("Failed to execute compiler: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Compiler returned non-zero exit code: {}. Stderr: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains("gren") {
            return Err(anyhow!("Compiler output doesn't look like Gren compiler: {}", stdout));
        }

        info!("Gren compiler available and responsive");
        Ok(())
    }

    /// Compile a module and return diagnostics
    pub async fn compile(&self, request: CompileRequest) -> Result<CompileResult> {
        // Acquire semaphore to limit concurrent compilations
        let _permit = self.semaphore.acquire().await
            .map_err(|_| anyhow!("Failed to acquire compilation semaphore"))?;

        debug!("Starting compilation of module '{}' in project {:?}", 
               request.module_name, request.project_root);

        // Validate project root has gren.json
        let gren_json_path = request.project_root.join("gren.json");
        if !fs::metadata(&gren_json_path).await.is_ok() {
            return Err(anyhow!("Project root {:?} does not contain gren.json", request.project_root));
        }

        // Create temporary workspace with in-memory documents
        let workspace = if !request.in_memory_documents.is_empty() {
            debug!("Creating temporary workspace for {} in-memory documents", 
                   request.in_memory_documents.len());
            Some(TempWorkspace::new(&request.project_root, &request.in_memory_documents).await?)
        } else {
            None
        };

        // Determine working directory (temp workspace or original project)
        let working_dir = workspace.as_ref()
            .map(|w| w.path())
            .unwrap_or(&request.project_root);

        debug!("Using working directory: {:?}", working_dir);

        // Build compiler command using module name directly
        let mut cmd = Command::new(&self.config.compiler_path);
        cmd.arg("make")
           .arg(&request.module_name)  // Use module name directly (e.g., "Main", "Utils.Helper")
           .arg("--report=json")
           .arg("--output=/dev/null") // We only want diagnostics, not JS output
           .current_dir(working_dir)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        if request.include_sourcemaps {
            cmd.arg("--sourcemaps");
        }

        // Execute with timeout
        let start_time = std::time::Instant::now();
        let output = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            cmd.output()
        ).await
        .map_err(|_| anyhow!("Compilation timed out after {}ms", self.config.timeout_ms))?
        .map_err(|e| anyhow!("Failed to execute compiler: {}", e))?;

        let compile_time = start_time.elapsed();
        debug!("Compilation completed in {:?} with exit code {:?}", 
               compile_time, output.status.code());

        // Process results
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();
        let exit_code = output.status.code();

        // Determine the output to use for diagnostic parsing
        // When using --report=json, Gren compiler outputs JSON errors to stderr, not stdout
        let diagnostic_output = if stdout.trim().is_empty() && !stderr.trim().is_empty() {
            debug!("Using stderr for diagnostic output (stdout was empty)");
            stderr.clone()
        } else {
            stdout.clone()
        };

        if success {
            info!("Module '{}' compiled successfully in {:?}", request.module_name, compile_time);
        } else {
            warn!("Module '{}' compilation failed with exit code {:?}. Output: {}", 
                  request.module_name, exit_code, diagnostic_output);
        }

        // Temporary workspace automatically cleaned up when dropped
        if workspace.is_some() {
            debug!("Temporary workspace cleanup will occur on drop");
        }

        Ok(CompileResult {
            success,
            output: diagnostic_output,
            stderr,
            exit_code,
        })
    }

    /// Parse compiler output into structured diagnostics
    pub fn parse_compiler_output(&self, output: &str) -> Result<Option<CompilerOutput>> {
        if output.trim().is_empty() {
            return Ok(None);
        }

        match serde_json::from_str::<CompilerOutput>(output) {
            Ok(parsed) => {
                debug!("Successfully parsed compiler output");
                Ok(Some(parsed))
            }
            Err(e) => {
                error!("Failed to parse compiler output as JSON: {}. Output: {}", e, output);
                Err(anyhow!("Invalid compiler output format: {}", e))
            }
        }
    }

    /// Get compiler configuration
    pub fn config(&self) -> &CompilerConfig {
        &self.config
    }

    /// Update compiler configuration
    pub fn update_config(&mut self, config: CompilerConfig) {
        let old_max_concurrent = self.config.max_concurrent;
        self.config = config;
        
        // Update semaphore if max concurrent changed
        if old_max_concurrent != self.config.max_concurrent {
            self.semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent));
            info!("Updated max concurrent compilations from {} to {}", 
                  old_max_concurrent, self.config.max_concurrent);
        }
    }

    /// Compile a single file for validation purposes
    pub async fn compile_file(&self, file_path: &std::path::Path) -> Result<CompileResult> {
        debug!("Compiling file for validation: {:?}", file_path);
        
        // Acquire semaphore permit to limit concurrent compilations
        let _permit = self.semaphore.acquire().await
            .map_err(|e| anyhow!("Failed to acquire compilation semaphore: {}", e))?;
        
        let start_time = std::time::Instant::now();
        
        // Run gren compiler on the file
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            Command::new(&self.config.compiler_path)
                .arg("make")
                .arg(file_path)
                .arg("--report=json")
                .arg("--output=/dev/null") // Don't generate output files
                .current_dir(file_path.parent().unwrap_or(std::path::Path::new(".")))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        ).await;
        
        let output = match result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(anyhow!("Failed to execute compiler: {}", e));
            }
            Err(_) => {
                return Err(anyhow!("Compilation timed out after {}ms", self.config.timeout_ms));
            }
        };
        
        let duration = start_time.elapsed();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();
        
        debug!(
            "Compilation {} in {:?} (exit code: {:?})",
            if success { "succeeded" } else { "failed" },
            duration,
            exit_code
        );
        
        if !success {
            debug!("Compilation stderr: {}", stderr);
        }
        
        Ok(CompileResult {
            success,
            output: stdout,
            stderr,
            exit_code,
        })
    }
}

/// Utility functions for working with Gren projects
pub mod project_utils {
    use super::*;

    /// Find the project root by searching upward for gren.json
    pub async fn find_project_root(start_path: &Path) -> Result<PathBuf> {
        let mut current = start_path.to_path_buf();
        
        loop {
            let gren_json = current.join("gren.json");
            if fs::metadata(&gren_json).await.is_ok() {
                debug!("Found project root at {:?}", current);
                return Ok(current);
            }
            
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return Err(anyhow!("Could not find gren.json in any parent directory of {:?}", start_path)),
            }
        }
    }

    /// Extract module name from file path within a project
    pub fn module_name_from_path(file_path: &Path, project_root: &Path) -> Result<String> {
        let relative_path = file_path.strip_prefix(project_root)
            .map_err(|_| anyhow!("File {:?} is not within project root {:?}", file_path, project_root))?;

        // Remove src/ prefix if present
        let path_in_src = relative_path.strip_prefix("src").unwrap_or(relative_path);
        
        // Remove .gren extension and convert path separators to dots
        let module_name = path_in_src
            .with_extension("")
            .components()
            .map(|c| c.as_os_str().to_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(".");

        if module_name.is_empty() {
            return Err(anyhow!("Could not determine module name from path {:?}", file_path));
        }

        Ok(module_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_compiler_config_from_env() {
        // Test default config
        let compiler = GrenCompiler::with_env().unwrap();
        assert_eq!(compiler.config.compiler_path, PathBuf::from("gren"));
        
        // Test with environment variable
        std::env::set_var("GREN_COMPILER_PATH", "/custom/path/to/gren");
        let compiler = GrenCompiler::with_env().unwrap();
        assert_eq!(compiler.config.compiler_path, PathBuf::from("/custom/path/to/gren"));
        
        std::env::remove_var("GREN_COMPILER_PATH");
    }

    #[tokio::test]
    async fn test_parse_compile_errors() {
        let compiler = GrenCompiler::new(CompilerConfig::default());
        
        let json_output = r#"{"type":"compile-errors","errors":[{"path":"/test/Main.gren","name":"Main","problems":[{"title":"SYNTAX ERROR","region":{"start":{"line":1,"column":1},"end":{"line":1,"column":5}},"message":["Test error message"]}]}]}"#;
        
        let result = compiler.parse_compiler_output(json_output).unwrap();
        assert!(result.is_some());
        
        match result.unwrap() {
            CompilerOutput::CompileErrors { errors } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].name, "Main");
                assert_eq!(errors[0].problems.len(), 1);
                assert_eq!(errors[0].problems[0].title, "SYNTAX ERROR");
            }
            _ => panic!("Expected compile errors"),
        }
    }

    #[tokio::test]
    async fn test_module_name_extraction() {
        use project_utils::*;
        
        let project_root = Path::new("/project");
        
        // Test main module
        let main_path = Path::new("/project/src/Main.gren");
        assert_eq!(module_name_from_path(main_path, project_root).unwrap(), "Main");
        
        // Test nested module
        let nested_path = Path::new("/project/src/Utils/Parser.gren");
        assert_eq!(module_name_from_path(nested_path, project_root).unwrap(), "Utils.Parser");
        
        // Test without src prefix
        let no_src_path = Path::new("/project/Example.gren");
        assert_eq!(module_name_from_path(no_src_path, project_root).unwrap(), "Example");
    }

    #[tokio::test]
    async fn test_find_project_root() {
        use project_utils::*;
        
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        let nested_dir = src_dir.join("Utils");
        
        // Create directory structure
        fs::create_dir_all(&nested_dir).await.unwrap();
        fs::write(project_root.join("gren.json"), "{}").await.unwrap();
        
        // Test finding from nested directory
        let found_root = find_project_root(&nested_dir).await.unwrap();
        assert_eq!(found_root, project_root);
        
        // Test finding from project root
        let found_root = find_project_root(project_root).await.unwrap();
        assert_eq!(found_root, project_root);
    }

    #[tokio::test]
    async fn test_temp_workspace_creation() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        
        // Create a basic project structure
        let src_dir = project_root.join("src");
        fs::create_dir_all(&src_dir).await.unwrap();
        fs::write(project_root.join("gren.json"), r#"{"type":"application"}"#).await.unwrap();
        fs::write(src_dir.join("Main.gren"), "module Main exposing (main)\nmain = \"hello\"").await.unwrap();
        
        // Create in-memory documents
        let mut in_memory_docs = HashMap::new();
        in_memory_docs.insert(PathBuf::from("src/Test.gren"), "module Test exposing (test)\ntest = 42".to_string());
        
        // Create temporary workspace
        let workspace = TempWorkspace::new(project_root, &in_memory_docs).await.unwrap();
        
        // Verify gren.json was copied
        let gren_json_content = fs::read_to_string(workspace.path().join("gren.json")).await.unwrap();
        assert!(gren_json_content.contains("application"));
        
        // Verify in-memory document was written
        let test_content = fs::read_to_string(workspace.path().join("src/Test.gren")).await.unwrap();
        assert!(test_content.contains("test = 42"));
        
        // Verify existing file was copied
        let main_content = fs::read_to_string(workspace.path().join("src/Main.gren")).await.unwrap();
        assert!(main_content.contains("main = \"hello\""));
    }
}