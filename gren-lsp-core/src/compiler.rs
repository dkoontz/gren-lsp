use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, info, warn};

/// Project type as defined in gren.json
#[derive(Debug, Clone, PartialEq)]
enum ProjectType {
    Application,
    Package,
}

/// Represents the Gren compiler integration layer
pub struct GrenCompiler {
    /// Path to the gren executable
    gren_path: PathBuf,
    /// Working directory for compilation
    working_dir: PathBuf,
    /// Cache of compilation results
    cache: std::collections::HashMap<PathBuf, CompilationResult>,
    /// Cached project type
    project_type_cache: Option<ProjectType>,
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
#[serde(rename_all = "kebab-case")]
enum GrenCompilerOutput {
    CompileErrors {
        errors: Vec<GrenError>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct GrenError {
    path: String,
    name: String,
    problems: Vec<GrenProblem>,
}

#[derive(Debug, Deserialize)]
struct GrenProblem {
    title: String,
    region: GrenRegion,
    message: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GrenRegion {
    start: GrenPosition,
    end: GrenPosition,
}

#[derive(Debug, Deserialize)]
struct GrenPosition {
    line: u32,
    column: u32,
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
            project_type_cache: None,
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
                info!("📦 Using cached compilation result for {}", file_path.display());
                return Ok(cached.clone());
            } else {
                info!("🔄 Cache invalidated for {} (content changed)", file_path.display());
            }
        } else {
            info!("🆕 No cached result for {}", file_path.display());
        }

        let result = self.run_compiler(file_path).await?;
        
        // Cache the result
        self.cache.insert(file_path.to_path_buf(), result.clone());
        
        Ok(result)
    }

    /// Run the Gren compiler on a file
    async fn run_compiler(&mut self, file_path: &Path) -> Result<CompilationResult> {
        let project_type = self.detect_project_type().await?;
        
        let mut cmd = AsyncCommand::new(&self.gren_path);
        cmd.arg("make")
            .arg(file_path)
            .arg("--report=json")
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Only add --output for applications, not packages
        if project_type == ProjectType::Application {
            cmd.arg("--output=/dev/null");
        }

        info!("🔨 Running Gren compiler on {} (project type: {:?})", file_path.display(), project_type);
        info!("📂 Working directory: {}", self.working_dir.display());
        debug!("Command: {:?}", cmd);

        let start_time = std::time::Instant::now();
        let output = cmd.output().await?;
        let duration = start_time.elapsed();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let success = output.status.success();
        info!("⏱️  Compilation took {:?}, success: {}", duration, success);
        
        if !stdout.is_empty() {
            debug!("Compiler stdout: {}", stdout);
        }
        if !stderr.is_empty() {
            info!("Compiler stderr: {}", stderr);
        }

        let diagnostics = self.parse_compiler_output(&stderr)?;
        info!("📋 Found {} compiler diagnostics", diagnostics.len());

        Ok(CompilationResult {
            success,
            diagnostics,
            timestamp: SystemTime::now(),
            content_hash: self.calculate_content_hash(file_path)?,
        })
    }

    /// Run the Gren compiler on a file from a specific working directory
    async fn run_compiler_in_directory(&mut self, file_path: &Path, working_dir: &Path) -> Result<CompilationResult> {
        let project_type = self.detect_project_type().await?;
        
        let mut cmd = AsyncCommand::new(&self.gren_path);
        cmd.arg("make")
            .arg(file_path)
            .arg("--report=json")
            .current_dir(working_dir)  // Use the provided working directory
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Only add --output for applications, not packages
        if project_type == ProjectType::Application {
            cmd.arg("--output=/dev/null");
        }

        info!("🔨 Running Gren compiler on {} (project type: {:?})", file_path.display(), project_type);
        info!("📂 Working directory: {}", working_dir.display());
        debug!("Command: {:?}", cmd);

        let start_time = std::time::Instant::now();
        let output = cmd.output().await?;
        let duration = start_time.elapsed();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let success = output.status.success();
        info!("⏱️  Compilation took {:?}, success: {}", duration, success);
        
        if !stdout.is_empty() {
            debug!("Compiler stdout: {}", stdout);
        }
        if !stderr.is_empty() {
            info!("Compiler stderr: {}", stderr);
        }

        let diagnostics = self.parse_compiler_output(&stderr)?;
        info!("📋 Found {} compiler diagnostics", diagnostics.len());

        Ok(CompilationResult {
            success,
            diagnostics,
            timestamp: SystemTime::now(),
            content_hash: self.calculate_content_hash(file_path)?,
        })
    }

