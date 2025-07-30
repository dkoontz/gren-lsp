use gren_lsp_core::{Symbol as GrenSymbol, Workspace};
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tracing::{debug, info, warn};

/// Information about a symbol at a cursor position
#[derive(Debug)]
struct SymbolAtPosition {
    function_name: String,
    module_path: Option<Vec<String>>,
}

pub struct Handlers {
    workspace: Arc<RwLock<Workspace>>,
}

impl Handlers {
    pub fn new(workspace: Arc<RwLock<Workspace>>) -> Self {
        Self { workspace }
    }

    pub async fn hover_with_capabilities(
        &self,
        params: HoverParams,
        client_capabilities: Option<&ClientCapabilities>,
    ) -> Result<Option<Hover>> {
        info!(
            "Hover requested at position {}:{}",
            params.text_document_position_params.position.line,
            params.text_document_position_params.position.character
        );

        let workspace = self.workspace.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Find the symbol at the cursor position
        match self
            .find_symbol_at_position(&workspace, uri, position)
            .await
        {
            Some(symbol_info) => {
                info!("Found symbol for hover: {:?}", symbol_info);

                // Search for the symbol definition to get type information
                let search_results = if let Some(module_path) = &symbol_info.module_path {
                    // For qualified symbols, search with module context
                    self.find_qualified_symbol(&workspace, &symbol_info.function_name, module_path)
                        .await
                } else {
                    // For unqualified symbols, use import-aware search
                    self.find_unqualified_symbol(&workspace, uri, &symbol_info.function_name)
                        .await
                };

                match search_results {
                    Ok(symbols) => {
                        if let Some(symbol) = symbols.first() {
                            // Check if client supports markdown format
                            let supports_markdown = client_capabilities
                                .and_then(|caps| caps.text_document.as_ref())
                                .and_then(|text_doc| text_doc.hover.as_ref())
                                .and_then(|hover| hover.content_format.as_ref())
                                .map(|formats| formats.contains(&MarkupKind::Markdown))
                                .unwrap_or(true); // Default to true for better compatibility

                            // Build hover content from the symbol
                            let hover_content = self
                                .build_hover_content(
                                    symbol,
                                    &symbol_info,
                                    &workspace,
                                    supports_markdown,
                                )
                                .await;

                            if !hover_content.is_empty() {
                                info!(
                                    "Generated hover content for '{}' (markdown support: {})",
                                    symbol_info.function_name, supports_markdown
                                );
                                return Ok(Some(Hover {
                                    contents: HoverContents::Markup(MarkupContent {
                                        kind: if supports_markdown {
                                            MarkupKind::Markdown
                                        } else {
                                            MarkupKind::PlainText
                                        },
                                        value: hover_content,
                                    }),
                                    range: None, // Let the client determine the hover range
                                }));
                            }
                        }

                        info!(
                            "No hover content generated for '{}'",
                            symbol_info.function_name
                        );
                        Ok(None)
                    }
                    Err(e) => {
                        info!(
                            "Error searching for hover symbol '{}': {}",
                            symbol_info.function_name, e
                        );
                        Ok(None)
                    }
                }
            }
            None => {
                info!("No symbol found at hover position");
                Ok(None)
            }
        }
    }

    pub async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        info!(
            "Completion requested at position {}:{}",
            params.text_document_position.position.line,
            params.text_document_position.position.character
        );

        let workspace = self.workspace.read().await;

        // Get symbols from the current file
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Get symbols from current file
        let mut completion_items = Vec::new();

