use crate::{Document, Parser, SymbolIndex};
use anyhow::Result;
use lsp_types::*;

pub struct AnalysisEngine {
    #[allow(dead_code)]
    parser: Parser,
    #[allow(dead_code)]
    symbol_index: SymbolIndex,
}

impl AnalysisEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            parser: Parser::new()?,
            symbol_index: SymbolIndex::new()?,
        })
    }

    pub async fn analyze_document(&mut self, _document: &Document) -> Result<Vec<Diagnostic>> {
        // TODO: Implement analysis
        // 1. Parse document with tree-sitter
        // 2. Extract symbols
        // 3. Run type checking
        // 4. Generate diagnostics

        Ok(vec![])
    }
}
