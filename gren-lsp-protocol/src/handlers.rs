#![allow(deprecated)]
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

    pub async fn find_references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        info!(
            "Find references requested at position {}:{} in file {} (include_declaration: {})",
            position.line, position.character, uri, include_declaration
        );

        let workspace = self.workspace.read().await;

        // Find the symbol at the cursor position
        let symbol_info = match self
            .find_symbol_at_position(&workspace, uri, position)
            .await
        {
            Some(symbol) => symbol,
            None => {
                info!(
                    "No symbol found at position {}:{}",
                    position.line, position.character
                );
                return Ok(None);
            }
        };

        info!(
            "Found symbol for references: {} (qualified: {:?})",
            symbol_info.function_name,
            symbol_info.module_path.as_deref().unwrap_or(&[])
        );

        // Find all references to this symbol across the workspace
        let references = match self
            .find_all_symbol_references(&workspace, &symbol_info, uri)
            .await
        {
            Ok(refs) => refs,
            Err(e) => {
                warn!(
                    "Failed to find references for symbol '{}': {}",
                    symbol_info.function_name, e
                );
                return Ok(Some(Vec::new()));
            }
        };

        if references.is_empty() {
            info!(
                "No references found for symbol '{}'",
                symbol_info.function_name
            );
            return Ok(Some(Vec::new()));
        }

        // Filter references based on include_declaration parameter
        let original_count = references.len();
        let filtered_references = if include_declaration {
            // Include all references and declarations
            references
        } else {
            // Exclude declarations, only include usage references
            self.filter_out_declarations(references, &symbol_info).await
        };

        info!(
            "Found {} references for symbol '{}' (filtered to {} based on include_declaration={})",
            original_count,
            symbol_info.function_name,
            filtered_references.len(),
            include_declaration
        );

        Ok(Some(filtered_references))
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
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        info!(
            "Code action requested for range {}:{}-{}:{} in file {}",
            params.range.start.line,
            params.range.start.character,
            params.range.end.line,
            params.range.end.character,
            params.text_document.uri
        );

        let mut actions = Vec::new();
        let workspace = self.workspace.read().await;

        // Handle quick fix actions (import suggestions for unresolved symbols)
        if params.context.only.is_none()
            || params
                .context
                .only
                .as_ref()
                .unwrap()
                .contains(&CodeActionKind::QUICKFIX)
        {
            if let Some(quickfix_actions) = self
                .generate_import_quickfix_actions(&workspace, &params)
                .await
            {
                actions.extend(quickfix_actions);
            }
        }

        // Handle source organize imports actions
        if params.context.only.is_none()
            || params
                .context
                .only
                .as_ref()
                .unwrap()
                .contains(&CodeActionKind::SOURCE_ORGANIZE_IMPORTS)
        {
            if let Some(organize_action) = self
                .generate_organize_imports_action(&workspace, &params.text_document.uri)
                .await
            {
                actions.push(CodeActionOrCommand::CodeAction(organize_action));
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    pub async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        info!(
            "Rename requested at position {}:{} with new name '{}'",
            params.text_document_position.position.line,
            params.text_document_position.position.character,
            params.new_name
        );

        let workspace = self.workspace.read().await;
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        // Validate the new name is a valid Gren identifier
        if !self.is_valid_gren_identifier(new_name) {
            warn!("Invalid identifier name for rename: '{}'", new_name);
            return Err(tower_lsp::jsonrpc::Error::invalid_params(format!(
                "'{}' is not a valid Gren identifier",
                new_name
            )));
        }

        // Find the symbol at the cursor position
        match self
            .find_symbol_at_position(&workspace, uri, position)
            .await
        {
            Some(symbol_info) => {
                info!("Found symbol for rename: {:?}", symbol_info);

                // Find all references to this symbol
                let references = match self
                    .find_all_symbol_references(&workspace, &symbol_info, uri)
                    .await
                {
                    Ok(refs) => refs,
                    Err(e) => {
                        warn!(
                            "Failed to find references for symbol '{}': {}",
                            symbol_info.function_name, e
                        );
                        return Ok(None);
                    }
                };

                if references.is_empty() {
                    info!(
                        "No references found for symbol '{}'",
                        symbol_info.function_name
                    );
                    return Ok(None);
                }

                // Generate workspace edit for all references
                let workspace_edit = self.generate_workspace_edit_for_rename(references, new_name);

                info!(
                    "Generated workspace edit for rename of '{}' to '{}' with {} changes",
                    symbol_info.function_name,
                    new_name,
                    workspace_edit
                        .document_changes
                        .as_ref()
                        .map(|changes| {
                            match changes {
                                DocumentChanges::Edits(edits) => edits.len(),
                                DocumentChanges::Operations(ops) => ops.len(),
                            }
                        })
                        .unwrap_or(0)
                );

                Ok(Some(workspace_edit))
            }
            None => {
                info!("No symbol found at rename position");
                Ok(None)
            }
        }
    }

    // Helper methods for rename functionality

    /// Validate that a string is a valid Gren identifier
    fn is_valid_gren_identifier(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        // Check if it's a reserved keyword
        if self.is_keyword(name) {
            return false;
        }

        let chars: Vec<char> = name.chars().collect();

        // First character must be letter or underscore, but not uppercase (unless it's a type)
        let first_char = chars[0];
        if !first_char.is_alphabetic() && first_char != '_' {
            return false;
        }

        // Rest of characters must be alphanumeric or underscore
        for &c in &chars[1..] {
            if !c.is_alphanumeric() && c != '_' {
                return false;
            }
        }

        true
    }

    /// Find all references to a symbol in the workspace
    async fn find_all_symbol_references(
        &self,
        workspace: &gren_lsp_core::Workspace,
        symbol_info: &SymbolAtPosition,
        current_uri: &Url,
    ) -> anyhow::Result<Vec<Location>> {
        let mut all_references = Vec::new();

        // Search for all occurrences of the symbol name
        let search_results = if let Some(module_path) = &symbol_info.module_path {
            // For qualified symbols, search both qualified and unqualified forms
            let qualified_symbols = self
                .find_qualified_symbol(workspace, &symbol_info.function_name, module_path)
                .await?;
            let unqualified_symbols = workspace.find_exact_symbols(&symbol_info.function_name)?;

            // Combine results and deduplicate
            let mut all_symbols = qualified_symbols;
            for symbol in unqualified_symbols {
                if !all_symbols
                    .iter()
                    .any(|existing| existing.location == symbol.location)
                {
                    all_symbols.push(symbol);
                }
            }
            all_symbols
        } else {
            // For unqualified symbols, use import-aware search
            self.find_unqualified_symbol(workspace, current_uri, &symbol_info.function_name)
                .await?
        };

        // Extract locations from symbols (these are definition locations)
        for symbol in search_results {
            all_references.push(symbol.location);
        }

        // Also search for textual references in all documents
        // This catches usage locations that might not be indexed as symbols
        let textual_references = self
            .find_textual_references(workspace, &symbol_info.function_name)
            .await?;
        all_references.extend(textual_references);

        // Remove duplicates by location
        all_references.sort_by(|a, b| {
            let uri_cmp = a.uri.cmp(&b.uri);
            if uri_cmp != std::cmp::Ordering::Equal {
                return uri_cmp;
            }
            let line_cmp = a.range.start.line.cmp(&b.range.start.line);
            if line_cmp != std::cmp::Ordering::Equal {
                return line_cmp;
            }
            a.range.start.character.cmp(&b.range.start.character)
        });
        all_references.dedup_by(|a, b| a.uri == b.uri && a.range == b.range);

        Ok(all_references)
    }

    /// Find textual references to a symbol name across all open documents
    async fn find_textual_references(
        &self,
        workspace: &gren_lsp_core::Workspace,
        symbol_name: &str,
    ) -> anyhow::Result<Vec<Location>> {
        let mut references = Vec::new();

        // Get all documents in the workspace
        // Note: This is a simplified approach - in a real implementation we'd want to
        // search through all files in the project, not just open documents
        for uri in workspace.get_open_document_uris() {
            if let Some(document) = workspace.get_document_readonly(&uri) {
                let text = document.text();
                let lines: Vec<&str> = text.lines().collect();

                for (line_idx, line) in lines.iter().enumerate() {
                    let mut char_idx = 0;
                    while let Some(match_start) = line[char_idx..].find(symbol_name) {
                        let absolute_start = char_idx + match_start;
                        let absolute_end = absolute_start + symbol_name.len();

                        // Check if this is a complete word match (not part of another identifier or module qualifier)
                        let is_complete_word = {
                            let before_ok = absolute_start == 0
                                || !self.is_identifier_char(
                                    line.chars().nth(absolute_start - 1).unwrap_or(' '),
                                );
                            let after_char = line.chars().nth(absolute_end).unwrap_or(' ');
                            let after_ok = absolute_end >= line.len()
                                || (!self.is_identifier_char(after_char) && after_char != '.');
                            before_ok && after_ok
                        };

                        // Check if this match is inside a comment
                        let is_in_comment = self.is_position_in_comment(line, absolute_start);

                        // Check if this match is inside an import statement
                        let is_in_import = self.is_position_in_import(line, absolute_start);

                        // Check if this match is inside a module declaration
                        let is_in_module_declaration = self
                            .is_position_in_module_declaration(
                                &uri,
                                line_idx as u32,
                                absolute_start as u32,
                            )
                            .await;

                        if is_complete_word
                            && !is_in_comment
                            && !is_in_import
                            && !is_in_module_declaration
                        {
                            references.push(Location {
                                uri: uri.clone(),
                                range: Range {
                                    start: Position {
                                        line: line_idx as u32,
                                        character: absolute_start as u32,
                                    },
                                    end: Position {
                                        line: line_idx as u32,
                                        character: absolute_end as u32,
                                    },
                                },
                            });
                        }

                        char_idx = absolute_start + 1; // Move past this match to find more
                    }
                }
            }
        }

        Ok(references)
    }

    /// Filter out declarations from references based on symbol information
    async fn filter_out_declarations(
        &self,
        references: Vec<Location>,
        symbol_info: &SymbolAtPosition,
    ) -> Vec<Location> {
        let mut filtered_references = Vec::new();

        for location in references {
            // Check if this location is a declaration by examining the AST context
            if !self.is_symbol_declaration(&location, symbol_info).await {
                filtered_references.push(location);
            }
        }

        filtered_references
    }

    /// Check if a location represents a symbol declaration rather than a usage
    /// For now, this uses a simple heuristic approach
    async fn is_symbol_declaration(
        &self,
        location: &Location,
        symbol_info: &SymbolAtPosition,
    ) -> bool {
        let workspace = self.workspace.read().await;

        // Get the document for this location
        let document = match workspace.get_document_readonly(&location.uri) {
            Some(doc) => doc,
            None => return false,
        };

        let source = document.text();
        let lines: Vec<&str> = source.lines().collect();

        // Get the line containing this location
        let line_index = location.range.start.line as usize;
        if line_index >= lines.len() {
            return false;
        }

        let line = lines[line_index];

        // Simple heuristic: check if this line looks like a declaration
        // Function declarations: "functionName : Type" or "functionName ="
        if line.trim_start().starts_with(&symbol_info.function_name) {
            // Check if it's followed by " : " (type annotation) or " =" (definition)
            let pattern = format!("{} :", symbol_info.function_name);
            let pattern2 = format!("{} =", symbol_info.function_name);
            if line.contains(&pattern) || line.contains(&pattern2) {
                return true;
            }
        }

        // Type declarations typically start with "type" keyword
        if line.trim_start().starts_with("type ") && line.contains(&symbol_info.function_name) {
            return true;
        }

        false
    }

    /// Generate a workspace edit for renaming all occurrences
    fn generate_workspace_edit_for_rename(
        &self,
        references: Vec<Location>,
        new_name: &str,
    ) -> WorkspaceEdit {
        use std::collections::HashMap;

        let mut changes_by_file: HashMap<Url, Vec<TextEdit>> = HashMap::new();

        // Group references by file
        for location in references {
            let edit = TextEdit {
                range: location.range,
                new_text: new_name.to_string(),
            };

            changes_by_file.entry(location.uri).or_default().push(edit);
        }

        // Sort edits within each file by position (reverse order for safe application)
        for edits in changes_by_file.values_mut() {
            edits.sort_by(|a, b| {
                let line_cmp = b.range.start.line.cmp(&a.range.start.line);
                if line_cmp != std::cmp::Ordering::Equal {
                    return line_cmp;
                }
                b.range.start.character.cmp(&a.range.start.character)
            });
        }

        // Convert to DocumentChanges format for better atomicity
        let document_changes: Vec<TextDocumentEdit> = changes_by_file
            .into_iter()
            .map(|(uri, edits)| TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri,
                    version: None, // Let the client handle versioning
                },
                edits: edits.into_iter().map(OneOf::Left).collect(),
            })
            .collect();

        WorkspaceEdit {
            changes: None, // Use document_changes instead for better atomicity
            document_changes: Some(DocumentChanges::Edits(document_changes)),
            change_annotations: None,
        }
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
        let all_symbols = workspace.find_exact_symbols(function_name)?;

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
            let all_symbols = workspace.find_exact_symbols(symbol_name)?;
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
        let matches =
            file_path.ends_with(&hierarchical_pattern) || file_path.ends_with(&simple_pattern);

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
            let is_sum_type = symbol
                .type_signature
                .as_ref()
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
                    match workspace.find_exact_symbols(&type_name) {
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
            let type_name = if let Some(stripped) = type_part.strip_prefix("type ") {
                stripped
            } else {
                type_part
            }
            .trim();

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

    /// Check if a position in a line is inside a comment
    /// Supports both single-line comments (--) and block comments ({- -})
    fn is_position_in_comment(&self, line: &str, position: usize) -> bool {
        // Check for single-line comment (-- comment)
        if let Some(comment_start) = line.find("--") {
            if position >= comment_start {
                return true;
            }
        }

        // Check for block comments ({- comment -})
        // This is a simplified check that only handles single-line block comments
        // A full implementation would need to track multi-line block comment state
        let mut in_block_comment = false;
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();

        while i < chars.len() {
            if i + 1 < chars.len() {
                // Check for block comment start {-
                if chars[i] == '{' && chars[i + 1] == '-' {
                    in_block_comment = true;
                    i += 2;
                    continue;
                }
                // Check for block comment end -}
                if chars[i] == '-' && chars[i + 1] == '}' && in_block_comment {
                    in_block_comment = false;
                    i += 2;
                    continue;
                }
            }

            // If we're at the target position and inside a block comment, return true
            if i == position && in_block_comment {
                return true;
            }

            i += 1;
        }

        // If we ended inside a block comment and the position is after the start, it's in a comment
        in_block_comment && position >= chars.len()
    }

    /// Check if a position in a line is within an import statement
    /// Import statements should not be considered as symbol references unless specifically searching for modules
    fn is_position_in_import(&self, line: &str, _position: usize) -> bool {
        let trimmed = line.trim_start();

        // Check for various import statement patterns
        // import Module
        // import Module as Alias
        // import Module exposing (..)
        // import Module exposing (symbol1, symbol2)
        trimmed.starts_with("import ")
    }

    /// Check if a position is within a module declaration using tree-sitter AST
    async fn is_position_in_module_declaration(
        &self,
        uri: &lsp_types::Url,
        line: u32,
        character: u32,
    ) -> bool {
        let workspace = self.workspace.read().await;

        // Get the document
        let document = match workspace.get_document_readonly(uri) {
            Some(doc) => doc,
            None => return false,
        };

        let content = document.text();

        // Parse the document using tree-sitter
        let mut parser = match gren_lsp_core::Parser::new() {
            Ok(parser) => parser,
            Err(_) => return false,
        };

        let tree = match parser.parse(content) {
            Ok(Some(tree)) => tree,
            _ => return false,
        };

        let language = gren_lsp_core::Parser::language();

        // Query to find module declarations and their export lists
        let module_query = match tree_sitter::Query::new(
            language,
            r#"
            ; Module declaration (the entire module declaration including exposing)
            (module_declaration) @module.declaration
            
            ; Exposing list specifically
            (exposing_list) @module.exports
            "#,
        ) {
            Ok(query) => query,
            Err(_) => return false,
        };

        let mut cursor = tree_sitter::QueryCursor::new();
        let source_bytes = content.as_bytes();
        let matches = cursor.matches(&module_query, tree.root_node(), source_bytes);

        let target_point = tree_sitter::Point {
            row: line as usize,
            column: character as usize,
        };

        for mat in matches {
            for capture in mat.captures {
                let node = capture.node;
                let start_point = node.start_position();
                let end_point = node.end_position();

                // Check if the target position is within this node
                if target_point >= start_point && target_point <= end_point {
                    return true;
                }
            }
        }

        false
    }

    /// Generate quick fix actions for import suggestions based on unresolved symbols
    async fn generate_import_quickfix_actions(
        &self,
        workspace: &gren_lsp_core::Workspace,
        params: &CodeActionParams,
    ) -> Option<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        // Check diagnostics for unresolved symbols
        for diagnostic in &params.context.diagnostics {
            if let Some(unresolved_symbol) = self.extract_unresolved_symbol(diagnostic) {
                // Find available symbols with matching names
                if let Ok(symbols) = workspace.find_symbols(&unresolved_symbol) {
                    for symbol in symbols {
                        if let Some(action) = self.create_import_action(
                            &symbol,
                            &params.text_document.uri,
                            diagnostic,
                        ) {
                            actions.push(CodeActionOrCommand::CodeAction(action));
                        }
                    }
                }
            }
        }

        if actions.is_empty() {
            None
        } else {
            Some(actions)
        }
    }

    /// Extract unresolved symbol name from diagnostic message
    fn extract_unresolved_symbol(&self, diagnostic: &Diagnostic) -> Option<String> {
        // Look for patterns indicating unresolved symbols
        let message = &diagnostic.message;

        // Common patterns for unresolved symbols in Gren compiler output
        if message.contains("not found") || message.contains("undefined") {
            // Extract symbol name using simple pattern matching
            // This would need to be refined based on actual Gren compiler messages
            if let Some(start) = message.find("`") {
                if let Some(end) = message[start + 1..].find("`") {
                    return Some(message[start + 1..start + 1 + end].to_string());
                }
            }
        }

        None
    }

    /// Create import code action for a symbol
    fn create_import_action(
        &self,
        symbol: &gren_lsp_core::Symbol,
        target_uri: &lsp_types::Url,
        diagnostic: &Diagnostic,
    ) -> Option<CodeAction> {
        // Extract module name from symbol location
        let module_name = self.extract_module_name_from_path(&symbol.location.uri)?;

        let title = format!("Import {} from {}", symbol.name, module_name);

        // Generate the import statement
        let import_statement = format!("import {} exposing ({})", module_name, symbol.name);

        // Find the position to insert the import (after module declaration, before other content)
        // For now, we'll use a simple approach and insert at the beginning of imports section
        let edit = TextEdit {
            range: Range {
                start: Position {
                    line: 1,
                    character: 0,
                }, // After module declaration
                end: Position {
                    line: 1,
                    character: 0,
                },
            },
            new_text: format!("{}\n", import_statement),
        };

        let mut changes = std::collections::HashMap::new();
        changes.insert(target_uri.clone(), vec![edit]);

        let workspace_edit = WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        })
    }

    /// Generate organize imports action
    async fn generate_organize_imports_action(
        &self,
        workspace: &gren_lsp_core::Workspace,
        uri: &lsp_types::Url,
    ) -> Option<CodeAction> {
        let document = workspace.get_document_readonly(uri)?;
        let content = document.text();

        // Parse the document to find import statements using tree-sitter
        let organized_imports = self.organize_imports_in_content(content).await?;

        if organized_imports == content {
            // No changes needed
            return None;
        }

        let edit = TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: content.lines().count() as u32,
                    character: 0,
                },
            },
            new_text: organized_imports,
        };

        let mut changes = std::collections::HashMap::new();
        changes.insert(uri.clone(), vec![edit]);

        let workspace_edit = WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeAction {
            title: "Organize imports".to_string(),
            kind: Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
            diagnostics: None,
            edit: Some(workspace_edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }

    /// Organize imports in file content using tree-sitter AST
    async fn organize_imports_in_content(&self, content: &str) -> Option<String> {
        // Parse the document using tree-sitter
        let mut parser = gren_lsp_core::Parser::new().ok()?;
        let tree = parser.parse(content).ok()??;
        let language = gren_lsp_core::Parser::language();

        // Query to find import statements
        let import_query = tree_sitter::Query::new(
            language,
            r#"
            (import_clause) @import
            "#,
        )
        .ok()?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let source_bytes = content.as_bytes();
        let matches = cursor.matches(&import_query, tree.root_node(), source_bytes);

        let mut import_statements = Vec::new();
        let mut import_ranges = Vec::new();

        // Extract all import statements and their ranges
        for mat in matches {
            for capture in mat.captures {
                let node = capture.node;
                let start_byte = node.start_byte();
                let end_byte = node.end_byte();
                let import_text = &content[start_byte..end_byte];

                import_statements.push(import_text.to_string());
                import_ranges.push((node.start_position(), node.end_position()));
            }
        }

        if import_statements.is_empty() {
            return None;
        }

        // Sort imports alphabetically by module name
        import_statements.sort_by(|a, b| {
            let module_a = self.extract_module_name_from_import(a);
            let module_b = self.extract_module_name_from_import(b);
            module_a.cmp(&module_b)
        });

        // Reconstruct the content with organized imports
        let mut result = content.to_string();

        // For simplicity, replace the import section with organized imports
        // In a production implementation, this would be more sophisticated
        let organized_import_section = import_statements.join("\n");

        // Find the range of all imports to replace
        if let (Some(first_range), Some(last_range)) = (import_ranges.first(), import_ranges.last())
        {
            let start_byte = first_range.0;
            let end_byte = last_range.1;

            let start_idx = content[..start_byte.row]
                .lines()
                .map(|l| l.len() + 1)
                .sum::<usize>();
            let end_idx = content[..end_byte.row]
                .lines()
                .map(|l| l.len() + 1)
                .sum::<usize>()
                + end_byte.column;

            result.replace_range(
                start_idx..end_idx.min(result.len()),
                &organized_import_section,
            );
        }

        Some(result)
    }

    /// Extract module name from import statement
    fn extract_module_name_from_import(&self, import_statement: &str) -> String {
        // Simple extraction: "import Module.Name" -> "Module.Name"
        let trimmed = import_statement.trim();
        if let Some(after_import) = trimmed.strip_prefix("import ") {
            if let Some(space_or_end) = after_import.find(|c: char| c.is_whitespace()) {
                after_import[..space_or_end].to_string()
            } else {
                after_import.trim().to_string()
            }
        } else {
            // Return the full string for malformed imports
            import_statement.to_string()
        }
    }

    /// Extract module name from file path
    fn extract_module_name_from_path(&self, uri: &lsp_types::Url) -> Option<String> {
        let path = uri.path();

        // Convert file path to module name
        // e.g., "/src/Data/List.gren" -> "Data.List"
        if let Some(file_name) = std::path::Path::new(path).file_stem() {
            if let Some(name_str) = file_name.to_str() {
                // Get parent directories to build module path
                let parent_path = std::path::Path::new(path).parent()?;
                let mut module_parts = Vec::new();

                // Walk up the directory structure to build module name
                for component in parent_path.components().rev() {
                    if let std::path::Component::Normal(part) = component {
                        if let Some(part_str) = part.to_str() {
                            if part_str == "src" {
                                break; // Stop at src directory
                            }
                            module_parts.insert(0, part_str.to_string());
                        }
                    }
                }

                module_parts.push(name_str.to_string());
                Some(module_parts.join("."))
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gren_lsp_core::Workspace;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_handlers() -> Handlers {
        let workspace = Arc::new(RwLock::new(Workspace::new().unwrap()));
        Handlers::new(workspace)
    }

    fn create_test_workspace() -> Workspace {
        Workspace::new().unwrap()
    }

    fn create_test_document(uri: &Url, content: &str) -> TextDocumentItem {
        TextDocumentItem {
            uri: uri.clone(),
            language_id: "gren".to_string(),
            version: 1,
            text: content.to_string(),
        }
    }

    #[test]
    fn test_is_valid_gren_identifier() {
        let handlers = create_test_handlers();

        // Valid identifiers
        assert!(handlers.is_valid_gren_identifier("myFunction"));
        assert!(handlers.is_valid_gren_identifier("_private"));
        assert!(handlers.is_valid_gren_identifier("value123"));
        assert!(handlers.is_valid_gren_identifier("my_function"));
        assert!(handlers.is_valid_gren_identifier("a"));

        // Invalid identifiers
        assert!(!handlers.is_valid_gren_identifier(""));
        assert!(!handlers.is_valid_gren_identifier("123invalid"));
        assert!(!handlers.is_valid_gren_identifier("my-function"));
        assert!(!handlers.is_valid_gren_identifier("my function"));
        assert!(!handlers.is_valid_gren_identifier("my@function"));

        // Gren keywords should be invalid
        assert!(!handlers.is_valid_gren_identifier("if"));
        assert!(!handlers.is_valid_gren_identifier("then"));
        assert!(!handlers.is_valid_gren_identifier("else"));
        assert!(!handlers.is_valid_gren_identifier("case"));
        assert!(!handlers.is_valid_gren_identifier("of"));
        assert!(!handlers.is_valid_gren_identifier("let"));
        assert!(!handlers.is_valid_gren_identifier("in"));
        assert!(!handlers.is_valid_gren_identifier("type"));
        assert!(!handlers.is_valid_gren_identifier("module"));
        assert!(!handlers.is_valid_gren_identifier("import"));
    }

    #[test]
    fn test_generate_workspace_edit_for_rename() {
        let handlers = create_test_handlers();

        let references = vec![
            Location {
                uri: Url::parse("file:///test/file1.gren").unwrap(),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 10,
                    },
                    end: Position {
                        line: 5,
                        character: 18,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///test/file1.gren").unwrap(),
                range: Range {
                    start: Position {
                        line: 10,
                        character: 5,
                    },
                    end: Position {
                        line: 10,
                        character: 13,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///test/file2.gren").unwrap(),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 8,
                    },
                },
            },
        ];

        let workspace_edit = handlers.generate_workspace_edit_for_rename(references, "newName");

        // Check that we have document changes
        assert!(workspace_edit.document_changes.is_some());

        if let Some(DocumentChanges::Edits(edits)) = workspace_edit.document_changes {
            assert_eq!(edits.len(), 2); // Two files should be edited

            // Check that edits are properly sorted (reverse order within files)
            for edit in &edits {
                for text_edit in &edit.edits {
                    if let OneOf::Left(text_edit) = text_edit {
                        assert_eq!(text_edit.new_text, "newName");
                    }
                }
            }
        } else {
            panic!("Expected DocumentChanges::Edits");
        }
    }

    #[tokio::test]
    async fn test_rename_with_invalid_identifier() {
        let handlers = create_test_handlers();

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse("file:///test/file.gren").unwrap(),
                },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            new_name: "123invalid".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = handlers.rename(params).await;
        assert!(result.is_err()); // Should return error for invalid identifier
    }

    #[tokio::test]
    async fn test_rename_with_keyword() {
        let handlers = create_test_handlers();

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse("file:///test/file.gren").unwrap(),
                },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            new_name: "if".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = handlers.rename(params).await;
        assert!(result.is_err()); // Should return error for keyword
    }

    #[tokio::test]
    async fn test_rename_no_symbol_at_position() {
        let handlers = create_test_handlers();

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse("file:///test/nonexistent.gren").unwrap(),
                },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            new_name: "validName".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let result = handlers.rename(params).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None); // Should return None when no symbol found
    }

    #[test]
    fn test_find_references_basic() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            // Create test document with a function
            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

