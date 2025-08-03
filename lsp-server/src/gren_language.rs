use anyhow::Result;
use tree_sitter::Language;

/// Get the Gren tree-sitter language
/// 
/// This function provides the tree-sitter language for Gren parsing.
/// The grammar is compiled from the tree-sitter-gren repository
/// and linked during the build process.
pub fn language() -> Result<Language> {
    Ok(tree_sitter_gren::language())
}

/// Check if the Gren language is available
/// 
/// This is a convenience function to check if tree-sitter-gren
/// is available without triggering an error.
pub fn is_available() -> bool {
    language().is_ok()
}

/// Get language metadata
pub fn metadata() -> LanguageMetadata {
    LanguageMetadata {
        name: "gren".to_string(),
        version: "5.7.0".to_string(),
        source: "https://github.com/MaeBrooks/tree-sitter-gren".to_string(),
        status: LanguageStatus::Available,
    }
}

#[derive(Debug, Clone)]
pub struct LanguageMetadata {
    pub name: String,
    pub version: String,
    pub source: String,
    pub status: LanguageStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LanguageStatus {
    Available,
    Pending,
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_metadata() {
        let meta = metadata();
        assert_eq!(meta.name, "gren");
        assert_eq!(meta.status, LanguageStatus::Available);
        assert!(meta.source.contains("tree-sitter-gren"));
        assert_eq!(meta.version, "5.7.0");
    }

    #[test]
    fn test_language_availability() {
        // Should now be available with compiled grammar
        assert!(is_available());
    }

    #[test]
    fn test_language_loading() {
        let result = language();
        assert!(result.is_ok());
        
        // Test that we can actually create a parser with this language
        let mut parser = tree_sitter::Parser::new();
        let lang = result.unwrap();
        parser.set_language(&lang).expect("Should be able to set language");
    }
}