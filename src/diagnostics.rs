use crate::compiler_interface::{CompilerOutput, CompileError, Problem};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::*;
use tracing::{debug, warn};

/// Converts Gren compiler output to LSP diagnostics
pub struct DiagnosticsConverter {
    /// Project root path for relative path resolution
    project_root: PathBuf,
}

impl DiagnosticsConverter {
    /// Create a new diagnostics converter
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Convert compiler output to LSP diagnostics grouped by file URI
    pub fn convert_to_diagnostics(
        &self, 
        compiler_output: &CompilerOutput
    ) -> Result<HashMap<Url, Vec<Diagnostic>>> {
        let mut diagnostics_map = HashMap::new();

        match compiler_output {
            CompilerOutput::CompileErrors { errors } => {
                for error in errors {
                    self.process_compile_error(error, &mut diagnostics_map)?;
                }
            }
            CompilerOutput::GeneralError { title, path, message } => {
                self.process_general_error(title, path, message, &mut diagnostics_map)?;
            }
        }

        debug!("Converted compiler output to diagnostics for {} files", diagnostics_map.len());
        Ok(diagnostics_map)
    }

    /// Process a compile error and add diagnostics to the map
    fn process_compile_error(
        &self,
        error: &CompileError,
        diagnostics_map: &mut HashMap<Url, Vec<Diagnostic>>,
    ) -> Result<()> {
        // Convert file path to URI
        let file_uri = self.path_to_uri(&error.path)?;
        
        // Convert problems to diagnostics
        let mut file_diagnostics = Vec::new();
        for problem in &error.problems {
            match self.problem_to_diagnostic(problem) {
                Ok(diagnostic) => file_diagnostics.push(diagnostic),
                Err(e) => {
                    warn!("Failed to convert problem to diagnostic: {}", e);
                    // Create a fallback diagnostic
                    file_diagnostics.push(self.create_fallback_diagnostic(problem));
                }
            }
        }

        // Add to map or extend existing diagnostics
        diagnostics_map
            .entry(file_uri)
            .or_insert_with(Vec::new)
            .extend(file_diagnostics);

        Ok(())
    }

    /// Process a general error and add diagnostics to the map
    fn process_general_error(
        &self,
        title: &str,
        path: &str,
        message: &str,
        diagnostics_map: &mut HashMap<Url, Vec<Diagnostic>>,
    ) -> Result<()> {
        // For general errors, create a diagnostic at the beginning of the file
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren-compiler".to_string()),
            message: format!("{}: {}", title, message),
            related_information: None,
            tags: None,
            data: None,
        };

        if path.is_empty() {
            // No specific file, this might be a project-level error
            // Create a synthetic URI for the project root
            let project_uri = Url::from_file_path(&self.project_root)
                .map_err(|_| anyhow!("Failed to create project URI"))?;
            diagnostics_map
                .entry(project_uri)
                .or_insert_with(Vec::new)
                .push(diagnostic);
        } else {
            let file_uri = self.path_to_uri(path)?;
            diagnostics_map
                .entry(file_uri)
                .or_insert_with(Vec::new)
                .push(diagnostic);
        }