testFunction : Int -> Int
testFunction x = x + 1

result = testFunction 42
"#;

            // Setup workspace with test document
            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            // Test find references at function definition
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position {
                        line: 2,
                        character: 0,
                    }, // "testFunction" definition
                },
                context: ReferenceContext {
                    include_declaration: true,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = handlers.find_references(params).await;
            assert!(result.is_ok());

            if let Ok(Some(references)) = result {
                // Should find at least the declaration and usage
                assert!(!references.is_empty(), "Should find references");

                // All references should be in the same file
                for reference in &references {
                    assert_eq!(reference.uri, uri);
                }
            }
        });
    }

    #[test]
    fn test_find_references_exclude_declaration() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

testFunction : Int -> Int
testFunction x = x + 1

result = testFunction 42
another = testFunction 100
"#;

            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            // Test find references excluding declaration
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position {
                        line: 3,
                        character: 0,
                    }, // "testFunction" definition
                },
                context: ReferenceContext {
                    include_declaration: false,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = handlers.find_references(params).await;
            assert!(result.is_ok());

            if let Ok(Some(references)) = result {
                // Should find usage references but not the declaration
                // Note: The exact count depends on our declaration detection heuristic
                assert!(!references.is_empty(), "Should find usage references");

                for reference in &references {
                    assert_eq!(reference.uri, uri);
                    // References should not be on the declaration line (line 3)
                    // This is a simple check - in practice, we'd verify the actual content
                }
            }
        });
    }

    #[test]
    fn test_find_references_no_symbol() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