        // Add local symbols from current file
        match workspace.get_file_symbols(uri) {
            Ok(symbols) => {
                for symbol in symbols {
                    // Skip symbols that are after the current position (can't complete future symbols)
                    if symbol.location.range.start.line > position.line {
                        continue;
                    }

                    // Apply basic context filtering
                    if !self.should_include_symbol_in_completion(&symbol, &params) {
                        continue;
                    }

                    let completion_item = CompletionItem {
                        label: symbol.name.clone(),
                        kind: Some(self.symbol_kind_to_completion_kind(symbol.kind)),
                        detail: symbol.type_signature.clone(),
                        documentation: symbol
                            .documentation
                            .as_ref()
                            .map(|doc| Documentation::String(doc.clone())),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("0_{}", symbol.name)), // Prioritize local symbols
                        ..Default::default()
                    };

                    completion_items.push(completion_item);
                }
            }
            Err(e) => {
                warn!("Failed to get current file symbols: {}", e);
            }
        }

        // Add symbols from workspace (other files)
        match workspace.find_symbols("") {
            Ok(workspace_symbols) => {
                for symbol in workspace_symbols {
                    // Skip symbols from the current file (already added above)
                    if symbol.location.uri == *uri {
                        continue;
                    }

                    // Skip module symbols to reduce noise
                    if symbol.kind == SymbolKind::MODULE {
                        continue;
                    }

                    // Apply basic context filtering
                    if !self.should_include_symbol_in_completion(&symbol, &params) {
                        continue;
                    }

                    let completion_item = CompletionItem {
                        label: symbol.name.clone(),
                        kind: Some(self.symbol_kind_to_completion_kind(symbol.kind)),
                        detail: symbol.type_signature.clone(),
                        documentation: symbol
                            .documentation
                            .as_ref()
                            .map(|doc| Documentation::String(doc.clone())),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("1_{}", symbol.name)), // Lower priority than local symbols
                        ..Default::default()
                    };

                    completion_items.push(completion_item);
                }
            }
            Err(e) => {
                warn!("Failed to get workspace symbols: {}", e);
            }
        }

        // Add Gren keywords
        let mut keyword_completions = self.get_keyword_completions();
        for keyword in &mut keyword_completions {
            keyword.sort_text = Some(format!("2_{}", keyword.label)); // Lowest priority
        }
        completion_items.extend(keyword_completions);

        info!("Returning {} completion items", completion_items.len());
        Ok(Some(CompletionResponse::Array(completion_items)))
    }

    pub async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        info!(
            "Go-to-definition requested at position {}:{}",
            params.text_document_position_params.position.line,
            params.text_document_position_params.position.character
        );

        let workspace = self.workspace.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // First, try to find the symbol at the cursor position
        match self
            .find_symbol_at_position(&workspace, uri, position)
            .await
        {
            Some(symbol_info) => {
                info!("Found symbol at cursor position: {:?}", symbol_info);

                // Search for the definition of this symbol in the workspace
                let search_results = if let Some(module_path) = &symbol_info.module_path {
                    // For qualified calls, search with module context
                    self.find_qualified_symbol(&workspace, &symbol_info.function_name, module_path)
                        .await
                } else {
                    // For unqualified calls, check imports first, then fallback to regular search
                    self.find_unqualified_symbol(&workspace, uri, &symbol_info.function_name)
                        .await
                };

                match search_results {
                    Ok(symbols) => {
                        // Filter symbols to find the best definition match
                        let mut definitions: Vec<Location> = symbols
                            .into_iter()
                            .filter(|symbol| {
                                // Prefer exact name matches
                                symbol.name == symbol_info.function_name &&
                                // Skip variable references, prefer definitions
                                symbol.kind != SymbolKind::VARIABLE &&
                                // Prefer function definitions over other types
                                (symbol.kind == SymbolKind::FUNCTION ||
                                 symbol.kind == SymbolKind::CLASS ||
                                 symbol.kind == SymbolKind::CONSTRUCTOR)
                            })
                            .map(|symbol| symbol.location)
                            .collect();

                        // Sort definitions to prefer the most likely ones
                        // For now, this is basic but could be enhanced with module awareness
                        definitions.sort_by(|a, b| {
                            // Prefer definitions from files with similar names
                            let a_filename = a.uri.path().split('/').last().unwrap_or("");
                            let b_filename = b.uri.path().split('/').last().unwrap_or("");

                            // If one is from a test file, prefer the other
                            let a_is_test =
                                a_filename.contains("test") || a_filename.contains("Test");
                            let b_is_test =
                                b_filename.contains("test") || b_filename.contains("Test");

                            match (a_is_test, b_is_test) {
                                (true, false) => std::cmp::Ordering::Greater, // b is better
                                (false, true) => std::cmp::Ordering::Less,    // a is better
                                _ => a.uri.cmp(&b.uri), // fallback to URI comparison
                            }
                        });

                        if definitions.is_empty() {
                            info!(
                                "No definitions found for symbol '{}'",
                                symbol_info.function_name
                            );
                            Ok(None)
                        } else if definitions.len() == 1 {
                            info!(
                                "Found single definition for '{}'",
                                symbol_info.function_name
                            );
                            Ok(Some(GotoDefinitionResponse::Scalar(
                                definitions.into_iter().next().unwrap(),
                            )))
                        } else {
                            info!(
                                "Found {} definitions for '{}'",
                                definitions.len(),
                                symbol_info.function_name
                            );
                            Ok(Some(GotoDefinitionResponse::Array(definitions)))
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to search for symbol '{}': {}",
                            symbol_info.function_name, e
                        );
                        Ok(None)
                    }
                }
            }
            None => {
                debug!("No symbol found at cursor position");
                Ok(None)
            }
        }
    }

    // Backward compatibility method - defaults to markdown support
    pub async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.hover_with_capabilities(params, None).await
    }

    pub async fn find_references(&self, _params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        // TODO: Implement find references
        Ok(None)
    }

    pub async fn document_symbols(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        info!(
            "Document symbols requested for: {}",
            params.text_document.uri
        );

        let workspace = self.workspace.read().await;

        // Get symbols for the specific file
        match workspace.get_file_symbols(&params.text_document.uri) {
            Ok(symbols) => {
                if symbols.is_empty() {
                    debug!(
                        "No symbols found for document: {}",
                        params.text_document.uri
                    );
                    return Ok(None);
                }

                info!(
                    "Found {} symbols for document: {}",
                    symbols.len(),
                    params.text_document.uri
                );

                // Convert to LSP document symbols with hierarchy
                let document_symbols = self.convert_to_document_symbols(symbols);

                Ok(Some(DocumentSymbolResponse::Nested(document_symbols)))
            }
            Err(e) => {
                warn!(
                    "Failed to get symbols for document {}: {}",
                    params.text_document.uri, e
                );
                Ok(None)
            }
        }
    }

    pub async fn workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        info!("Workspace symbols requested with query: '{}'", params.query);

        let workspace = self.workspace.read().await;

        // Search for symbols matching the query
        match workspace.find_symbols(&params.query) {
            Ok(symbols) => {
                if symbols.is_empty() {
                    debug!("No symbols found for query: '{}'", params.query);
                    return Ok(Some(Vec::new()));
                }

                info!(
                    "Found {} symbols for query: '{}'",
                    symbols.len(),
                    params.query
                );

                // Convert to LSP symbol information
                let symbol_information = symbols
                    .into_iter()
                    .map(|symbol| self.convert_to_symbol_information(symbol))
                    .collect();

                Ok(Some(symbol_information))
            }
            Err(e) => {
                warn!(
                    "Failed to search symbols for query '{}': {}",
                    params.query, e
                );
                Ok(Some(Vec::new()))
            }
        }
    }

    pub async fn code_action(
        &self,
        _params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        // TODO: Implement code actions
        Ok(None)
    }

    pub async fn rename(&self, _params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        // TODO: Implement rename
        Ok(None)
    }

    // Helper methods for symbol conversion

    /// Convert internal symbols to LSP DocumentSymbol format with hierarchy
    fn convert_to_document_symbols(&self, symbols: Vec<GrenSymbol>) -> Vec<DocumentSymbol> {
        let mut document_symbols = Vec::new();
        let mut processed_modules = std::collections::HashSet::new();
        let mut processed_types = std::collections::HashSet::new();

        // Group symbols by type for hierarchical organization
        let mut modules = Vec::new();
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut constructors = Vec::new();

        for symbol in &symbols {
            match symbol.kind {
                SymbolKind::MODULE => modules.push(symbol.clone()),
                SymbolKind::CLASS => types.push(symbol.clone()),
                SymbolKind::FUNCTION => functions.push(symbol.clone()),
                SymbolKind::CONSTRUCTOR => constructors.push(symbol.clone()),
                _ => {}
            }
        }

        // Add modules first (top-level)
        for module in modules {
            // Skip modules that are just file names or duplicates
            if module.name.contains(" exposing ") || processed_modules.contains(&module.name) {
                continue;
            }

            let doc_symbol = DocumentSymbol {
                name: module.name.clone(),
                detail: None,
                kind: module.kind,
                range: module.location.range,
                selection_range: module.location.range,
                children: None,
                tags: None,
                #[allow(deprecated)]
                #[allow(deprecated)]
                deprecated: Some(false),
            };
            document_symbols.push(doc_symbol);
            processed_modules.insert(module.name.clone());
        }

        // Sort types to process simple names first, then verbose ones
        types.sort_by(|a, b| {
            let a_is_verbose = a.name.starts_with("type ");
            let b_is_verbose = b.name.starts_with("type ");
            a_is_verbose.cmp(&b_is_verbose)
        });

        // Process types with smart deduplication
        for typ in types {
            // Extract clean type name
            let type_name = if typ.name.starts_with("type ") {
                // Extract type name from "type Foo = ..." or "type alias Foo = ..."
                if let Some(name_part) = typ.name.split_whitespace().nth(1) {
                    name_part
                        .split('=')
                        .next()
                        .unwrap_or(name_part)
                        .trim()
                        .to_string()
                } else {
                    continue;
                }
            } else {
                typ.name.clone()
            };

            // Skip if we already processed this type name
            if processed_types.contains(&type_name) {
                continue;
            }

            // Find constructors that belong to this type using container_name
            let type_constructors: Vec<DocumentSymbol> = constructors
                .iter()
                .filter(|c| {
                    // Use the container_name field to properly associate constructors with their parent type
                    c.container_name
                        .as_ref()
                        .map(|container| container == &type_name)
                        .unwrap_or(false)
                })
                .map(|c| DocumentSymbol {
                    name: c.name.clone(),
                    detail: c.type_signature.clone(),
                    kind: c.kind,
                    range: c.location.range,
                    selection_range: c.location.range,
                    children: None,
                    tags: None,
                    #[allow(deprecated)]
                    #[allow(deprecated)]
                    #[allow(deprecated)]
                    deprecated: Some(false),
                })
                .collect();

            let doc_symbol = DocumentSymbol {
                name: type_name.clone(),
                detail: typ.type_signature.clone(),
                kind: typ.kind,
                range: typ.location.range,
                selection_range: typ.location.range,
                children: if type_constructors.is_empty() {
                    None
                } else {
                    Some(type_constructors)
                },
                tags: None,
                #[allow(deprecated)]
                #[allow(deprecated)]
                deprecated: Some(false),
            };

            document_symbols.push(doc_symbol);
            processed_types.insert(type_name);
        }

        // Add functions (no deduplication needed as they're typically unique)
        for function in functions {
            let doc_symbol = DocumentSymbol {
                name: function.name.clone(),
                detail: function.type_signature.clone(),
                kind: function.kind,
                range: function.location.range,
                selection_range: function.location.range,
                children: None,
                tags: None,
                #[allow(deprecated)]
                #[allow(deprecated)]
                deprecated: Some(false),
            };
            document_symbols.push(doc_symbol);
        }

        // Sort by line number for consistent ordering
        document_symbols.sort_by(|a, b| a.range.start.line.cmp(&b.range.start.line));

        debug!(
            "Converted {} internal symbols to {} document symbols",
            symbols.len(),
            document_symbols.len()
        );

        document_symbols
    }

    /// Convert internal symbol to LSP SymbolInformation format
    fn convert_to_symbol_information(&self, symbol: GrenSymbol) -> SymbolInformation {
        let container_name = symbol.container_name.clone();

        SymbolInformation {
            name: symbol.name,
            kind: symbol.kind,
            location: symbol.location,
            container_name,
            tags: None,
            #[allow(deprecated)]
            deprecated: Some(false),
        }
    }

    /// Convert SymbolKind to CompletionItemKind
    fn symbol_kind_to_completion_kind(&self, symbol_kind: SymbolKind) -> CompletionItemKind {
        match symbol_kind {
            SymbolKind::FUNCTION => CompletionItemKind::FUNCTION,
            SymbolKind::VARIABLE => CompletionItemKind::VARIABLE,
            SymbolKind::CLASS => CompletionItemKind::CLASS,
            SymbolKind::INTERFACE => CompletionItemKind::INTERFACE,
            SymbolKind::MODULE => CompletionItemKind::MODULE,
            SymbolKind::PROPERTY => CompletionItemKind::PROPERTY,
            SymbolKind::ENUM => CompletionItemKind::ENUM,
            SymbolKind::CONSTRUCTOR => CompletionItemKind::CONSTRUCTOR,
            SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
            _ => CompletionItemKind::TEXT,
        }
    }

    /// Apply basic context filtering to determine if a symbol should be included in completion
    fn should_include_symbol_in_completion(
        &self,
        symbol: &GrenSymbol,
        _params: &CompletionParams,
    ) -> bool {
        // For now, implement basic filtering - can be enhanced later with more sophisticated context analysis

        // Skip very long or complex symbol names that are likely noise
        if symbol.name.len() > 100 {
            return false;
        }

        // Skip symbols that look like internal compiler artifacts
        if symbol.name.starts_with("_") || symbol.name.contains("$") {
            return false;
        }

        // Skip module declarations with exposing clauses (verbose forms)
        if symbol.kind == SymbolKind::MODULE && symbol.name.contains(" exposing ") {
            return false;
        }

        // Skip verbose type definitions in favor of clean type names
        if symbol.kind == SymbolKind::CLASS
            && symbol.name.starts_with("type ")
            && symbol.name.contains(" = ")
        {
            return false;
        }

        true
    }

    /// Find the symbol name at the given cursor position
    async fn find_symbol_at_position(
        &self,
        workspace: &gren_lsp_core::Workspace,
        uri: &Url,
        position: Position,
    ) -> Option<SymbolAtPosition> {
        // Get the document text
        let document = workspace.get_document_readonly(uri)?;
        let lines: Vec<&str> = document.text().lines().collect();

        // Check if position is valid
        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let char_pos = position.character as usize;

        if char_pos >= line.len() {
            return None;
        }

        // Find word boundaries around the cursor position
        let chars: Vec<char> = line.chars().collect();

        // Check if we're on a valid identifier character
        if char_pos >= chars.len() || !self.is_identifier_char(chars[char_pos]) {
            return None;
        }

        // Find the start of the identifier
        let mut start = char_pos;
        while start > 0 && self.is_identifier_char(chars[start - 1]) {
            start -= 1;
        }

        // Find the end of the identifier
        let mut end = char_pos;
        while end < chars.len() && self.is_identifier_char(chars[end]) {
            end += 1;
        }

        // Extract the identifier
        let identifier: String = chars[start..end].iter().collect();

        // Filter out keywords and single characters
        if identifier.len() < 2 || self.is_keyword(&identifier) {
            return None;
        }

        // Check if this is a qualified name like "Utils.isEmpty" or "Gren.Kernel.Bytes.flatten"
        // Look backwards to find the full qualified path
        let mut module_path = Vec::new();
        let mut search_start = start;

        // Keep looking backwards for module qualifiers
        while search_start > 0 && chars[search_start - 1] == '.' {
            // Find the module name before this dot
            let mut module_start = search_start - 1;
            while module_start > 0 && self.is_identifier_char(chars[module_start - 1]) {
                module_start -= 1;
            }

            if module_start < search_start - 1 {
                let module_name: String = chars[module_start..search_start - 1].iter().collect();
                module_path.insert(0, module_name); // Insert at beginning to maintain order
                search_start = module_start;
            } else {
                break;
            }
        }

        let symbol_info = if module_path.is_empty() {
            SymbolAtPosition {
                function_name: identifier,
                module_path: None,
            }
        } else {
            info!(
                "Found qualified reference: {}.{}",
                module_path.join("."),
                identifier
            );
            SymbolAtPosition {
                function_name: identifier,
                module_path: Some(module_path),
            }
        };

        Some(symbol_info)
    }

    /// Check if a character is part of an identifier
    fn is_identifier_char(&self, c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    /// Check if a string is a Gren keyword
    fn is_keyword(&self, word: &str) -> bool {
        matches!(
            word,
            "if" | "then"
                | "else"
                | "case"
                | "of"
                | "let"
                | "in"
                | "type"
                | "alias"
                | "module"
                | "import"
                | "exposing"
                | "as"
                | "where"
        )
    }

    /// Find a symbol in a specific module context
    async fn find_qualified_symbol(
        &self,
        workspace: &gren_lsp_core::Workspace,
        function_name: &str,
        module_path: &[String],
    ) -> anyhow::Result<Vec<gren_lsp_core::Symbol>> {
        info!(
            "Searching for '{}' in module '{}'",
            function_name,
            module_path.join(".")
        );

        // Check if this is a Gren.Kernel.* module - these are built-in and don't have user files
        if module_path.len() >= 2 && module_path[0] == "Gren" && module_path[1] == "Kernel" {
            info!("Detected Gren.Kernel.* module '{}' - these are built-in functions with no user-defined files", 
                  module_path.join("."));
            return Ok(Vec::new());
        }

        // First, try to find symbols with the function name
        let all_symbols = workspace.find_symbols(function_name)?;

        // Filter symbols that match the specified module path
        let qualified_symbols: Vec<_> = all_symbols
            .into_iter()
            .filter(|symbol| self.symbol_matches_module_path(symbol, module_path))
            .collect();

        // For qualified calls, we only return results if we have high confidence
        // Better to show nothing than the wrong thing
        if !qualified_symbols.is_empty() {
            info!(
                "Found {} qualified matches for '{}' in module '{}'",
                qualified_symbols.len(),
                function_name,
                module_path.join(".")
            );
            Ok(qualified_symbols)
        } else {
            info!("No qualified matches found for '{}' in module '{}' - returning empty to avoid incorrect results", 
                  function_name, module_path.join("."));
            Ok(Vec::new())
        }
    }

    /// Check if a symbol likely belongs to the specified module path
    /// This is highly conservative - we only return true when we have very strong confidence
    fn symbol_matches_module_path(
        &self,
        symbol: &gren_lsp_core::Symbol,
        module_path: &[String],
    ) -> bool {
        let file_path = symbol.location.uri.path();

        info!(
            "🔍 Checking if symbol '{}' in file '{}' matches module path '{}'",
            symbol.name,
            file_path,
            module_path.join(".")
        );

        // For qualified calls, we demand very high confidence in the match
        // It's better to show no results than incorrect ones

        if module_path.is_empty() {
            info!("❌ Module path is empty");
            return false;
        }

        let empty_string = String::new();
        let module_name = module_path.last().unwrap_or(&empty_string);

        if module_name.is_empty() {
            info!("❌ Module name is empty");
            return false;
        }

        // Pattern 1: Exact module file match - /path/to/ModuleName.gren
        let pattern1 = format!("/{}.gren", module_name);
        if file_path.ends_with(&pattern1) {
            info!("✅ Symbol matches exact module file pattern: {}", pattern1);
            return true;
        } else {
            info!(
                "❌ Pattern 1 failed: '{}' does not end with '{}'",
                file_path, pattern1
            );
        }

        // Pattern 2: Full hierarchical path - /path/to/Gren/Kernel/Bytes.gren
        let full_path = module_path.join("/");
        let pattern2 = format!("/{}.gren", full_path);
        if file_path.ends_with(&pattern2) {
            info!("✅ Symbol matches full hierarchical path: {}", pattern2);
            return true;
        } else {
            info!(
                "❌ Pattern 2 failed: '{}' does not end with '{}'",
                file_path, pattern2
            );
        }

        // Pattern 3: Check if the file path contains the complete module hierarchy
        // For Gren.Kernel.Bytes, we want files containing /Gren/Kernel/Bytes.gren
        if module_path.len() > 1 {
            let hierarchical_pattern = format!("/{}.gren", module_path.join("/"));
            if file_path.contains(&hierarchical_pattern) {
                info!(
                    "✅ Symbol matches hierarchical module pattern: {}",
                    hierarchical_pattern
                );
                return true;
            } else {
                info!(
                    "❌ Pattern 3 failed: '{}' does not contain '{}'",
                    file_path, hierarchical_pattern
                );
            }
        }

        // No match found - be conservative and reject
        info!(
            "❌ Symbol in '{}' does not match required module path '{}'",
            file_path,
            module_path.join(".")
        );
        false
    }

    /// Find an unqualified symbol by checking local symbols first, then imports only
    /// No workspace fallback - if it's not local or imported, it's a compile error
    async fn find_unqualified_symbol(
        &self,
        workspace: &gren_lsp_core::Workspace,
        file_uri: &Url,
        symbol_name: &str,
    ) -> anyhow::Result<Vec<gren_lsp_core::Symbol>> {
        info!(
            "Finding unqualified symbol '{}' in file '{}'",
            symbol_name, file_uri
        );

        // First, check for local symbols in the same file
        let local_symbols = workspace.get_file_symbols(file_uri)?;
        let local_matches: Vec<_> = local_symbols
            .into_iter()
            .filter(|symbol| {
                symbol.name == symbol_name &&
                // Prefer function definitions over other types for go-to-definition
                (symbol.kind == SymbolKind::FUNCTION ||
                 symbol.kind == SymbolKind::CLASS ||
                 symbol.kind == SymbolKind::CONSTRUCTOR)
            })
            .collect();

        if !local_matches.is_empty() {
            info!(
                "Found {} local symbols for '{}' in same file - using local definition",
                local_matches.len(),
                symbol_name
            );
            return Ok(local_matches);
        }

        // Next, parse imports from the current file
        let import_map = self.parse_imports(workspace, file_uri).await;

        // Check if this symbol is explicitly imported from a specific module
        if let Some(module_name) = import_map.get(symbol_name) {
            info!(
                "Symbol '{}' is imported from module '{}' - searching there only",
                symbol_name, module_name
            );

            // Search for the symbol in the specific imported module
            let all_symbols = workspace.find_symbols(symbol_name)?;
            let module_symbols: Vec<_> = all_symbols
                .into_iter()
                .filter(|symbol| {
                    // Check if the symbol is from the imported module
                    self.symbol_is_from_module(symbol, module_name)
                })
                .collect();

            if !module_symbols.is_empty() {
                info!(
                    "Found {} symbols from imported module '{}'",
                    module_symbols.len(),
                    module_name
                );
                return Ok(module_symbols);
            } else {
                info!(
                    "No symbols found in imported module '{}' - this might be a built-in",
                    module_name
                );
                return Ok(Vec::new());
            }
        }

        // If not found locally or in imports, return empty (don't do workspace search)
        // This would be a compile error in valid Gren code
        info!(
            "Symbol '{}' not found locally or in imports - would be compile error",
            symbol_name
        );
        Ok(Vec::new())
    }

    /// Parse import statements from a file to build a symbol -> module mapping
    async fn parse_imports(
        &self,
        workspace: &gren_lsp_core::Workspace,
        file_uri: &Url,
    ) -> std::collections::HashMap<String, String> {
        let mut import_map = std::collections::HashMap::new();

        // Get the document content
        if let Some(document) = workspace.get_document_readonly(file_uri) {
            let content = document.text();

            // Parse import statements line by line
            for line in content.lines() {
                let line = line.trim();

                // Look for import statements with exposing clause
                if line.starts_with("import ") && line.contains(" exposing ") {
                    if let Some((module_part, exposing_part)) = line.split_once(" exposing ") {
                        // Extract the actual module name, handling aliases
                        // e.g., "import Dedris.Motion as Motion" -> "Dedris.Motion"
                        let module_declaration = module_part.trim_start_matches("import ").trim();
                        let module_name = if module_declaration.contains(" as ") {
                            // Split on " as " and take the first part (actual module name)
                            module_declaration.split(" as ").next().unwrap().trim()
                        } else {
                            module_declaration
                        };

                        // Parse the exposing clause
                        let exposing_content = exposing_part.trim();

                        // Handle different exposing formats
                        if exposing_content == "(..)" {
                            // exposing (..) - exposes everything, we can't track specific symbols
                            info!("Found exposing (..) for module '{}' - cannot track specific imports", module_name);
                        } else if exposing_content.starts_with('(')
                            && exposing_content.ends_with(')')
                        {
                            // exposing (symbol1, symbol2, ...)
                            let symbols_str = &exposing_content[1..exposing_content.len() - 1];
                            for symbol in symbols_str.split(',') {
                                let symbol = symbol.trim();
                                if !symbol.is_empty() {
                                    info!(
                                        "Found import: '{}' from module '{}'",
                                        symbol, module_name
                                    );
                                    import_map.insert(symbol.to_string(), module_name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        import_map
    }

    /// Check if a symbol is from a specific module based on file path
    fn symbol_is_from_module(&self, symbol: &gren_lsp_core::Symbol, module_name: &str) -> bool {
        let file_path = symbol.location.uri.path();

        // Handle hierarchical module names (e.g., "Dedris.Motion" -> "/Dedris/Motion.gren")
        let hierarchical_pattern = format!("/{}.gren", module_name.replace(".", "/"));
        let simple_pattern = format!("/{}.gren", module_name);
        
        // Try hierarchical pattern first, then fall back to simple pattern
        let matches = file_path.ends_with(&hierarchical_pattern) || file_path.ends_with(&simple_pattern);

        if matches {
            let matched_pattern = if file_path.ends_with(&hierarchical_pattern) {
                &hierarchical_pattern
            } else {
                &simple_pattern
            };
            info!(
                "✅ Symbol '{}' matches module '{}' using pattern '{}' (file: {})",
                symbol.name, module_name, matched_pattern, file_path
            );
        } else {
            info!(
                "❌ Symbol '{}' does not match module '{}' - tried patterns '{}' and '{}' (file: {})",
                symbol.name, module_name, hierarchical_pattern, simple_pattern, file_path
            );
        }

        matches
    }

    /// Build hover content in Markdown format from symbol information
    async fn build_hover_content(
        &self,
        symbol: &gren_lsp_core::Symbol,
        symbol_info: &SymbolAtPosition,
        workspace: &gren_lsp_core::Workspace,
        supports_markdown: bool,
    ) -> String {
        let mut content = Vec::new();

        // Add symbol name and kind as header
        let kind_name = match symbol.kind {
            SymbolKind::FUNCTION => "function",
            SymbolKind::CLASS => "type",
            SymbolKind::CONSTRUCTOR => "constructor",
            SymbolKind::MODULE => "module",
            SymbolKind::VARIABLE => "variable",
            SymbolKind::CONSTANT => "constant",
            _ => "symbol",
        };

        if supports_markdown {
            // Check if this is a sum type first
            let is_sum_type = symbol.type_signature.as_ref()
                .map(|sig| sig.contains('=') && sig.contains('|'))
                .unwrap_or(false);

            if is_sum_type {
                // For sum types, skip the header and use the formatted version directly
                if let Some(type_signature) = &symbol.type_signature {
                    let formatted_signature = self.format_sum_type(type_signature);
                    content.push(formatted_signature);
                }
            } else {
                // For non-sum types, show the normal header
                if symbol.kind == SymbolKind::CONSTRUCTOR {
                    // Special case for constructors: show parent type
                    if let Some(parent_type) = &symbol.container_name {
                        content.push(format!(
                            "*{}* **{}** of type **{}**",
                            kind_name, symbol.name, parent_type
                        ));
                    } else {
                        content.push(format!("*{}* **{}**", kind_name, symbol.name));
                    }
                } else {
                    content.push(format!("*{}* **{}**", kind_name, symbol.name));
                }

                // Add formatted type signature in code block
                if let Some(type_signature) = &symbol.type_signature {
                    let formatted_signature = self.format_type_signature(type_signature);
                    content.push(format!("```gren\n{}\n```", formatted_signature));
                }
            }

            // Add Types section with clickable links (only for functions)
            if symbol.kind == SymbolKind::FUNCTION {
                if let Some(type_signature) = &symbol.type_signature {
                    let types_section = self.create_types_section(type_signature, workspace).await;
                    if !types_section.is_empty() {
                        content.push(types_section);
                    }
                }
            }

            // Add module information for qualified symbols
            if let Some(module_path) = &symbol_info.module_path {
                content.push(format!("*from module `{}`*", module_path.join(".")));
            } else if let Some(container) = &symbol.container_name {
                content.push(format!("*from module `{}`*", container));
            }

            // Add documentation if available
            if let Some(documentation) = &symbol.documentation {
                let doc = documentation.trim();
                if !doc.is_empty() {
                    content.push("---".to_string());
                    content.push(doc.to_string());
                }
            }


            content.join("\n\n")
        } else {
            // Plain text format - similar to JavaScript LSP
            let mut parts = Vec::new();

            // Add kind and symbol name (swapped order)
            if symbol.kind == SymbolKind::CONSTRUCTOR {
                // Special case for constructors: show parent type
                if let Some(parent_type) = &symbol.container_name {
                    parts.push(format!(
                        "{} {} of type {}",
                        kind_name, symbol.name, parent_type
                    ));
                } else {
                    parts.push(format!("{} {}", kind_name, symbol.name));
                }
            } else {
                parts.push(format!("{} {}", kind_name, symbol.name));
            }

            // Add type signature (plain, no clickable links for plaintext)
            if let Some(type_signature) = &symbol.type_signature {
                parts.push(type_signature.clone());
            }

            // Add module information
            if let Some(module_path) = &symbol_info.module_path {
                parts.push(format!("from module {}", module_path.join(".")));
            } else if let Some(container) = &symbol.container_name {
                parts.push(format!("from module {}", container));
            }

            parts.join("\n")
        }
    }

    /// Create a Types section with clickable links for types found in the signature
    async fn create_types_section(
        &self,
        type_signature: &str,
        workspace: &gren_lsp_core::Workspace,
    ) -> String {
        let mut type_links = Vec::new();
        let mut seen_types = std::collections::HashSet::new();

        // Extract type names using tree-sitter parsing
        match self
            .extract_type_names_from_signature_tree_sitter(type_signature)
            .await
        {
            Ok(type_names) => {
                for type_name in type_names {
                    // Skip constructors that shouldn't be clickable
                    if matches!(
                        type_name.as_str(),
                        "True" | "False" | "Just" | "Nothing" | "Active" | "Inactive"
                    ) {
                        continue;
                    }

                    // Skip if we already processed this type
                    if seen_types.contains(&type_name) {
                        continue;
                    }
                    seen_types.insert(type_name.clone());

                    // Try to find this type in the workspace
                    match workspace.find_symbols(&type_name) {
                        Ok(symbols) => {
                            let type_symbols: Vec<_> = symbols
                                .into_iter()
                                .filter(|s| {
                                    s.kind == SymbolKind::CLASS || s.kind == SymbolKind::CONSTRUCTOR
                                })
                                .collect();

                            if let Some(type_symbol) = type_symbols.first() {
                                // Create a clickable link using the file URI with line/column info
                                let file_uri = &type_symbol.location.uri;
                                let start_line = type_symbol.location.range.start.line;
                                let start_char = type_symbol.location.range.start.character;

                                // VS Code format: file:///path/to/file.ext#L<line>:<column>
                                let clickable_uri =
                                    format!("{}#L{}:{}", file_uri, start_line + 1, start_char);
                                let clickable_link = format!("[{}]({})", type_name, clickable_uri);

                                type_links.push(clickable_link);
                            }
                            // For built-in types or types not found, skip them
                        }
                        Err(_) => {
                            // Could not search, skip this type
                        }
                    }
                }
            }
            Err(_) => {
                // Failed to parse type signature, return empty
            }
        }

        if type_links.is_empty() {
            String::new()
        } else {
            format!("**Types:** {}", type_links.join(", "))
        }
    }

    /// Extract type names from a type signature using tree-sitter parsing
    async fn extract_type_names_from_signature_tree_sitter(
        &self,
        type_signature: &str,
    ) -> anyhow::Result<Vec<String>> {
        // Create a minimal Gren source with just the type expression to parse
        let gren_source = format!("dummy : {}", type_signature);

        // Parse using gren-lsp-core parser
        let mut parser = gren_lsp_core::Parser::new()?;
        if let Some(tree) = parser.parse(&gren_source)? {
            let language = gren_lsp_core::Parser::language();

            // Query to find all type references in a type expression
            let type_ref_query = tree_sitter::Query::new(
                language,
                r#"
                ; Type references (upper case identifiers in type positions)
                (type_ref
                    (upper_case_qid
                        (upper_case_identifier) @type.name))
                
                ; Simple type references
                (upper_case_identifier) @type.name
            "#,
            )?;

            let mut cursor = tree_sitter::QueryCursor::new();
            let source_bytes = gren_source.as_bytes();
            let matches = cursor.matches(&type_ref_query, tree.root_node(), source_bytes);

            let mut type_names = Vec::new();
            for m in matches {
                for capture in m.captures {
                    if let Ok(text) = capture.node.utf8_text(source_bytes) {
                        // Skip single-letter generic types (a, b, etc.)
                        if text.len() > 1 {
                            type_names.push(text.to_string());
                        }
                    }
                }
            }

            Ok(type_names)
        } else {
            Err(anyhow::anyhow!("Failed to parse type signature"))
        }
    }

    /// Get Gren language keyword completions
    fn get_keyword_completions(&self) -> Vec<CompletionItem> {
        let keywords = vec![
            ("if", "if condition then value else otherValue"),
            ("then", "if condition then value else otherValue"),
            ("else", "if condition then value else otherValue"),
            ("case", "case value of\\n    pattern -> result"),
            ("of", "case value of\\n    pattern -> result"),
            ("let", "let\\n    binding = value\\nin\\n    expression"),
            ("in", "let\\n    binding = value\\nin\\n    expression"),
            ("type", "type TypeName = Constructor"),
            ("alias", "type alias AliasName = Type"),
            ("module", "module ModuleName exposing (..)"),
            ("import", "import ModuleName"),
            ("exposing", "import ModuleName exposing (function)"),
            ("as", "import ModuleName as Alias"),
            ("where", "import ModuleName where"),
        ];

        keywords
            .into_iter()
            .map(|(keyword, snippet)| CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("Gren keyword: {}", keyword)),
                documentation: Some(Documentation::String(format!("Usage: {}", snippet))),
                insert_text: Some(keyword.to_string()),
                ..Default::default()
            })
            .collect()
    }

    /// Format type signatures for better readability, especially sum types
    fn format_type_signature(&self, signature: &str) -> String {
        // Check if this looks like a sum type (contains '|' and '=')
        if signature.contains('=') && signature.contains('|') {
            self.format_sum_type(signature)
        } else {
            // For other types, return as-is
            signature.to_string()
        }
    }

    /// Format sum types with clean constructor listing using markdown
    fn format_sum_type(&self, signature: &str) -> String {
        // Find the '=' to split the type name from constructors
        if let Some(eq_pos) = signature.find('=') {
            let (type_part, constructors_part) = signature.split_at(eq_pos);
            let type_part = type_part.trim();
            let constructors_part = constructors_part[1..].trim(); // Skip the '=' character

            // Extract just the type name (remove leading whitespace and "type " if present)
            let type_name = if type_part.starts_with("type ") {
                &type_part[5..]
            } else {
                type_part
            }.trim();

            // Split constructors by '|' and format them
            let constructors: Vec<&str> = constructors_part.split('|').collect();
            
            if constructors.len() > 1 || !constructors[0].trim().is_empty() {
                let mut formatted = String::new();
                formatted.push_str(&format!("type {}\n\n**Constructors**", type_name));
                
                for constructor in constructors.iter() {
                    let constructor = constructor.trim();
                    if !constructor.is_empty() {
                        formatted.push_str(&format!("\n- {}", constructor));
                    }
                }
                
                formatted
            } else {
                // Single constructor or malformed, return as-is
                signature.to_string()
            }
        } else {
            // No '=' found, return as-is
            signature.to_string()
        }
    }
}