        Ok(())
    }

    /// Convert a Gren problem to an LSP diagnostic
    fn problem_to_diagnostic(&self, problem: &Problem) -> Result<Diagnostic> {
        let range = self.gren_region_to_lsp_range(&problem.region)?;
        let severity = self.determine_severity(&problem.title);
        
        // Join message parts with proper formatting
        let message = if problem.message.len() == 1 {
            problem.message[0].clone()
        } else {
            problem.message.join("\n")
        };

        Ok(Diagnostic {
            range,
            severity: Some(severity),
            code: None, // Gren doesn't provide specific error codes
            code_description: None,
            source: Some("gren-compiler".to_string()),
            message: format!("{}: {}", problem.title, message),
            related_information: None,
            tags: self.determine_tags(&problem.title),
            data: None,
        })
    }

    /// Create a fallback diagnostic when conversion fails
    fn create_fallback_diagnostic(&self, problem: &Problem) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren-compiler".to_string()),
            message: format!("{}: {}", problem.title, problem.message.join(" ")),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Convert Gren region to LSP range
    fn gren_region_to_lsp_range(&self, region: &crate::compiler_interface::Region) -> Result<Range> {
        // Gren uses 1-based line and column numbers, LSP uses 0-based
        let start = Position {
            line: region.start.line.saturating_sub(1),
            character: region.start.column.saturating_sub(1),
        };
        
        let end = Position {
            line: region.end.line.saturating_sub(1),
            character: region.end.column.saturating_sub(1),
        };

        // Ensure end is not before start
        let end = if end.line < start.line || (end.line == start.line && end.character < start.character) {
            Position {
                line: start.line,
                character: start.character + 1, // Highlight at least one character
            }
        } else {
            end
        };

        Ok(Range { start, end })
    }

    /// Determine diagnostic severity based on problem title
    fn determine_severity(&self, title: &str) -> DiagnosticSeverity {
        match title {
            // Syntax errors are definitely errors
            title if title.contains("SYNTAX") => DiagnosticSeverity::ERROR,
            title if title.contains("PARSE") => DiagnosticSeverity::ERROR,
            title if title.contains("UNFINISHED") => DiagnosticSeverity::ERROR,
            
            // Type errors are errors
            title if title.contains("TYPE") => DiagnosticSeverity::ERROR,
            title if title.contains("MISMATCH") => DiagnosticSeverity::ERROR,
            
            // Import/module errors are errors
            title if title.contains("IMPORT") => DiagnosticSeverity::ERROR,
            title if title.contains("MODULE") => DiagnosticSeverity::ERROR,
            title if title.contains("MISSING") => DiagnosticSeverity::ERROR,
            title if title.contains("NOT FOUND") => DiagnosticSeverity::ERROR,
            
            // Warnings for style and optimization
            title if title.contains("UNUSED") => DiagnosticSeverity::WARNING,
            title if title.contains("STYLE") => DiagnosticSeverity::WARNING,
            title if title.contains("DEPRECATED") => DiagnosticSeverity::WARNING,
            
            // Information for hints
            title if title.contains("HINT") => DiagnosticSeverity::INFORMATION,
            title if title.contains("INFO") => DiagnosticSeverity::INFORMATION,
            
            // Default to error for safety
            _ => DiagnosticSeverity::ERROR,
        }
    }

    /// Determine diagnostic tags based on problem title
    fn determine_tags(&self, title: &str) -> Option<Vec<DiagnosticTag>> {
        let mut tags = Vec::new();
        
        if title.contains("UNUSED") {
            tags.push(DiagnosticTag::UNNECESSARY);
        }
        
        if title.contains("DEPRECATED") {
            tags.push(DiagnosticTag::DEPRECATED);
        }
        
        if tags.is_empty() {
            None
        } else {
            Some(tags)
        }
    }

    /// Convert file path to URI
    fn path_to_uri(&self, path: &str) -> Result<Url> {
        let path_buf = PathBuf::from(path);
        
        // Handle both absolute and relative paths
        let absolute_path = if path_buf.is_absolute() {
            path_buf
        } else {
            self.project_root.join(path_buf)
        };

        Url::from_file_path(&absolute_path)
            .map_err(|_| anyhow!("Failed to convert path to URI: {}", path))
    }

    /// Update project root
    pub fn set_project_root(&mut self, project_root: PathBuf) {
        self.project_root = project_root;
    }

    /// Get project root
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }
}

/// Helper functions for working with diagnostics
pub mod diagnostics_utils {
    use super::*;