-- Comment here
"#;

            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            // Test find references at a position with no symbol
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position: Position {
                        line: 2,
                        character: 0,
                    }, // Beginning of comment line
                },
                context: ReferenceContext {
                    include_declaration: true,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = handlers.find_references(params).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None, "Should return None for no symbol");
        });
    }

    #[test]
    fn test_is_symbol_declaration_heuristic() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

testFunction : Int -> Int
testFunction x = x + 1

result = testFunction 42
"#;

            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            let symbol_info = SymbolAtPosition {
                function_name: "testFunction".to_string(),
                module_path: None,
            };

            // Test declaration detection
            let declaration_location = Location {
                uri: uri.clone(),
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 12,
                    },
                },
            };

            let is_declaration = handlers
                .is_symbol_declaration(&declaration_location, &symbol_info)
                .await;
            assert!(
                is_declaration,
                "Should detect function type annotation as declaration"
            );

            // Test usage detection
            let usage_location = Location {
                uri,
                range: Range {
                    start: Position {
                        line: 5,
                        character: 9,
                    },
                    end: Position {
                        line: 5,
                        character: 21,
                    },
                },
            };

            let is_usage_declaration = handlers
                .is_symbol_declaration(&usage_location, &symbol_info)
                .await;
            assert!(
                !is_usage_declaration,
                "Should detect function usage as non-declaration"
            );
        });
    }

    #[test]
    fn test_filter_out_declarations() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

