use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tracing::{debug, trace};
use url::Url;

use crate::symbol_index::{Symbol, SymbolIndex, i32_to_symbol_kind};
use crate::scope_analysis::{ScopeAnalysis, LocalVariable};
use crate::tree_sitter_queries::GrenQueryEngine;
use crate::import_completion::ImportCompletionEngine;
use crate::import_manager::ImportManager;

/// Code completion engine for Gren language
pub struct CompletionEngine {
    /// Symbol index for cross-module resolution
    symbol_index: SymbolIndex,
    /// Tree-sitter query engine for AST analysis
    query_engine: GrenQueryEngine,
    /// Scope analysis for local variables
    scope_analysis: Arc<RwLock<ScopeAnalysis>>,
    /// Import completion engine for automatic imports (optional)
    import_completion: Option<Arc<ImportCompletionEngine>>,
}

/// Context information for completion requests
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// The position where completion was triggered
    pub position: Position,
    /// Current document URI
    pub uri: Url,
    /// Document content
    pub content: String,
    /// Character that triggered completion (if any)
    pub trigger_character: Option<String>,
    /// Current line text up to cursor
    pub line_prefix: String,
    /// Word being completed
    pub word_prefix: String,
}

/// Different types of completion triggers
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionType {
    /// Module member access (e.g., "Module.")
    ModuleMember { module_name: String },
    /// Local variable/function completion
    LocalScope,
    /// Keyword completion
    Keyword,
    /// Import completion
    Import,
    /// Type completion
    Type,
}

impl CompletionEngine {
    /// Create a new completion engine
    pub fn new(symbol_index: SymbolIndex) -> Result<Self> {
        let query_engine = GrenQueryEngine::new()?;
        let scope_analysis = ScopeAnalysis::new()?;

        Ok(Self {
            symbol_index,
            query_engine,
            scope_analysis: Arc::new(RwLock::new(scope_analysis)),
            import_completion: None,
        })
    }

    /// Create a new completion engine with import completion enabled
    pub fn new_with_import_completion(symbol_index: SymbolIndex) -> Result<Self> {
        let query_engine = GrenQueryEngine::new()?;
        let scope_analysis = ScopeAnalysis::new()?;
        
        // Create import manager and import completion engine
        let import_manager = Arc::new(ImportManager::new()?);
        let symbol_index_arc = Arc::new(RwLock::new(Some(symbol_index.clone())));
        let import_completion = Arc::new(ImportCompletionEngine::new(symbol_index_arc, import_manager));

        Ok(Self {
            symbol_index,
            query_engine,
            scope_analysis: Arc::new(RwLock::new(scope_analysis)),
            import_completion: Some(import_completion),
        })
    }

    /// Handle completion request from LSP
    pub async fn handle_completion(
        &self,
        params: CompletionParams,
        document_content: &str,
    ) -> Result<Option<CompletionResponse>> {
        let start_time = Instant::now();
        
        // Build completion context
        let context = self.build_completion_context(params, document_content)?;
        debug!("Completion context: {:?}", context);

        // Determine completion type based on context
        let completion_type = self.determine_completion_type(&context)?;
        trace!("Completion type: {:?}", completion_type);

        // Generate completion items based on type
        let items = match completion_type {
            CompletionType::ModuleMember { module_name } => {
                self.complete_module_members(&context, &module_name).await?
            }
            CompletionType::LocalScope => {
                self.complete_local_scope(&context).await?
            }
            CompletionType::Keyword => {
                self.complete_keywords(&context).await?
            }
            CompletionType::Import => {
                self.complete_imports(&context).await?
            }
            CompletionType::Type => {
                self.complete_types(&context).await?
            }
        };

        let elapsed = start_time.elapsed();
        debug!("Completion generated {} items in {:?}", items.len(), elapsed);

        Ok(Some(CompletionResponse::Array(items)))
    }