    /// Filter diagnostics by severity
    pub fn filter_by_severity(
        diagnostics: &HashMap<Url, Vec<Diagnostic>>,
        min_severity: DiagnosticSeverity,
    ) -> HashMap<Url, Vec<Diagnostic>> {
        let severity_order = |s: DiagnosticSeverity| match s {
            DiagnosticSeverity::ERROR => 4,
            DiagnosticSeverity::WARNING => 3,
            DiagnosticSeverity::INFORMATION => 2,
            DiagnosticSeverity::HINT => 1,
            _ => 4, // Default unknown severities to error level
        };

        let min_order = severity_order(min_severity);

        diagnostics
            .iter()
            .map(|(uri, diags)| {
                let filtered_diags: Vec<Diagnostic> = diags
                    .iter()
                    .filter(|d| {
                        d.severity
                            .map(severity_order)
                            .unwrap_or(4) >= min_order
                    })
                    .cloned()
                    .collect();
                (uri.clone(), filtered_diags)
            })
            .filter(|(_, diags)| !diags.is_empty())
            .collect()
    }

    /// Count diagnostics by severity
    pub fn count_by_severity(diagnostics: &HashMap<Url, Vec<Diagnostic>>) -> (usize, usize, usize, usize) {
        let mut errors = 0;
        let mut warnings = 0;
        let mut info = 0;
        let mut hints = 0;
        
        for diags in diagnostics.values() {
            for diag in diags {
                match diag.severity.unwrap_or(DiagnosticSeverity::ERROR) {
                    DiagnosticSeverity::ERROR => errors += 1,
                    DiagnosticSeverity::WARNING => warnings += 1,
                    DiagnosticSeverity::INFORMATION => info += 1,
                    DiagnosticSeverity::HINT => hints += 1,
                    _ => errors += 1, // Default unknown severities to errors
                }
            }
        }
        
        (errors, warnings, info, hints)
    }

    /// Merge diagnostic maps (useful for incremental updates)
    pub fn merge_diagnostics(
        mut base: HashMap<Url, Vec<Diagnostic>>,
        new: HashMap<Url, Vec<Diagnostic>>,
    ) -> HashMap<Url, Vec<Diagnostic>> {
        for (uri, diagnostics) in new {
            base.insert(uri, diagnostics);
        }
        base
    }