testFunction : Int -> Int
testFunction x = x + 1

result = testFunction 42
"#;

            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            let symbol_info = SymbolAtPosition {
                function_name: "testFunction".to_string(),
                module_path: None,
            };

            let all_references = vec![
                Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: 2,
                            character: 0,
                        },
                        end: Position {
                            line: 2,
                            character: 12,
                        },
                    },
                },
                Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: 5,
                            character: 9,
                        },
                        end: Position {
                            line: 5,
                            character: 21,
                        },
                    },
                },
            ];

            let filtered = handlers
                .filter_out_declarations(all_references, &symbol_info)
                .await;

            // Should filter out declarations, keeping only usage references
            assert!(!filtered.is_empty(), "Should have usage references");
            assert!(filtered.len() <= 2, "Should filter out some declarations");

            // Verify that remaining references are not declarations
            for reference in &filtered {
                let is_decl = handlers
                    .is_symbol_declaration(reference, &symbol_info)
                    .await;
                assert!(!is_decl, "Filtered references should not be declarations");
            }
        });
    }

    #[test]
    fn test_find_references_excludes_comments() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

testFunction : Int -> Int
testFunction x = x + 1

-- This comment mentions testFunction but should be ignored
result = testFunction 42  -- testFunction in comment should be ignored
{- testFunction in block comment should also be ignored -}
another = testFunction 100
"#;

            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            // Test find references including declarations
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position {
                        line: 2,
                        character: 0,
                    }, // "testFunction" definition
                },
                context: ReferenceContext {
                    include_declaration: true,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = handlers.find_references(params).await;
            assert!(result.is_ok());

            if let Ok(Some(references)) = result {
                assert!(!references.is_empty(), "Should find references");

                // Check that none of the references are in comments
                for reference in &references {
                    let line_num = reference.range.start.line as usize;
                    let char_pos = reference.range.start.character as usize;

                    // Get the document to check line content
                    let workspace = workspace.read().await;
                    if let Some(document) = workspace.get_document_readonly(&reference.uri) {
                        let lines: Vec<&str> = document.text().lines().collect();
                        if line_num < lines.len() {
                            let line = lines[line_num];
                            let is_in_comment = handlers.is_position_in_comment(line, char_pos);
                            assert!(
                                !is_in_comment,
                                "Reference at line {} char {} should not be in comment: '{}'",
                                line_num, char_pos, line
                            );
                        }
                    }
                }

                // We should find the declaration, definition, and usage references but not comment mentions
                // Exact count depends on implementation but should be reasonable (3-4 valid references)
                assert!(
                    references.len() >= 3,
                    "Should find at least 3 valid references (not in comments)"
                );
                assert!(
                    references.len() <= 5,
                    "Should not find too many references (comments should be filtered)"
                );
            }
        });
    }

    #[test]
    fn test_find_references_excludes_imports() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

