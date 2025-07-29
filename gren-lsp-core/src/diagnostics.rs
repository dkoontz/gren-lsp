use crate::parser::ParseError;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Converts tree-sitter parse errors to LSP diagnostics
pub fn parse_errors_to_diagnostics(errors: Vec<ParseError>) -> Vec<Diagnostic> {
    // Deduplicate and merge overlapping errors
    let deduplicated_errors = deduplicate_errors(errors);

    deduplicated_errors
        .into_iter()
        .map(|error| {
            let range = Range::new(
                Position::new(
                    error.start_position.row as u32,
                    error.start_position.column as u32,
                ),
                Position::new(
                    error.end_position.row as u32,
                    error.end_position.column as u32,
                ),
            );

            let message = create_detailed_error_message(&error);

            Diagnostic::new(
                range,
                Some(DiagnosticSeverity::ERROR),
                None,                         // code
                Some("gren-lsp".to_string()), // source
                message,
                None, // related_information
                None, // tags
            )
        })
        .collect()
}

/// Deduplicate overlapping or nested parse errors to avoid duplicate diagnostics
fn deduplicate_errors(mut errors: Vec<ParseError>) -> Vec<ParseError> {
    if errors.is_empty() {
        return errors;
    }

    // Sort by start position, then by length (larger errors first)
    errors.sort_by(|a, b| {
        let pos_cmp = a
            .start_position
            .row
            .cmp(&b.start_position.row)
            .then(a.start_position.column.cmp(&b.start_position.column));
        if pos_cmp != std::cmp::Ordering::Equal {
            return pos_cmp;
        }
        // For same position, prefer larger ranges (parent errors over child errors)
        let a_len = a.end_byte - a.start_byte;
        let b_len = b.end_byte - b.start_byte;
        b_len.cmp(&a_len) // Larger ranges first
    });

    let mut deduplicated = Vec::new();

    for error in errors {
        let should_keep = deduplicated.iter().all(|existing: &ParseError| {
            !ranges_overlap(&error, existing) || is_significantly_different(&error, existing)
        });

        if should_keep {
            deduplicated.push(error);
        }
    }

    deduplicated
}

/// Check if two parse error ranges overlap
fn ranges_overlap(a: &ParseError, b: &ParseError) -> bool {
    // Check if ranges overlap on the same line or span multiple lines
    let a_start = (a.start_position.row, a.start_position.column);
    let a_end = (a.end_position.row, a.end_position.column);
    let b_start = (b.start_position.row, b.start_position.column);
    let b_end = (b.end_position.row, b.end_position.column);

    // No overlap if one range ends before the other starts
    !(a_end <= b_start || b_end <= a_start)
}

/// Check if two errors are significantly different enough to keep both
fn is_significantly_different(a: &ParseError, b: &ParseError) -> bool {
    // Keep both if they have different error kinds
    if a.kind != b.kind {
        return true;
    }

    // For overlapping errors of the same kind, check if they're really different enough to keep both
    if let (Some(a_parent), Some(b_parent)) = (&a.context.parent_kind, &b.context.parent_kind) {
        // Don't keep both if one parent is just "ERROR" (generic) and the other is more specific
        if a_parent == "ERROR" || b_parent == "ERROR" {
            return false;
        }

        // Keep both only if they have truly different meaningful contexts
        if a_parent != b_parent {
            return true;
        }
    }

    // Keep both if they have very different actual text that isn't just a substring
    if let (Some(a_text), Some(b_text)) = (&a.context.actual_text, &b.context.actual_text) {
        if a_text != b_text && !a_text.contains(b_text) && !b_text.contains(a_text) {
            return true;
        }
    }

    false
}