    /// Build completion context from LSP parameters
    fn build_completion_context(
        &self,
        params: CompletionParams,
        document_content: &str,
    ) -> Result<CompletionContext> {
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri.clone();
        let trigger_character = params.context
            .and_then(|ctx| ctx.trigger_character);

        // Extract current line and prefix
        let lines: Vec<&str> = document_content.lines().collect();
        let line_idx = position.line as usize;
        
        if line_idx >= lines.len() {
            return Err(anyhow!("Position line {} exceeds document length {}", line_idx, lines.len()));
        }

        let current_line = lines[line_idx];
        let char_idx = position.character as usize;
        let line_prefix = if char_idx <= current_line.len() {
            &current_line[..char_idx]
        } else {
            current_line
        };

        // Extract word being completed
        let word_prefix = extract_word_prefix(line_prefix);

        Ok(CompletionContext {
            position,
            uri,
            content: document_content.to_string(),
            trigger_character,
            line_prefix: line_prefix.to_string(),
            word_prefix,
        })
    }

    /// Determine what type of completion is needed
    fn determine_completion_type(&self, context: &CompletionContext) -> Result<CompletionType> {
        let line_prefix = &context.line_prefix;

        // Check for module member access (e.g., "Module." or "Module.func")
        if let Some(module_name) = extract_module_access(line_prefix) {
            return Ok(CompletionType::ModuleMember { module_name });
        }

        // Check for import context
        if line_prefix.trim_start().starts_with("import ") {
            return Ok(CompletionType::Import);
        }

        // Check for type context (after : or type declarations)
        if is_type_context(line_prefix) {
            return Ok(CompletionType::Type);
        }

        // Check for keyword context
        if should_suggest_keywords(line_prefix, &context.word_prefix) {
            return Ok(CompletionType::Keyword);
        }

        // Default to local scope completion
        Ok(CompletionType::LocalScope)
    }

    /// Complete module members (functions, types from imported modules)
    async fn complete_module_members(
        &self,
        context: &CompletionContext,
        module_name: &str,
    ) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        let prefix = &context.word_prefix;

        // Get available symbols from the specified module
        let module_symbols = self.symbol_index
            .find_available_symbols(&context.uri, module_name)
            .await?;

        for symbol in module_symbols {
            // Filter by prefix
            if !symbol.name.starts_with(prefix) {
                continue;
            }

            let completion_item = self.symbol_to_completion_item(&symbol, &context.uri);
            items.push(completion_item);
        }

