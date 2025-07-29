use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use crate::parser::ParseError;

/// Converts tree-sitter parse errors to LSP diagnostics
pub fn parse_errors_to_diagnostics(errors: Vec<ParseError>) -> Vec<Diagnostic> {
    errors
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

            let message = if error.is_missing {
                format!("Missing {}", error.kind)
            } else {
                format!("Syntax error: unexpected {}", error.kind)
            };

            Diagnostic::new(
                range,
                Some(DiagnosticSeverity::ERROR),
                None, // code
                Some("gren-lsp".to_string()), // source
                message,
                None, // related_information
                None, // tags
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Point;
    use crate::Parser;

    #[test]
    fn test_parse_error_to_diagnostic() {
        let error = ParseError {
            start_byte: 0,
            end_byte: 5,
            start_position: Point { row: 0, column: 0 },
            end_position: Point { row: 0, column: 5 },
            kind: "ERROR".to_string(),
            is_missing: false,
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
        };

        let diagnostics = parse_errors_to_diagnostics(vec![error]);
        
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        
        assert_eq!(diagnostic.range.start.line, 1);
        assert_eq!(diagnostic.range.start.character, 10);
        assert!(diagnostic.message.contains("Missing identifier"));
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
        
        let tree = parser.parse(invalid_gren).expect("Parse should not fail").expect("Tree should exist");
        let errors = Parser::extract_errors(&tree);
        
        println!("Found {} errors in invalid Gren code", errors.len());
        for error in &errors {
            println!("Error: {} at {:?}-{:?} (missing: {})", 
                     error.kind, error.start_position, error.end_position, error.is_missing);
        }
        
        // We should find at least some syntax errors
        assert!(!errors.is_empty(), "Parser should detect syntax errors in invalid Gren code");
    }
}