/// Create a detailed error message based on parse error context
fn create_detailed_error_message(error: &ParseError) -> String {
    if error.is_missing {
        // Handle missing nodes
        if let Some(expected) = &error.context.expected {
            format!("Missing {}", expected)
        } else {
            format!("Missing {}", error.kind)
        }
    } else {
        // Handle unexpected tokens with context
        match (
            &error.context.expected,
            &error.context.actual_text,
            &error.context.parent_kind,
        ) {
            (Some(expected), Some(actual), Some(parent)) => {
                // Special case for type annotations to be more specific
                if parent == "type_annotation" || parent == "type_ref" {
                    format!(
                        "Expected '{}' in type signature, but found '{}'",
                        expected, actual
                    )
                } else {
                    format!(
                        "Expected {} in {}, but found '{}'",
                        expected, parent, actual
                    )
                }
            }
            (Some(expected), None, Some(parent)) => {
                if parent == "type_annotation" || parent == "type_ref" {
                    format!("Expected '{}' in type signature", expected)
                } else {
                    format!("Expected {} in {}", expected, parent)
                }
            }
            (Some(expected), Some(actual), None) => {
                format!("Expected {}, but found '{}'", expected, actual)
            }
            (Some(expected), None, None) => {
                format!("Expected {}", expected)
            }
            (None, Some(actual), Some(parent)) => {
                if parent == "type_annotation" || parent == "type_ref" {
                    format!("Invalid type signature: unexpected '{}'", actual)
                } else {
                    format!("Unexpected '{}' in {}", actual, parent)
                }
            }
            (None, Some(actual), None) => {
                format!("Unexpected '{}'", actual)
            }
            (None, None, Some(parent)) => {
                if parent == "type_annotation" || parent == "type_ref" {
                    "Invalid type signature".to_string()
                } else {
                    format!("Syntax error in {}", parent)
                }
            }
            _ => {
                // Fallback to generic message
                format!("Syntax error: unexpected {}", error.kind)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ParseErrorContext;
    use crate::Parser;
    use tree_sitter::Point;

    #[test]
    fn test_parse_error_to_diagnostic() {
        let error = ParseError {
            start_byte: 0,
            end_byte: 5,
            start_position: Point { row: 0, column: 0 },
            end_position: Point { row: 0, column: 5 },
            kind: "ERROR".to_string(),
            is_missing: false,
            context: ParseErrorContext::default(),
        };

        let diagnostics = parse_errors_to_diagnostics(vec![error]);

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];

        assert_eq!(diagnostic.range.start.line, 0);
        assert_eq!(diagnostic.range.start.character, 0);
        assert_eq!(diagnostic.range.end.line, 0);
        assert_eq!(diagnostic.range.end.character, 5);
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source, Some("gren-lsp".to_string()));
        assert!(diagnostic.message.contains("Syntax error"));
    }

    #[test]
    fn test_missing_node_diagnostic() {
        let error = ParseError {
            start_byte: 0,
            end_byte: 0,
            start_position: Point { row: 1, column: 10 },
            end_position: Point { row: 1, column: 10 },
            kind: "identifier".to_string(),
            is_missing: true,
            context: ParseErrorContext::default(),
        };

        let diagnostics = parse_errors_to_diagnostics(vec![error]);

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];

        assert_eq!(diagnostic.range.start.line, 1);
        assert_eq!(diagnostic.range.start.character, 10);
        assert!(diagnostic.message.contains("Missing identifier"));
    }

    #[test]
    fn test_detailed_error_message_with_context() {
        let mut context = ParseErrorContext::default();
        context.expected = Some("->".to_string());
        context.actual_text = Some("Int".to_string());
        context.parent_kind = Some("type_annotation".to_string());

        let error = ParseError {
            start_byte: 0,
            end_byte: 3,
            start_position: Point { row: 0, column: 15 },
            end_position: Point { row: 0, column: 18 },
            kind: "ERROR".to_string(),
            is_missing: false,
            context,
        };

        let diagnostics = parse_errors_to_diagnostics(vec![error]);
        let diagnostic = &diagnostics[0];

        assert_eq!(
            diagnostic.message,
            "Expected '->' in type signature, but found 'Int'"
        );
    }

    #[test]
    fn test_parser_finds_syntax_errors() {
        let mut parser = Parser::new().expect("Failed to create parser");

        // Test with invalid Gren syntax
        let invalid_gren = r#"
module Test exposing (..)

func x y = 
  let z = 
  -- missing 'in' keyword here
  x + y + z

badFunc = 
"#;

        let tree = parser
            .parse(invalid_gren)
            .expect("Parse should not fail")
            .expect("Tree should exist");
        let errors = Parser::extract_errors(&tree);

        println!("Found {} errors in invalid Gren code", errors.len());
        for error in &errors {
            println!(
                "Error: {} at {:?}-{:?} (missing: {})",
                error.kind, error.start_position, error.end_position, error.is_missing
            );
        }

        // We should find at least some syntax errors
        assert!(
            !errors.is_empty(),
            "Parser should detect syntax errors in invalid Gren code"
        );
    }
}