        // Sort by relevance (exact prefix matches first, then alphabetical)
        items.sort_by(|a, b| {
            let a_exact = a.label.starts_with(prefix);
            let b_exact = b.label.starts_with(prefix);
            
            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.label.cmp(&b.label),
            }
        });

        Ok(items)
    }

    /// Complete local scope (variables, parameters, local functions)
    async fn complete_local_scope(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        let prefix = &context.word_prefix;

        // Analyze local scope at cursor position
        let scope_info = self.scope_analysis
            .write()
            .await
            .analyze_scope_at_position(&context.content, context.position)
            .await?;

        // Add local variables
        for var in &scope_info.local_variables {
            if var.name.starts_with(prefix) {
                let item = self.local_variable_to_completion_item(var);
                items.push(item);
            }
        }

        // Add imported symbols that are directly available
        let imported_symbols = self.symbol_index
            .find_symbols_by_prefix(&context.word_prefix, 100)
            .await?;

        for symbol in imported_symbols {
            if symbol.name.starts_with(prefix) {
                let completion_item = self.symbol_to_completion_item(&symbol, &context.uri);
                items.push(completion_item);
            }
        }

        // Add unimported symbols with automatic import completion (if enabled)
        if let Some(ref import_completion) = self.import_completion {
            let import_items = import_completion.complete_unimported_symbols(context).await?;
            for import_item in import_items {
                items.push(import_item.completion_item);
            }
        }

        // Sort with local variables taking precedence
        items.sort_by(|a, b| {
            let a_is_local = matches!(a.kind, Some(CompletionItemKind::VARIABLE));
            let b_is_local = matches!(b.kind, Some(CompletionItemKind::VARIABLE));
            
            match (a_is_local, b_is_local) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.label.cmp(&b.label),
            }
        });

        Ok(items)
    }

    /// Complete Gren keywords
    async fn complete_keywords(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let keywords = get_gren_keywords();
        let prefix = &context.word_prefix;
        let mut items = Vec::new();

        for keyword in keywords {
            if keyword.starts_with(prefix) {
                let item = CompletionItem {
                    label: keyword.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(format!("Gren keyword")),
                    documentation: Some(Documentation::String(
                        get_keyword_documentation(keyword)
                    )),
                    insert_text: Some(keyword.to_string()),
                    ..Default::default()
                };
                items.push(item);
            }
        }

        Ok(items)
    }

    /// Complete import module names
    async fn complete_imports(&self, _context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        // For now, return empty - would need access to available modules in workspace
        // This could be enhanced to read from gren.json dependencies or scan workspace
        Ok(Vec::new())
    }

    /// Complete type names
    async fn complete_types(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        let prefix = &context.word_prefix;

        // Get type symbols from symbol index
        let type_symbols = self.symbol_index
            .find_symbols_by_kind(SymbolKind::ENUM) // Custom types
            .await?;
        
        let alias_symbols = self.symbol_index
            .find_symbols_by_kind(SymbolKind::STRUCT) // Type aliases
            .await?;

        for symbol in type_symbols.into_iter().chain(alias_symbols.into_iter()) {
            if symbol.name.starts_with(prefix) {
                let completion_item = self.symbol_to_completion_item(&symbol, &context.uri);
                items.push(completion_item);
            }
        }

        // Add built-in types
        let builtin_types = get_builtin_types();
        for builtin_type in builtin_types {
            if builtin_type.starts_with(prefix) {
                let item = CompletionItem {
                    label: builtin_type.to_string(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("Built-in type".to_string()),
                    insert_text: Some(builtin_type.to_string()),
                    ..Default::default()
                };
                items.push(item);
            }
        }

        Ok(items)
    }

    /// Convert Symbol to CompletionItem
    fn symbol_to_completion_item(&self, symbol: &Symbol, _context_uri: &Url) -> CompletionItem {
        let kind = match i32_to_symbol_kind(symbol.kind) {
            SymbolKind::FUNCTION => CompletionItemKind::FUNCTION,
            SymbolKind::ENUM => CompletionItemKind::ENUM,
            SymbolKind::STRUCT => CompletionItemKind::STRUCT,
            SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
            SymbolKind::MODULE => CompletionItemKind::MODULE,
            _ => CompletionItemKind::TEXT,
        };

        let detail = if let Some(ref signature) = symbol.signature {
            // Extract just the type part from full signature like "greet : String -> String"
            if let Some(colon_pos) = signature.find(" : ") {
                signature[colon_pos + 3..].to_string()
            } else {
                signature.clone()
            }
        } else if let Some(ref container) = symbol.container {
            format!("from {}", container)
        } else {
            "local".to_string()
        };

        CompletionItem {
            label: symbol.name.clone(),
            kind: Some(kind),
            detail: Some(detail),
            documentation: symbol.signature.as_ref().map(|sig| 
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```gren\n{}\n```", sig),
                })
            ),
            insert_text: Some(symbol.name.clone()),
            ..Default::default()
        }
    }

    /// Convert LocalVariable to CompletionItem
    fn local_variable_to_completion_item(&self, var: &LocalVariable) -> CompletionItem {
        CompletionItem {
            label: var.name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some("local variable".to_string()),
            documentation: var.type_hint.as_ref().map(|type_hint|
                Documentation::String(format!("Type: {}", type_hint))
            ),
            insert_text: Some(var.name.clone()),
            ..Default::default()
        }
    }
}

/// Extract the word being completed from line prefix
fn extract_word_prefix(line_prefix: &str) -> String {
    // Find the last word boundary
    let mut word_start = line_prefix.len();
    for (i, c) in line_prefix.char_indices().rev() {
        if !c.is_alphanumeric() && c != '_' {
            word_start = i + c.len_utf8();
            break;
        }
        if i == 0 {
            word_start = 0;
            break;
        }
    }
    
    line_prefix[word_start..].to_string()
}

