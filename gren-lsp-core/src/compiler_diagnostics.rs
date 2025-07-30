use crate::compiler::{CompilerDiagnostic, DiagnosticSeverity};
use lsp_types::*;
use std::collections::HashMap;

/// Convert compiler diagnostics to LSP diagnostics
pub fn compiler_diagnostics_to_lsp(
    compiler_diagnostics: &[CompilerDiagnostic],
    uri: &Url,
) -> Vec<Diagnostic> {
    compiler_diagnostics
        .iter()
        .filter_map(|diag| compiler_diagnostic_to_lsp(diag, uri))
        .collect()
}

/// Convert a single compiler diagnostic to LSP diagnostic
fn compiler_diagnostic_to_lsp(diag: &CompilerDiagnostic, uri: &Url) -> Option<Diagnostic> {
    // Only include diagnostics for the current file
    if let Some(ref diag_path) = diag.path {
        if let Ok(diag_uri) = Url::from_file_path(diag_path) {
            if diag_uri != *uri {
                return None;
            }
        }
    }

    let lsp_severity = match diag.severity {
        DiagnosticSeverity::Error => Some(lsp_types::DiagnosticSeverity::ERROR),
        DiagnosticSeverity::Warning => Some(lsp_types::DiagnosticSeverity::WARNING),
        DiagnosticSeverity::Info => Some(lsp_types::DiagnosticSeverity::INFORMATION),
    };

    // Try to extract range from message or use default range
    let range = extract_range_from_diagnostic(diag).unwrap_or(Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 1,
        },
    });

    Some(Diagnostic {
        range,
        severity: lsp_severity,
        code: None,
        code_description: None,
        source: Some("gren".to_string()),
        message: format_diagnostic_message(diag),
        related_information: None,
        tags: None,
        data: None,
    })
}

/// Extract line/column range from diagnostic message or location
fn extract_range_from_diagnostic(diag: &CompilerDiagnostic) -> Option<Range> {
    // If we have explicit location information, use it
    if let Some(ref location) = diag.location {
        return Some(Range {
            start: Position {
                line: location.line.saturating_sub(1), // Convert to 0-based
                character: location.column.saturating_sub(1),
            },
            end: Position {
                line: location.end_line.unwrap_or(location.line).saturating_sub(1),
                character: location
                    .end_column
                    .unwrap_or(location.column + 1)
                    .saturating_sub(1),
            },
        });
    }

    // Try to parse location from message text
    parse_location_from_message(&diag.message)
}

/// Parse location information from error message text
#[allow(clippy::disallowed_methods, clippy::disallowed_types)]
fn parse_location_from_message(message: &str) -> Option<Range> {
    // Look for patterns like "line 5, column 10" or "5:10"
    // Note: Using regex here is acceptable for parsing compiler diagnostic output

    // Pattern: "line X, column Y"
    if let Some(captures) = regex::Regex::new(r"line (\d+),?\s*column (\d+)")
        .ok()?
        .captures(message)
    {
        if let (Ok(line), Ok(col)) = (
            captures.get(1)?.as_str().parse::<u32>(),
            captures.get(2)?.as_str().parse::<u32>(),
        ) {
            return Some(Range {
                start: Position {
                    line: line.saturating_sub(1),
                    character: col.saturating_sub(1),
                },
                end: Position {
                    line: line.saturating_sub(1),
                    character: col,
                },
            });
        }
    }

    // Pattern: "X:Y" (line:column)
    if let Some(captures) = regex::Regex::new(r"(\d+):(\d+)").ok()?.captures(message) {
        if let (Ok(line), Ok(col)) = (
            captures.get(1)?.as_str().parse::<u32>(),
            captures.get(2)?.as_str().parse::<u32>(),
        ) {
            return Some(Range {
                start: Position {
                    line: line.saturating_sub(1),
                    character: col.saturating_sub(1),
                },
                end: Position {
                    line: line.saturating_sub(1),
                    character: col,
                },
            });
        }
    }

    None
}