    /// Clear diagnostics for specific files
    pub fn clear_file_diagnostics(
        diagnostics: &mut HashMap<Url, Vec<Diagnostic>>,
        uris: &[Url],
    ) {
        for uri in uris {
            diagnostics.remove(uri);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_interface::{Region, Problem, Position as GrenPosition};
    use tempfile::TempDir;

    #[test]
    fn test_diagnostics_converter_creation() {
        let temp_dir = TempDir::new().unwrap();
        let converter = DiagnosticsConverter::new(temp_dir.path().to_path_buf());
        assert_eq!(converter.project_root(), temp_dir.path());
    }

    #[test]
    fn test_gren_region_to_lsp_range() {
        let temp_dir = TempDir::new().unwrap();
        let converter = DiagnosticsConverter::new(temp_dir.path().to_path_buf());
        
        let gren_region = Region {
            start: GrenPosition { line: 1, column: 1 },
            end: GrenPosition { line: 1, column: 5 },
        };
        
        let lsp_range = converter.gren_region_to_lsp_range(&gren_region).unwrap();
        
        // Gren 1-based -> LSP 0-based
        assert_eq!(lsp_range.start.line, 0);
        assert_eq!(lsp_range.start.character, 0);
        assert_eq!(lsp_range.end.line, 0);
        assert_eq!(lsp_range.end.character, 4);
    }

    #[test]
    fn test_determine_severity() {
        let temp_dir = TempDir::new().unwrap();
        let converter = DiagnosticsConverter::new(temp_dir.path().to_path_buf());
        
        assert_eq!(converter.determine_severity("SYNTAX ERROR"), DiagnosticSeverity::ERROR);
        assert_eq!(converter.determine_severity("TYPE MISMATCH"), DiagnosticSeverity::ERROR);
        assert_eq!(converter.determine_severity("UNUSED VARIABLE"), DiagnosticSeverity::WARNING);
        assert_eq!(converter.determine_severity("HINT"), DiagnosticSeverity::INFORMATION);
        assert_eq!(converter.determine_severity("UNKNOWN ISSUE"), DiagnosticSeverity::ERROR);
    }

    #[test]
    fn test_determine_tags() {
        let temp_dir = TempDir::new().unwrap();
        let converter = DiagnosticsConverter::new(temp_dir.path().to_path_buf());
        
        let unused_tags = converter.determine_tags("UNUSED VARIABLE");
        assert!(unused_tags.is_some());
        assert!(unused_tags.unwrap().contains(&DiagnosticTag::UNNECESSARY));
        
        let deprecated_tags = converter.determine_tags("DEPRECATED FUNCTION");
        assert!(deprecated_tags.is_some());
        assert!(deprecated_tags.unwrap().contains(&DiagnosticTag::DEPRECATED));
        
        let no_tags = converter.determine_tags("SYNTAX ERROR");
        assert!(no_tags.is_none());
    }

    #[test]
    fn test_problem_to_diagnostic() {
        let temp_dir = TempDir::new().unwrap();
        let converter = DiagnosticsConverter::new(temp_dir.path().to_path_buf());
        
        let problem = Problem {
            title: "SYNTAX ERROR".to_string(),
            region: Region {
                start: GrenPosition { line: 2, column: 5 },
                end: GrenPosition { line: 2, column: 10 },
            },
            message: vec!["Expected closing parenthesis".to_string()],
        };
        
        let diagnostic = converter.problem_to_diagnostic(&problem).unwrap();
        
        assert_eq!(diagnostic.range.start.line, 1); // 0-based
        assert_eq!(diagnostic.range.start.character, 4); // 0-based
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diagnostic.message.contains("SYNTAX ERROR"));
        assert!(diagnostic.message.contains("Expected closing parenthesis"));
        assert_eq!(diagnostic.source, Some("gren-compiler".to_string()));
    }

    #[test]
    fn test_diagnostics_utils_filter_by_severity() {
        use diagnostics_utils::*;
        
        let mut diagnostics = HashMap::new();
        let uri = Url::parse("file:///test.gren").unwrap();
        
        diagnostics.insert(uri.clone(), vec![
            Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                message: "Error".to_string(),
                source: Some("test".to_string()),
                code: None,
                code_description: None,
                related_information: None,
                tags: None,
                data: None,
            },
            Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::WARNING),
                message: "Warning".to_string(),
                source: Some("test".to_string()),
                code: None,
                code_description: None,
                related_information: None,
                tags: None,
                data: None,
            },
        ]);
        
        let errors_only = filter_by_severity(&diagnostics, DiagnosticSeverity::ERROR);
        assert_eq!(errors_only.get(&uri).unwrap().len(), 1);
        
        let warnings_and_errors = filter_by_severity(&diagnostics, DiagnosticSeverity::WARNING);
        assert_eq!(warnings_and_errors.get(&uri).unwrap().len(), 2);
    }

    #[test]
    fn test_diagnostics_utils_count_by_severity() {
        use diagnostics_utils::*;
        
        let mut diagnostics = HashMap::new();
        let uri = Url::parse("file:///test.gren").unwrap();
        
        diagnostics.insert(uri.clone(), vec![
            Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                message: "Error 1".to_string(),
                source: Some("test".to_string()),
                code: None,
                code_description: None,
                related_information: None,
                tags: None,
                data: None,
            },
            Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                message: "Error 2".to_string(),
                source: Some("test".to_string()),
                code: None,
                code_description: None,
                related_information: None,
                tags: None,
                data: None,
            },
            Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::WARNING),
                message: "Warning".to_string(),
                source: Some("test".to_string()),
                code: None,
                code_description: None,
                related_information: None,
                tags: None,
                data: None,
            },
        ]);
        
        let (errors, warnings, info, hints) = count_by_severity(&diagnostics);
        assert_eq!(errors, 2);
        assert_eq!(warnings, 1);
        assert_eq!(info, 0);
        assert_eq!(hints, 0);
    }
}