/// Extract module name from member access pattern (e.g., "Module." -> "Module")
fn extract_module_access(line_prefix: &str) -> Option<String> {
    // Look for pattern like "ModuleName." or "ModuleName.part"
    let trimmed = line_prefix.trim_end();
    
    // Find the last token that ends with a dot
    if let Some(dot_pos) = trimmed.rfind('.') {
        let before_dot = &trimmed[..dot_pos];
        
        // Extract the module name (could be qualified like Http.Request)
        let words: Vec<&str> = before_dot.split_whitespace().collect();
        if let Some(last_word) = words.last() {
            // Check if it looks like a module name (starts with uppercase)
            if last_word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                // For qualified names like "Http.Request", return the full qualified name
                return Some(last_word.to_string());
            }
        }
    }
    
    None
}

/// Check if we're in a type context
fn is_type_context(line_prefix: &str) -> bool {
    let trimmed = line_prefix.trim();
    
    // After type annotation colon
    if trimmed.contains(':') && !trimmed.contains('=') {
        return true;
    }
    
    // In type declaration
    if trimmed.starts_with("type ") {
        return true;
    }
    
    false
}

/// Check if we should suggest keywords
fn should_suggest_keywords(line_prefix: &str, word_prefix: &str) -> bool {
    let trimmed = line_prefix.trim_start();
    
    // At start of line or after certain tokens
    if trimmed.is_empty() || word_prefix.is_empty() {
        return true;
    }
    
    // After certain keywords or punctuation
    let triggers = ["=", "->", "when", "if", "then", "else", "let", "in"];
    for trigger in &triggers {
        if trimmed.ends_with(trigger) {
            return true;
        }
    }
    
    false
}

/// Get Gren language keywords
fn get_gren_keywords() -> &'static [&'static str] {
    &[
        "when", "is", "if", "then", "else", "let", "in", "type", "alias",
        "import", "exposing", "as", "module", "where", "port", "and", "or",
    ]
}

/// Get keyword documentation
fn get_keyword_documentation(keyword: &str) -> String {
    match keyword {
        "when" => "Pattern matching expression",
        "is" => "Pattern matching arm separator",
        "if" => "Conditional expression",
        "then" => "True branch of conditional",
        "else" => "False branch of conditional",
        "let" => "Local variable binding",
        "in" => "Body of let expression",
        "type" => "Custom type declaration",
        "alias" => "Type alias declaration",
        "import" => "Import module",
        "exposing" => "Expose specific symbols from import",
        "as" => "Import alias",
        "module" => "Module declaration",
        _ => "Gren keyword",
    }.to_string()
}

/// Get built-in type names
fn get_builtin_types() -> &'static [&'static str] {
    &[
        "Int", "Float", "String", "Bool", "Char",
        "Array", "Dict", "Set", "Maybe", "Result",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_word_prefix() {
        assert_eq!(extract_word_prefix("hello wo"), "wo");
        assert_eq!(extract_word_prefix("  func"), "func");
        assert_eq!(extract_word_prefix("Module."), "");
        assert_eq!(extract_word_prefix("Module.mem"), "mem");
        assert_eq!(extract_word_prefix(""), "");
    }

    #[test]
    fn test_extract_module_access() {
        assert_eq!(extract_module_access("Module."), Some("Module".to_string()));
        assert_eq!(extract_module_access("Http.Request."), Some("Http.Request".to_string()));
        assert_eq!(extract_module_access("  Module.func"), Some("Module".to_string()));
        assert_eq!(extract_module_access("lowercase."), None);
        assert_eq!(extract_module_access("no dot"), None);
    }

    #[test]
    fn test_is_type_context() {
        assert!(is_type_context("name : "));
        assert!(is_type_context("func : String -> "));
        assert!(is_type_context("type MyType "));
        assert!(!is_type_context("let x = "));
        assert!(!is_type_context("if condition"));
    }

    #[test]
    fn test_should_suggest_keywords() {
        assert!(should_suggest_keywords("  ", ""));
        assert!(should_suggest_keywords("= ", ""));
        assert!(should_suggest_keywords("when ", ""));
        assert!(!should_suggest_keywords("Module.func", "func"));
    }
}