    /// Copy all source files from the project to the temporary directory
    /// This is needed so the compiler can resolve imports when running from .tmp
    async fn copy_source_files_to_temp(&self, temp_base: &Path) -> Result<()> {
        use std::collections::HashSet;
        
        let src_dir = self.working_dir.join("src");
        if !src_dir.exists() {
            return Ok(()); // No src directory to copy
        }
        
        let temp_src_dir = temp_base.join("src");
        
        // Create a set to track which files we've already copied to avoid duplicates
        let mut copied_files = HashSet::new();
        
        // Recursively copy all .gren files from src to .tmp/src
        self.copy_gren_files_recursive(&src_dir, &temp_src_dir, &mut copied_files).await?;
        
        info!("📋 Copied {} source files to temp directory", copied_files.len());
        Ok(())
    }
    
    /// Recursively copy .gren files from source to destination
    fn copy_gren_files_recursive<'a>(
        &'a self, 
        src_dir: &'a Path, 
        dst_dir: &'a Path, 
        copied_files: &'a mut std::collections::HashSet<std::path::PathBuf>
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            use tokio::fs;
            
            if !src_dir.exists() {
                return Ok(());
            }
            
            // Create destination directory
            fs::create_dir_all(dst_dir).await?;
            
            let mut entries = fs::read_dir(src_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let src_path = entry.path();
                let file_name = src_path.file_name().unwrap();
                let dst_path = dst_dir.join(file_name);
                
                if src_path.is_dir() {
                    // Recursively copy subdirectories
                    self.copy_gren_files_recursive(&src_path, &dst_path, copied_files).await?;
                } else if src_path.extension().map_or(false, |ext| ext == "gren") {
                    // Copy .gren files, but skip if we've already copied this file
                    if !copied_files.contains(&src_path) {
                        if let Err(e) = fs::copy(&src_path, &dst_path).await {
                            warn!("Failed to copy {} to {}: {}", src_path.display(), dst_path.display(), e);
                        } else {
                            copied_files.insert(src_path);
                        }
                    }
                }
            }
            