/// Format the diagnostic message for display
fn format_diagnostic_message(diag: &CompilerDiagnostic) -> String {
    if diag.title.is_empty() {
        diag.message.clone()
    } else if diag.message.is_empty() {
        diag.title.clone()
    } else {
        format!("{}: {}", diag.title, diag.message)
    }
}

/// Merge compiler diagnostics with existing syntax diagnostics
pub fn merge_diagnostics(
    compiler_diagnostics: Vec<Diagnostic>,
    syntax_diagnostics: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    let mut all_diagnostics = Vec::new();

    // Add syntax diagnostics first (they have priority)
    all_diagnostics.extend(syntax_diagnostics);

    // Add compiler diagnostics that don't overlap with syntax errors
    for compiler_diag in compiler_diagnostics {
        // Check if there's already a syntax diagnostic at this location
        let has_syntax_error = all_diagnostics.iter().any(|syntax_diag| {
            ranges_overlap(&syntax_diag.range, &compiler_diag.range)
                && syntax_diag.severity == Some(lsp_types::DiagnosticSeverity::ERROR)
        });

        if !has_syntax_error {
            all_diagnostics.push(compiler_diag);
        }
    }

    all_diagnostics
}

/// Check if two ranges overlap
fn ranges_overlap(a: &Range, b: &Range) -> bool {
    // Convert to comparable format
    let a_start = (a.start.line, a.start.character);
    let a_end = (a.end.line, a.end.character);
    let b_start = (b.start.line, b.start.character);
    let b_end = (b.end.line, b.end.character);

    // Check if ranges overlap
    a_start <= b_end && b_start <= a_end
}

/// Group diagnostics by file URI
pub fn group_diagnostics_by_uri(
    compiler_diagnostics: HashMap<Url, Vec<CompilerDiagnostic>>,
) -> HashMap<Url, Vec<Diagnostic>> {
    compiler_diagnostics
        .into_iter()
        .map(|(uri, diags)| {
            let lsp_diags = compiler_diagnostics_to_lsp(&diags, &uri);
            (uri, lsp_diags)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{DiagnosticLocation, DiagnosticSeverity};
    use std::path::PathBuf;

    #[test]
    fn test_compiler_diagnostic_conversion() {
        let uri = Url::parse("file:///test/Main.gren").unwrap();
        let compiler_diag = CompilerDiagnostic {
            severity: DiagnosticSeverity::Error,
            title: "TYPE MISMATCH".to_string(),
            message: "Expected Int but got String on line 5, column 10".to_string(),
            path: Some(PathBuf::from("/test/Main.gren")),
            location: Some(DiagnosticLocation {
                line: 5,
                column: 10,
                end_line: Some(5),
                end_column: Some(15),
            }),
        };

        let lsp_diags = compiler_diagnostics_to_lsp(&[compiler_diag], &uri);
        assert_eq!(lsp_diags.len(), 1);

        let diag = &lsp_diags[0];
        assert_eq!(diag.severity, Some(lsp_types::DiagnosticSeverity::ERROR));
        assert_eq!(diag.source, Some("gren".to_string()));
        assert!(diag.message.contains("TYPE MISMATCH"));
        assert_eq!(diag.range.start.line, 4); // 0-based
        assert_eq!(diag.range.start.character, 9); // 0-based
    }

    #[test]
    fn test_location_parsing() {
        let message = "Something went wrong on line 10, column 5";
        let range = parse_location_from_message(message).unwrap();

        assert_eq!(range.start.line, 9); // 0-based
        assert_eq!(range.start.character, 4); // 0-based
    }

    #[test]
    fn test_range_overlap() {
        let range1 = Range {
            start: Position {
                line: 5,
                character: 10,
            },
            end: Position {
                line: 5,
                character: 15,
            },
        };
        let range2 = Range {
            start: Position {
                line: 5,
                character: 12,
            },
            end: Position {
                line: 5,
                character: 20,
            },
        };
        let range3 = Range {
            start: Position {
                line: 10,
                character: 0,
            },
            end: Position {
                line: 10,
                character: 5,
            },
        };

        assert!(ranges_overlap(&range1, &range2));
        assert!(!ranges_overlap(&range1, &range3));
    }
}