import Dedris.Tetromino as Tetromino
import Other.Tetromino exposing (Tetromino)

type alias Tetromino =
    { type_ : Type
    , blocks : Array { row : Int , col : Int }
    }

createTetromino : Tetromino
createTetromino = { type_ = Type.I, blocks = [] }

useTetromino : Tetromino -> Int
useTetromino tetromino = Array.length tetromino.blocks
"#;

            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            // Test find references for Tetromino type alias
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position {
                        line: 5,
                        character: 11,
                    }, // "Tetromino" in type alias definition
                },
                context: ReferenceContext {
                    include_declaration: true,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = handlers.find_references(params).await;
            assert!(result.is_ok());

            if let Ok(Some(references)) = result {
                assert!(!references.is_empty(), "Should find references");

                // Check that none of the references are in import statements
                for reference in &references {
                    let line_num = reference.range.start.line as usize;
                    let char_pos = reference.range.start.character as usize;

                    // Get the document to check line content
                    let workspace = workspace.read().await;
                    if let Some(document) = workspace.get_document_readonly(&reference.uri) {
                        let lines: Vec<&str> = document.text().lines().collect();
                        if line_num < lines.len() {
                            let line = lines[line_num];
                            let is_in_import = handlers.is_position_in_import(line, char_pos);
                            assert!(
                                !is_in_import,
                                "Reference at line {} char {} should not be in import: '{}'",
                                line_num, char_pos, line
                            );
                        }
                    }
                }

                // We should find valid references (type alias declaration, function signatures, usage)
                // but NOT the import statement matches
                assert!(
                    references.len() >= 3,
                    "Should find at least 3 valid references (not in imports)"
                );
                assert!(
                    references.len() <= 6,
                    "Should not find too many references (imports should be filtered)"
                );
            }
        });
    }

    #[test]
    fn test_find_references_excludes_module_qualifiers() {
        use lsp_types::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let workspace = Arc::new(RwLock::new(create_test_workspace()));
            let handlers = Handlers::new(workspace.clone());

            let uri = Url::parse("file:///test/sample.gren").unwrap();
            let content = r#"module Sample exposing (..)

type alias Tetromino =
    { type_ : Type
    , blocks : Array { row : Int , col : Int }
    }

type Msg
    = ActiveMotion Motion
    | NewTmino Tetromino.Type
    | OtherMsg Tetromino

useFunction : Tetromino -> Int
useFunction tetromino = 42
"#;
            {
                let mut ws = workspace.write().await;
                let doc = create_test_document(&uri, content);
                ws.open_document(doc).unwrap();
            }

            // Test find references for Tetromino type alias
            let params = ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position { line: 2, character: 11 }, // "Tetromino" in type alias definition
                },
                context: ReferenceContext {
                    include_declaration: true,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let result = handlers.find_references(params).await;
            assert!(result.is_ok());

            if let Ok(Some(references)) = result {
                assert!(!references.is_empty(), "Should find references");
                // Check that none of the references are module qualifiers (followed by a dot)
                for reference in &references {
                    let line_num = reference.range.start.line as usize;
                    let char_pos = reference.range.end.character as usize; // Check the end position
                    // Get the document to check line content
                    let workspace = workspace.read().await;
                    if let Some(document) = workspace.get_document_readonly(&reference.uri) {
                        let lines: Vec<&str> = document.text().lines().collect();
                        if line_num < lines.len() {
                            let line = lines[line_num];
                            let next_char = line.chars().nth(char_pos).unwrap_or(' ');
                            assert_ne!(next_char, '.',
                                "Reference at line {} char {} should not be followed by '.' (module qualifier): '{}'",
                                line_num, char_pos, line);
                        }
                    }
                }
                // We should find valid references (type alias declaration, function parameter, return type)
                // but NOT the module qualifier in "Tetromino.Type"
                assert!(references.len() >= 3, "Should find at least 3 valid references (not module qualifiers)");
                assert!(references.len() <= 5, "Should not find too many references (module qualifiers should be filtered)");
            }
        });
    }

    #[tokio::test]
    async fn test_find_references_excludes_module_declarations() {
        let workspace = Arc::new(RwLock::new(create_test_workspace()));
        let handlers = Handlers::new(workspace.clone());
        let uri = Url::parse("file:///test/test_module_declarations.gren").unwrap();

        // Test content with module declaration, exports, and actual usage
        let content = r#"module Dedris.Tetromino exposing
    ( Tetromino
    , Type (..)
    , useFunction
    )

type alias Tetromino = { type_ : Type }

type Type
    = IBlock
    | JBlock

useFunction : Tetromino -> Int
useFunction tetromino = 42
"#;

        {
            let mut ws = workspace.write().await;
            let doc = create_test_document(&uri, content);
            ws.open_document(doc).unwrap();
        }

        // Test find references for Tetromino type alias
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 6,
                    character: 11,
                }, // "Tetromino" in type alias definition
            },
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handlers.find_references(params).await;
        assert!(result.is_ok());

        if let Ok(Some(references)) = result {
            assert!(!references.is_empty(), "Should find references");

            // Check that none of the references are in module declarations
            for reference in &references {
                let line_num = reference.range.start.line as usize;
                let char_pos = reference.range.start.character as usize;

                // Get the document to check line content
                let workspace = workspace.read().await;
                if let Some(document) = workspace.get_document_readonly(&reference.uri) {
                    let lines: Vec<&str> = document.text().lines().collect();
                    if line_num < lines.len() {
                        let line = lines[line_num];
                        let is_in_module_declaration = handlers
                            .is_position_in_module_declaration(
                                &reference.uri,
                                line_num as u32,
                                char_pos as u32,
                            )
                            .await;
                        assert!(
                            !is_in_module_declaration,
                            "Reference at line {} char {} should not be in module declaration: '{}'",
                            line_num, char_pos, line
                        );
                    }
                }
            }

            // We should find valid references (type alias declaration, function parameter, return type)
            // but NOT the module name or exports list
            assert!(
                references.len() >= 2,
                "Should find at least 2 valid references (not module declarations)"
            );
            assert!(
                references.len() <= 4,
                "Should not find too many references (module declarations should be filtered)"
            );
        }
    }

    #[test]
    fn test_import_detection() {
        let handlers = create_test_handlers();

        // Test various import statement patterns
        assert!(handlers.is_position_in_import("import Dedris.Tetromino as Tetromino", 15));
        assert!(handlers.is_position_in_import("import Module", 5));
        assert!(handlers.is_position_in_import("import Module as Alias", 10));
        assert!(handlers.is_position_in_import("import Module exposing (..)", 15));
        assert!(handlers.is_position_in_import("import Module exposing (symbol1, symbol2)", 20));
        assert!(handlers.is_position_in_import("    import Module", 10)); // with indentation

        // Test non-import statements
        assert!(!handlers.is_position_in_import("module MyModule exposing (..)", 10));
        assert!(!handlers.is_position_in_import("testFunction = import", 15)); // "import" as regular text
        assert!(!handlers.is_position_in_import("-- import Module", 10)); // import in comment
        assert!(!handlers.is_position_in_import("type Tetromino = { type_ : Type }", 15));
        assert!(!handlers.is_position_in_import("", 0));
    }

    #[test]
    fn test_comment_detection() {
        let handlers = create_test_handlers();

        // Test single-line comments
        assert!(handlers.is_position_in_comment("-- This is a comment with testFunction", 20));
        assert!(handlers.is_position_in_comment("someCode -- testFunction in comment", 25));
        assert!(!handlers.is_position_in_comment("testFunction -- comment after", 5));
        assert!(!handlers.is_position_in_comment("testFunction on next line", 5));

        // Test block comments (single line)
        assert!(handlers.is_position_in_comment("{- testFunction in block comment -}", 15));
        assert!(handlers.is_position_in_comment("code {- testFunction -} more code", 15));
        assert!(!handlers.is_position_in_comment("testFunction {- comment after -}", 5));

        // Test mixed scenarios
        assert!(!handlers.is_position_in_comment("testFunction = 42 -- normal code", 0));
        assert!(handlers.is_position_in_comment("code = 42 -- testFunction in comment", 25));

        // Test edge cases
        assert!(!handlers.is_position_in_comment("", 0));
        assert!(!handlers.is_position_in_comment("no comments here", 5));
        assert!(handlers.is_position_in_comment("-- testFunction", 3));
    }

    #[tokio::test]
    async fn test_module_declaration_detection() {
        let workspace = Arc::new(RwLock::new(create_test_workspace()));
        let handlers = Handlers::new(workspace.clone());

        // Test single-line module declaration
        let uri1 = Url::parse("file:///test/single_line_module.gren").unwrap();
        let content1 = "module Dedris.Tetromino exposing (..)";
        {
            let mut ws = workspace.write().await;
            let doc = create_test_document(&uri1, content1);
            ws.open_document(doc).unwrap();
        }
        assert!(
            handlers
                .is_position_in_module_declaration(&uri1, 0, 15)
                .await
        ); // "Tetromino" in module name
        assert!(
            handlers
                .is_position_in_module_declaration(&uri1, 0, 35)
                .await
        ); // inside export list

        // Test multi-line module declaration
        let uri2 = Url::parse("file:///test/multi_line_module.gren").unwrap();
        let content2 = r#"module Dedris.Tetromino exposing
    ( Tetromino
    , Type (..)
    )

type alias Tetromino = { type_ : Type }"#;
        {
            let mut ws = workspace.write().await;
            let doc = create_test_document(&uri2, content2);
            ws.open_document(doc).unwrap();
        }
        assert!(
            handlers
                .is_position_in_module_declaration(&uri2, 0, 15)
                .await
        ); // "Tetromino" in module name
        assert!(
            handlers
                .is_position_in_module_declaration(&uri2, 1, 8)
                .await
        ); // "Tetromino" in export list
        assert!(
            handlers
                .is_position_in_module_declaration(&uri2, 2, 8)
                .await
        ); // "Type" in export list
        assert!(
            !handlers
                .is_position_in_module_declaration(&uri2, 5, 15)
                .await
        ); // type declaration outside module

        // Test non-module content
        let uri3 = Url::parse("file:///test/no_module.gren").unwrap();
        let content3 = r#"-- This is not a module
testFunction = 42"#;
        {
            let mut ws = workspace.write().await;
            let doc = create_test_document(&uri3, content3);
            ws.open_document(doc).unwrap();
        }
        assert!(
            !handlers
                .is_position_in_module_declaration(&uri3, 0, 10)
                .await
        );
        assert!(
            !handlers
                .is_position_in_module_declaration(&uri3, 1, 5)
                .await
        );
    }

    #[test]
    fn test_word_boundary_detection() {
        let handlers = create_test_handlers();

        // Test the specific case that's failing
        let line = "setBlock";
        let symbol_name = "set";

        let match_start = line.find(symbol_name).unwrap(); // Should be 0
        let absolute_start = match_start;
        let absolute_end = absolute_start + symbol_name.len(); // Should be 3

        // Check if this is a complete word match (updated to match new logic)
        let before_ok = absolute_start == 0
            || !handlers.is_identifier_char(line.chars().nth(absolute_start - 1).unwrap_or(' '));
        let after_char = line.chars().nth(absolute_end).unwrap_or(' ');
        let after_ok = absolute_end >= line.len()
            || (!handlers.is_identifier_char(after_char) && after_char != '.');
        let is_complete_word = before_ok && after_ok;

        println!("Testing '{}' in '{}'", symbol_name, line);
        println!(
            "  absolute_start: {}, absolute_end: {}",
            absolute_start, absolute_end
        );
        println!(
            "  char at absolute_end: {:?}",
            line.chars().nth(absolute_end)
        );
        println!("  before_ok: {}, after_ok: {}", before_ok, after_ok);
        println!("  is_complete_word: {}", is_complete_word);

        // This should be false - "set" is not a complete word in "setBlock"
        assert!(
            !is_complete_word,
            "set should not be considered a complete word in setBlock"
        );

        // Test a case that should work
        let line2 = "use set here";
        let match_start2 = line2.find(symbol_name).unwrap(); // Should be 4
        let absolute_start2 = match_start2;
        let absolute_end2 = absolute_start2 + symbol_name.len(); // Should be 7

        let before_ok2 = absolute_start2 == 0
            || !handlers.is_identifier_char(line2.chars().nth(absolute_start2 - 1).unwrap_or(' '));
        let after_ok2 = absolute_end2 >= line2.len()
            || !handlers.is_identifier_char(line2.chars().nth(absolute_end2).unwrap_or(' '));
        let is_complete_word2 = before_ok2 && after_ok2;

        println!("Testing '{}' in '{}'", symbol_name, line2);
        println!(
            "  absolute_start: {}, absolute_end: {}",
            absolute_start2, absolute_end2
        );
        println!(
            "  char at absolute_end: {:?}",
            line2.chars().nth(absolute_end2)
        );
        println!("  before_ok: {}, after_ok: {}", before_ok2, after_ok2);
        println!("  is_complete_word: {}", is_complete_word2);

        // This should be true - "set" is a complete word in "use set here"
        assert!(
            is_complete_word2,
            "set should be considered a complete word in 'use set here'"
        );
    }

    #[test]
    fn test_exact_symbol_search() {
        use gren_lsp_core::{Symbol, SymbolIndex};
        use lsp_types::*;

        // Create a symbol index with test symbols
        let index = SymbolIndex::new().expect("Failed to create symbol index");
        let file_uri = Url::parse("file:///test.gren").expect("Invalid URI");

        // Add two symbols: "uniqueTest" and "uniqueTestBlock" to avoid conflicts with other tests
        let set_symbol = Symbol {
            name: "uniqueTest".to_string(),
            kind: SymbolKind::FUNCTION,
            location: Location::new(
                file_uri.clone(),
                Range::new(Position::new(1, 0), Position::new(1, 10)),
            ),
            container_name: None,
            type_signature: Some("a -> b -> a".to_string()),
            documentation: None,
        };

        let set_block_symbol = Symbol {
            name: "uniqueTestBlock".to_string(),
            kind: SymbolKind::FUNCTION,
            location: Location::new(
                file_uri.clone(),
                Range::new(Position::new(5, 0), Position::new(5, 15)),
            ),
            container_name: None,
            type_signature: Some("Block -> Block".to_string()),
            documentation: None,
        };

        // Index both symbols
        index
            .index_symbol(&set_symbol)
            .expect("Failed to index set symbol");
        index
            .index_symbol(&set_block_symbol)
            .expect("Failed to index setBlock symbol");

        // Test fuzzy search - should find both
        let fuzzy_results = index
            .find_symbol("uniqueTest")
            .expect("Failed to search symbols");
        assert_eq!(
            fuzzy_results.len(),
            2,
            "Fuzzy search should find both 'uniqueTest' and 'uniqueTestBlock'"
        );

        // Test exact search - should find only "uniqueTest"
        let exact_results = index
            .find_exact_symbol("uniqueTest")
            .expect("Failed to search exact symbols");
        assert_eq!(
            exact_results.len(),
            1,
            "Exact search should find only 'uniqueTest'"
        );
        assert_eq!(exact_results[0].name, "uniqueTest");

        // Clean up
        index
            .clear_file_symbols(file_uri.as_str())
            .expect("Failed to clear symbols");
    }

    #[test]
    fn test_extract_unresolved_symbol() {
        let handlers = create_test_handlers();

        // Test various diagnostic message patterns
        let diagnostic1 = Diagnostic {
            range: Range::default(),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "Variable `undefinedFunction` not found".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        assert_eq!(
            handlers.extract_unresolved_symbol(&diagnostic1),
            Some("undefinedFunction".to_string())
        );

        // Test diagnostic without backticks
        let diagnostic2 = Diagnostic {
            range: Range::default(),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "Cannot find symbol missingType".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        assert_eq!(handlers.extract_unresolved_symbol(&diagnostic2), None);

        // Test empty diagnostic
        let diagnostic3 = Diagnostic {
            range: Range::default(),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "Syntax error".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        assert_eq!(handlers.extract_unresolved_symbol(&diagnostic3), None);
    }

    #[test]
    fn test_extract_module_name_from_import() {
        let handlers = create_test_handlers();

        // Test simple import
        assert_eq!(
            handlers.extract_module_name_from_import("import Dict"),
            "Dict"
        );

        // Test qualified import
        assert_eq!(
            handlers.extract_module_name_from_import("import Data.List"),
            "Data.List"
        );

        // Test import with exposing
        assert_eq!(
            handlers.extract_module_name_from_import("import Array exposing (Array)"),
            "Array"
        );

        // Test import with alias
        assert_eq!(
            handlers.extract_module_name_from_import("import Json.Decode as Decode"),
            "Json.Decode"
        );

        // Test malformed import
        assert_eq!(
            handlers.extract_module_name_from_import("malformed import statement"),
            "malformed import statement"
        );
    }

    #[test]
    fn test_extract_module_name_from_path() {
        let handlers = create_test_handlers();

        // Test simple module path
        let uri1 = Url::parse("file:///src/Dict.gren").unwrap();
        assert_eq!(
            handlers.extract_module_name_from_path(&uri1),
            Some("Dict".to_string())
        );

        // Test nested module path
        let uri2 = Url::parse("file:///src/Data/List.gren").unwrap();
        assert_eq!(
            handlers.extract_module_name_from_path(&uri2),
            Some("Data.List".to_string())
        );

        // Test deeply nested module path
        let uri3 = Url::parse("file:///src/Json/Decode/Pipeline.gren").unwrap();
        assert_eq!(
            handlers.extract_module_name_from_path(&uri3),
            Some("Json.Decode.Pipeline".to_string())
        );

        // Test path without src directory
        let uri4 = Url::parse("file:///other/Module.gren").unwrap();
        assert_eq!(
            handlers.extract_module_name_from_path(&uri4),
            Some("other.Module".to_string())
        );

        // Test invalid path
        let uri5 = Url::parse("file:///").unwrap();
        assert_eq!(handlers.extract_module_name_from_path(&uri5), None);
    }

    #[tokio::test]
    async fn test_code_action_import_suggestions() {
        let workspace = Arc::new(RwLock::new(create_test_workspace()));
        let handlers = Handlers::new(workspace.clone());

        // Create test document with unresolved symbol
        let uri = Url::parse("file:///test/main.gren").unwrap();
        let content = r#"module Main exposing (..)

main = 
    undefinedFunction 42
"#;

        {
            let mut ws = workspace.write().await;
            let doc = create_test_document(&uri, content);
            ws.open_document(doc).unwrap();
        }

        // Create diagnostic for unresolved symbol
        let diagnostic = Diagnostic {
            range: Range {
                start: Position {
                    line: 3,
                    character: 4,
                },
                end: Position {
                    line: 3,
                    character: 21,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "Variable `undefinedFunction` not found".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range: diagnostic.range,
            context: CodeActionContext {
                diagnostics: vec![diagnostic],
                only: Some(vec![CodeActionKind::QUICKFIX]),
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handlers.code_action(params).await;
        assert!(result.is_ok());

        // For now, we expect None since no symbols are indexed
        // In a full implementation, this would return import suggestions
        let actions = result.unwrap();
        // Test passes if no errors occur - actual behavior depends on symbol indexing
        assert!(actions.is_none() || actions.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_organize_imports_action() {
        let workspace = Arc::new(RwLock::new(create_test_workspace()));
        let handlers = Handlers::new(workspace.clone());

        // Create test document with unorganized imports
        let uri = Url::parse("file:///test/main.gren").unwrap();
        let content = r#"module Main exposing (..)

import Json.Decode
import Array
import Dict exposing (Dict)
import Json.Encode as Encode

main = 42
"#;

        {
            let mut ws = workspace.write().await;
            let doc = create_test_document(&uri, content);
            ws.open_document(doc).unwrap();
        }

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 7,
                    character: 0,
                },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: Some(vec![CodeActionKind::SOURCE_ORGANIZE_IMPORTS]),
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let result = handlers.code_action(params).await;
        assert!(result.is_ok());

        // Test that organize imports action is available
        if let Some(actions) = result.unwrap() {
            assert!(!actions.is_empty());

            // Check that we have an organize imports action
            let organize_action = actions.iter().find(|action| {
                if let CodeActionOrCommand::CodeAction(ca) = action {
                    ca.title == "Organize imports"
                } else {
                    false
                }
            });

            assert!(organize_action.is_some());

            if let Some(CodeActionOrCommand::CodeAction(action)) = organize_action {
                assert_eq!(action.kind, Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS));
                assert!(action.edit.is_some());
            }
        }
    }

    #[tokio::test]
    async fn test_organize_imports_in_content() {
        let handlers = create_test_handlers();

        // Test content with unorganized imports
        let content = r#"module Main exposing (..)

import Json.Decode
import Array
import Dict exposing (Dict)

main = 42
"#;

        let result = handlers.organize_imports_in_content(content).await;

        // The organize_imports_in_content should process the imports
        // For now, we just test that it doesn't crash
        // In a full implementation, this would reorder the imports alphabetically
        if let Some(organized) = result {
            assert!(!organized.is_empty());
            assert!(organized.contains("import"));
            assert!(organized.contains("main = 42"));
        }
    }

    #[test]
    fn test_create_import_action() {
        let handlers = create_test_handlers();

        // Create a mock symbol
        let symbol_uri = Url::parse("file:///src/Utils.gren").unwrap();
        let symbol = gren_lsp_core::Symbol {
            name: "helperFunction".to_string(),
            kind: SymbolKind::FUNCTION,
            location: Location {
                uri: symbol_uri.clone(),
                range: Range {
                    start: Position {
                        line: 5,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 14,
                    },
                },
            },
            container_name: None,
            type_signature: None,
            documentation: None,
        };

        let target_uri = Url::parse("file:///src/Main.gren").unwrap();
        let diagnostic = Diagnostic {
            range: Range {
                start: Position {
                    line: 3,
                    character: 4,
                },
                end: Position {
                    line: 3,
                    character: 18,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("gren".to_string()),
            message: "Variable `helperFunction` not found".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let action = handlers.create_import_action(&symbol, &target_uri, &diagnostic);

        assert!(action.is_some());

        if let Some(import_action) = action {
            assert_eq!(import_action.title, "Import helperFunction from Utils");
            assert_eq!(import_action.kind, Some(CodeActionKind::QUICKFIX));
            assert!(import_action.edit.is_some());
            assert_eq!(import_action.is_preferred, Some(true));

            if let Some(edit) = import_action.edit {
                assert!(edit.changes.is_some());

                if let Some(changes) = edit.changes {
                    assert!(changes.contains_key(&target_uri));

                    if let Some(text_edits) = changes.get(&target_uri) {
                        assert!(!text_edits.is_empty());

                        let text_edit = &text_edits[0];
                        assert!(text_edit
                            .new_text
                            .contains("import Utils exposing (helperFunction)"));
                    }
                }
            }
        }
    }
}