            Ok(())
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
                match compiler_output {
                    GrenCompilerOutput::CompileErrors { errors } => {
                        for error in errors {
                            for problem in error.problems {
                                let diagnostic = CompilerDiagnostic {
                                    severity: DiagnosticSeverity::Error,
                                    title: problem.title,
                                    message: self.extract_message_text(problem.message),
                                    path: Some(PathBuf::from(error.path.clone())),
                                    location: Some(DiagnosticLocation {
                                        line: problem.region.start.line,
                                        column: problem.region.start.column,
                                        end_line: Some(problem.region.end.line),
                                        end_column: Some(problem.region.end.column),
                                    }),
                                };
                                diagnostics.push(diagnostic);
                            }
                        }
                    }
                    GrenCompilerOutput::Other => {
                        warn!("Unknown compiler output format, ignoring");
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse compiler output as JSON: {}", e);
                debug!("Compiler output was: {}", output);
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
        info!("🗑️  Invalidated cache for {}", file_path.display());
    }

    /// Invalidate cache for all files (useful when project configuration changes)
    pub fn invalidate_all_cache(&mut self) {
        let count = self.cache.len();
        self.cache.clear();
        self.project_type_cache = None; // Clear project type cache too
        info!("🗑️  Invalidated cache for all {} files and project type", count);
    }

    /// Force recompilation by bypassing cache
    pub async fn force_compile_file(&mut self, file_path: &Path) -> Result<CompilationResult> {
        info!("🔄 Force compiling {} (bypassing cache)", file_path.display());
        self.invalidate_cache(file_path);
        self.compile_file(file_path).await
    }

    /// Compile in-memory content by writing to a temporary file
    pub async fn compile_content(&mut self, content: &str, original_path: &Path) -> Result<CompilationResult> {
        use tokio::fs;
        use tokio::io::AsyncWriteExt;
        
        // Debug: Show the lines around where the error occurred to see what the compiler is seeing
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() >= 8 {
            info!("🔍 Content around line 8 (where error occurred):");
            for (i, line) in lines.iter().enumerate().skip(5).take(5) {
                info!("  {}: {}", i + 1, line);
            }
        }
        
        
        // Create a .tmp directory structure at project root that mirrors the original structure
        // This preserves module names and allows proper import resolution
        let relative_path = if original_path.is_absolute() {
            // Convert absolute path to relative by stripping the working directory
            original_path.strip_prefix(&self.working_dir)
                .unwrap_or(original_path)
        } else {
            original_path
        };
        
        // Create temp directory structure in OS temp directory
        let temp_base = std::env::temp_dir()
            .join("gren-lsp")
            .join(format!("compile_{}", std::process::id()));
        let temp_file_path = temp_base.join(relative_path);
        let temp_dir = temp_file_path.parent().unwrap_or(&temp_base).to_path_buf();
        
        info!("💾 Writing in-memory content to temporary file: {}", temp_file_path.display());
        info!("📁 Temp file directory: {}", temp_dir.display());
        
        // Ensure the temp directory exists
        if let Err(e) = fs::create_dir_all(&temp_dir).await {
            warn!("Failed to create temp directory {}: {}", temp_dir.display(), e);
        }
        
        // Copy gren.json to .tmp directory so compiler can find project configuration
        let gren_json_src = self.working_dir.join("gren.json");
        let gren_json_dst = temp_base.join("gren.json");
        if gren_json_src.exists() && !gren_json_dst.exists() {
            if let Err(e) = fs::copy(&gren_json_src, &gren_json_dst).await {
                warn!("Failed to copy gren.json to temp directory: {}", e);
            } else {
                info!("📋 Copied gren.json to temp directory");
            }
        }
        
        // Copy all source files to .tmp directory FIRST so imports can be resolved
        // This is needed because Gren needs access to all modules for compilation
        if let Err(e) = self.copy_source_files_to_temp(&temp_base).await {
            warn!("Failed to copy source files to temp directory: {}", e);
        } else {
            info!("📂 Copied source files to temp directory for import resolution");
        }
        
        // Write the corrected content to temporary file AFTER copying other files
        // This ensures our in-memory changes override the disk version
        let mut temp_file = fs::File::create(&temp_file_path).await?;
        temp_file.write_all(content.as_bytes()).await?;
        temp_file.flush().await?;
        
        // Ensure the file is closed before compilation
        drop(temp_file);
        info!("✏️  Overwrote temp file with corrected in-memory content");
        
        // Debug: Read back the temp file to verify what was actually written
        if let Ok(written_content) = fs::read_to_string(&temp_file_path).await {
            let written_lines: Vec<&str> = written_content.lines().collect();
            if written_lines.len() >= 8 {
                info!("🔍 Final temp file content around line 8:");
                for (i, line) in written_lines.iter().enumerate().skip(5).take(5) {
                    info!("  {}: {}", i + 1, line);
                }
            }
        }
        
        // Compile the temporary file, running from the .tmp directory
        let mut result = self.run_compiler_in_directory(&temp_file_path, &temp_base).await;
        
        // Adjust the diagnostic paths to point to the original file
        match result {
            Ok(mut compilation_result) => {
                for diagnostic in &mut compilation_result.diagnostics {
                    if let Some(ref mut path) = diagnostic.path {
                        // Check if the diagnostic path matches the temp file path
                        // Use canonicalized comparison to handle /var vs /private/var differences on macOS
                        let paths_match = if let (Ok(canonical_diagnostic), Ok(canonical_temp)) = 
                            (path.canonicalize(), temp_file_path.canonicalize()) {
                            canonical_diagnostic == canonical_temp
                        } else {
                            // Fallback to direct comparison if canonicalization fails
                            path == &temp_file_path
                        };
                        
                        if paths_match {
                            info!("📍 Adjusting diagnostic path from {} to {}", path.display(), original_path.display());
                            *path = original_path.to_path_buf();
                        } else {
                            info!("⚠️ Diagnostic path {} doesn't match temp file path {}", path.display(), temp_file_path.display());
                        }
                    }
                }
                
                // Update the content hash to be based on the actual content, not the temp file
                compilation_result.content_hash = self.calculate_content_hash_from_string(content);
                
                info!("✅ Successfully compiled in-memory content with {} diagnostics", 
                      compilation_result.diagnostics.len());
                
                result = Ok(compilation_result);
            }
            Err(e) => {
                warn!("❌ Failed to compile temporary file: {}", e);
                result = Err(e);
            }
        }
        
        // Clean up the temporary file
        if let Err(e) = fs::remove_file(&temp_file_path).await {
            warn!("Failed to cleanup temporary file {}: {}", temp_file_path.display(), e);
        } else {
            info!("🗑️  Cleaned up temporary file: {}", temp_file_path.display());
        }
        
        // Clean up the temporary gren.json file if it exists
        let gren_json_dst = temp_base.join("gren.json");
        if gren_json_dst.exists() {
            if let Err(e) = fs::remove_file(&gren_json_dst).await {
                warn!("Failed to cleanup temporary gren.json: {}", e);
            } else {
                info!("🗑️  Cleaned up temporary gren.json");
            }
        }
        
        // Clean up the entire .tmp directory to remove all copied source files
        if temp_base.exists() {
            if let Err(e) = fs::remove_dir_all(&temp_base).await {
                warn!("Failed to cleanup temporary directory {}: {}", temp_base.display(), e);
            } else {
                info!("🗑️  Cleaned up temporary directory: {}", temp_base.display());
            }
        }
        
        result
    }

    /// Calculate content hash from string content (for in-memory compilation)
    fn calculate_content_hash_from_string(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Detect the project type by reading gren.json
    async fn detect_project_type(&mut self) -> Result<ProjectType> {
        // Return cached type if available
        if let Some(ref cached_type) = self.project_type_cache {
            return Ok(cached_type.clone());
        }

        let gren_json_path = self.working_dir.join("gren.json");
        
        if !gren_json_path.exists() {
            info!("📋 No gren.json found, assuming application project");
            let project_type = ProjectType::Application;
            self.project_type_cache = Some(project_type.clone());
            return Ok(project_type);
        }

        match tokio::fs::read_to_string(&gren_json_path).await {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        let project_type = match json.get("type")
                            .and_then(|v| v.as_str()) 
                        {
                            Some("package") => {
                                info!("📦 Detected Gren package project");
                                ProjectType::Package
                            }
                            Some("application") => {
                                info!("🚀 Detected Gren application project");
                                ProjectType::Application
                            }
                            Some(other) => {
                                warn!("🤔 Unknown project type '{}', assuming application", other);
                                ProjectType::Application
                            }
                            None => {
                                warn!("⚠️  No 'type' field in gren.json, assuming application");
                                ProjectType::Application
                            }
                        };
                        
                        self.project_type_cache = Some(project_type.clone());
                        Ok(project_type)
                    }
                    Err(e) => {
                        warn!("❌ Failed to parse gren.json: {}, assuming application", e);
                        let project_type = ProjectType::Application;
                        self.project_type_cache = Some(project_type.clone());
                        Ok(project_type)
                    }
                }
            }
            Err(e) => {
                warn!("❌ Failed to read gren.json: {}, assuming application", e);
                let project_type = ProjectType::Application;
                self.project_type_cache = Some(project_type.clone());
                Ok(project_type)
            }
        }
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
                project_type_cache: None,
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

    #[test]
    fn test_real_gren_compiler_output_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let compiler = GrenCompiler::new(temp_dir.path().to_path_buf()).unwrap_or_else(|_| {
            // Create a mock compiler for testing
            GrenCompiler {
                gren_path: PathBuf::from("gren"),
                working_dir: temp_dir.path().to_path_buf(),
                cache: std::collections::HashMap::new(),
                project_type_cache: None,
            }
        });

        // Test with actual Gren compiler output format
        let test_json = r#"{"type":"compile-errors","errors":[{"path":"/Users/david/dev/gren-lang/core/src/String.gren","name":"String","problems":[{"title":"TOO MANY ARGS","region":{"start":{"line":154,"column":9},"end":{"line":154,"column":19}},"message":["The `String` type needs 0 arguments, but I see 1 instead:\n\n154| count : String Int\n             ",{"bold":false,"underline":false,"color":"RED","string":"^^^^^^^^^^"},"\nWhich is the extra one? Maybe some parentheses are missing?"]}]}]}"#;

        let diagnostics = compiler.parse_compiler_output(test_json).unwrap();
        
        assert_eq!(diagnostics.len(), 1);
        
        let diag = &diagnostics[0];
        assert_eq!(diag.title, "TOO MANY ARGS");
        assert!(diag.message.contains("String` type needs 0 arguments"));
        assert_eq!(diag.path, Some(PathBuf::from("/Users/david/dev/gren-lang/core/src/String.gren")));
        
        // Most importantly, check that location is parsed correctly
        assert!(diag.location.is_some());
        let location = diag.location.as_ref().unwrap();
        assert_eq!(location.line, 154);
        assert_eq!(location.column, 9);
        assert_eq!(location.end_line, Some(154));
        assert_eq!(location.end_column, Some(19));
    }
}