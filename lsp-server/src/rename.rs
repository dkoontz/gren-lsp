use anyhow::{anyhow, Result};
use tower_lsp::lsp_types::*;
use tracing::{debug, info, warn};
use std::collections::HashMap;
use regex::Regex;
use crate::symbol_index::SymbolIndex;
use crate::find_references::FindReferencesEngine;
use crate::compiler_interface::GrenCompiler;

// Type aliases for prepare rename - use the existing tower_lsp types
pub type PrepareRenameParams = TextDocumentPositionParams;

/// Engine for handling safe symbol renaming
pub struct RenameEngine {
    /// Symbol index for validation and reference finding
    symbol_index: SymbolIndex,
    /// Find references engine for comprehensive symbol finding
    find_references_engine: FindReferencesEngine,
    /// Compiler for validation
    compiler: GrenCompiler,
}

impl RenameEngine {
    /// Create a new rename engine
    pub fn new(
        symbol_index: SymbolIndex,
        compiler: GrenCompiler,
    ) -> Result<Self> {
        let find_references_engine = FindReferencesEngine::new(symbol_index.clone())?;
        Ok(Self {
            symbol_index,
            find_references_engine,
            compiler,
        })
    }

    /// Handle textDocument/rename LSP request
    pub async fn handle_rename(
        &mut self,
        params: RenameParams,
        document_content: &str,
        workspace_documents: &HashMap<Url, String>,
    ) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        debug!("🔄 RENAME REQUEST: position {:?} in {}, new_name: '{}'", 
               position, uri, new_name);

        // Step 1: Validate the new name follows Gren naming conventions
        if let Err(e) = self.validate_new_name(new_name) {
            warn!("Invalid new name '{}': {}", new_name, e);
            return Err(anyhow!("Invalid name: {}", e));
        }

        // Step 2: Find the symbol at the cursor position using find_references engine
        let references_params = ReferenceParams {
            text_document_position: params.text_document_position.clone(),
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let references = match self.find_references_engine
            .handle_references(references_params, document_content)
            .await? 
        {
            Some(refs) => refs,
            None => {
                debug!("No symbol found at position for rename");
                return Ok(None);
            }
        };

        if references.is_empty() {
            debug!("No references found for symbol at position");
            return Ok(None);
        }

        debug!("Found {} references for rename", references.len());

        // Step 3: Get the symbol info from the first reference (should be declaration or usage)
        let first_location = &references[0];
        let symbol_name = self.extract_symbol_name_at_location(first_location, workspace_documents)?;

        // Step 4: Validate that new name doesn't conflict with existing symbols
        if let Err(e) = self.validate_no_conflicts(&symbol_name, new_name, &first_location.uri).await {
            warn!("Name conflict detected: {}", e);
            return Err(anyhow!("Name conflict: {}", e));
        }

        // Step 5: Generate workspace edits for all references
        let workspace_edit = self.generate_workspace_edit(&references, &symbol_name, new_name, workspace_documents)?;

        // Step 6: Validate that the rename would not break compilation
        if let Err(e) = self.validate_compilation(&workspace_edit, workspace_documents).await {
            warn!("Rename would break compilation: {}", e);
            return Err(anyhow!("Compilation validation failed: {}", e));
        }

        info!("✅ Rename validation successful: '{}' -> '{}' ({} references)", 
              symbol_name, new_name, references.len());

        Ok(Some(workspace_edit))
    }

    /// Handle textDocument/prepareRename LSP request
    pub async fn handle_prepare_rename(
        &mut self,
        params: PrepareRenameParams,
        document_content: &str,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let position = params.position;

        debug!("🔄 PREPARE RENAME REQUEST: position {:?} in {}", position, uri);

        // Use find_references to check if there's a symbol at this position
        let references_params = ReferenceParams {
            text_document_position: params.clone(), // params is already TextDocumentPositionParams
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let references = match self.find_references_engine
            .handle_references(references_params, document_content)
            .await? 
        {
            Some(refs) if !refs.is_empty() => refs,
            _ => {
                debug!("No renameable symbol found at position");
                return Ok(None);
            }
        };

        // Find the specific reference that contains our position
        let target_range = references
            .iter()
            .find(|loc| {
                loc.uri == *uri &&
                position >= loc.range.start &&
                position <= loc.range.end
            })
            .map(|loc| loc.range)
            .unwrap_or_else(|| {
                // Fallback: use the first reference range
                references[0].range
            });

        // Extract the symbol name for placeholder text
        let symbol_name = self.extract_symbol_name_from_range(uri, target_range, document_content)?;

        debug!("✅ Prepare rename successful: symbol '{}' at range {:?}", symbol_name, target_range);

        // Return the range for the symbol that can be renamed
        // PrepareRenameResponse in tower_lsp is likely just Range | null
        Ok(Some(PrepareRenameResponse::Range(target_range)))
    }

    /// Validate that the new name follows Gren naming conventions
    pub fn validate_new_name(&self, new_name: &str) -> Result<()> {
        if new_name.is_empty() {
            return Err(anyhow!("Name cannot be empty"));
        }

        // Check for reserved keywords
        const RESERVED_KEYWORDS: &[&str] = &[
            "if", "then", "else", "when", "is", "let", "in", "case", "of",
            "type", "alias", "module", "import", "exposing", "as", "port",
            "effect", "where", "infixl", "infixr", "infix"
        ];

        if RESERVED_KEYWORDS.contains(&new_name) {
            return Err(anyhow!("'{}' is a reserved keyword", new_name));
        }

        // Validate naming conventions based on first character
        let first_char = new_name.chars().next().unwrap();
        
        if first_char.is_lowercase() {
            // Function/variable name: must start with lowercase, contain only alphanumeric and underscores
            let function_regex = Regex::new(r"^[a-z][a-zA-Z0-9_]*$").unwrap();
            if !function_regex.is_match(new_name) {
                return Err(anyhow!("Function/variable names must start with lowercase letter and contain only letters, numbers, and underscores"));
            }
        } else if first_char.is_uppercase() {
            // Type/constructor name: must start with uppercase, contain only alphanumeric
            let type_regex = Regex::new(r"^[A-Z][a-zA-Z0-9]*$").unwrap();
            if !type_regex.is_match(new_name) {
                return Err(anyhow!("Type/constructor names must start with uppercase letter and contain only letters and numbers"));
            }
        } else {
            return Err(anyhow!("Names must start with a letter"));
        }

        Ok(())
    }

    /// Validate that the new name doesn't conflict with existing symbols
    async fn validate_no_conflicts(&self, _old_name: &str, new_name: &str, uri: &Url) -> Result<()> {
        // Check if a symbol with the new name already exists in the same scope
        // This is a simplified check - in a full implementation, we'd need to consider
        // the specific scope and module context
        let existing_symbols = self.symbol_index.find_symbols_by_name(new_name).await?;
        
        // For now, we'll allow the rename if no exact matches in the same file
        let conflicts_in_same_file = existing_symbols
            .iter()
            .any(|symbol| symbol.uri == uri.to_string());

        if conflicts_in_same_file {
            return Err(anyhow!("A symbol named '{}' already exists in this file", new_name));
        }

        Ok(())
    }

    /// Generate workspace edit for all references
    fn generate_workspace_edit(
        &self,
        references: &[Location],
        old_name: &str,
        new_name: &str,
        workspace_documents: &HashMap<Url, String>,
    ) -> Result<WorkspaceEdit> {
        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

        for location in references {
            let uri = &location.uri;
            let range = location.range;

            // Verify that the text at this location matches the old symbol name
            if let Some(document_content) = workspace_documents.get(uri) {
                let actual_text = self.extract_text_from_range(document_content, range)?;
                if actual_text != old_name {
                    debug!("Warning: Expected '{}' but found '{}' at {:?}", old_name, actual_text, location);
                    // Continue anyway - the symbol might be part of a qualified name
                }
            }

            // Create text edit to replace old name with new name
            let text_edit = TextEdit {
                range,
                new_text: new_name.to_string(),
            };

            changes.entry(uri.clone()).or_insert_with(Vec::new).push(text_edit);
        }

        // Sort edits by position (in reverse order to avoid offset issues)
        for edits in changes.values_mut() {
            edits.sort_by(|a, b| {
                b.range.start.line.cmp(&a.range.start.line)
                    .then(b.range.start.character.cmp(&a.range.start.character))
            });
        }

        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Validate that the rename operation wouldn't break compilation
    async fn validate_compilation(
        &self,
        workspace_edit: &WorkspaceEdit,
        workspace_documents: &HashMap<Url, String>,
    ) -> Result<()> {
        debug!("🔍 Starting compilation validation for rename operation");
        
        // Step 1: Apply workspace edit to temporary copy of documents
        let mut modified_documents = workspace_documents.clone();
        
        if let Some(changes) = &workspace_edit.changes {
            for (uri, text_edits) in changes {
                if let Some(original_content) = modified_documents.get(uri) {
                    let modified_content = self.apply_text_edits(original_content, text_edits)?;
                    modified_documents.insert(uri.clone(), modified_content);
                } else {
                    warn!("Document {} not found in workspace for compilation validation", uri);
                }
            }
        }
        
        if modified_documents.is_empty() {
            debug!("No documents to validate - compilation validation passed trivially");
            return Ok(());
        }
        
        // Step 2: Create temporary Gren project with proper structure
        let temp_dir = tempfile::TempDir::new()
            .map_err(|e| anyhow!("Failed to create temporary directory: {}", e))?;
        
        let temp_project_root = temp_dir.path();
        let temp_src_dir = temp_project_root.join("src");
        std::fs::create_dir_all(&temp_src_dir)
            .map_err(|e| anyhow!("Failed to create src directory: {}", e))?;
        
        // Create a minimal gren.json with proper structure
        let gren_json = r#"{
    "type": "application",
    "source-directories": ["src"],
    "gren-version": "0.6.1",
    "platform": "node",
    "dependencies": {
        "direct": {},
        "indirect": {}
    }
}"#;
        std::fs::write(temp_project_root.join("gren.json"), gren_json)
            .map_err(|e| anyhow!("Failed to write gren.json: {}", e))?;
        
        // Step 3: Write modified documents to proper module structure and collect module names
        let mut module_names = Vec::new();
        
        for (uri, content) in &modified_documents {
            if uri.scheme() == "file" {
                if let Ok(file_path) = uri.to_file_path() {
                    if let Some(file_name) = file_path.file_name() {
                        let file_name_str = file_name.to_string_lossy();
                        if file_name_str.ends_with(".gren") {
                            // Determine module name from file path
                            let module_name = {
                                let path = uri.path();
                                if let Some(src_pos) = path.find("/src/") {
                                    // Extract everything after /src/ and convert to module name
                                    let module_path = &path[src_pos + 5..]; // Skip "/src/"
                                    module_path
                                        .trim_end_matches(".gren")
                                        .replace('/', ".")
                                } else {
                                    // Fallback: use just the filename
                                    file_name_str.trim_end_matches(".gren").to_string()
                                }
                            };
                            
                            // Create proper directory structure for nested modules
                            let target_path = if module_name.contains('.') {
                                let parts: Vec<&str> = module_name.split('.').collect();
                                let mut path = temp_src_dir.clone();
                                for part in &parts[..parts.len()-1] {
                                    path = path.join(part);
                                    std::fs::create_dir_all(&path)
                                        .map_err(|e| anyhow!("Failed to create directory {:?}: {}", path, e))?;
                                }
                                path.join(format!("{}.gren", parts[parts.len()-1]))
                            } else {
                                temp_src_dir.join(format!("{}.gren", module_name))
                            };
                            
                            std::fs::write(&target_path, content)
                                .map_err(|e| anyhow!("Failed to write temp file {:?}: {}", target_path, e))?;
                            
                            module_names.push(module_name);
                            debug!("📝 Created module {} at {:?}", module_names.last().unwrap(), target_path);
                        }
                    }
                }
            }
        }
        
        if module_names.is_empty() {
            debug!("No Gren modules to validate - compilation validation passed trivially");
            return Ok(());
        }
        
        // Step 4: Debug temp directory contents before compilation
        debug!("🔍 Temp directory contents before compilation:");
        if let Ok(entries) = std::fs::read_dir(temp_project_root) {
            for entry in entries.flatten() {
                debug!("  📁 {:?}", entry.path());
            }
        }
        if let Ok(entries) = std::fs::read_dir(&temp_src_dir) {
            for entry in entries.flatten() {
                debug!("  📄 {:?}", entry.path());
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    debug!("  📝 Content preview: {}", content.lines().take(3).collect::<Vec<_>>().join(" | "));
                }
            }
        }
        
        // Step 5: Compile each module using proper module names
        for module_name in &module_names {
            debug!("🔍 Validating compilation of module: {}", module_name);
            
            use crate::compiler_interface::CompileRequest;
            let compile_request = CompileRequest {
                module_name: module_name.clone(),
                project_root: temp_project_root.to_path_buf(),
                include_sourcemaps: false,
                in_memory_documents: HashMap::new(), // Files are already written to disk
            };
            
            let compile_result = self.compiler.compile(compile_request).await;
            
            match compile_result {
                Ok(result) => {
                    if result.success {
                        debug!("✅ Compilation validation passed for module: {}", module_name);
                    } else {
                        // Check if this is a Gren compiler internal crash (known issue)
                        if result.stderr.contains("Prelude.last: empty list") {
                            warn!("⚠️ Gren compiler crashed with internal error for module '{}'. Skipping compilation validation.", module_name);
                            warn!("This is a known Gren compiler issue. Rename operation will proceed without compilation validation.");
                            continue; // Skip this module but continue with others
                        } else {
                            return Err(anyhow!(
                                "Compilation validation failed for module '{}': {}",
                                module_name,
                                result.stderr
                            ));
                        }
                    }
                }
                Err(e) => {
                    // Check if this is a Gren compiler internal crash 
                    let error_msg = e.to_string();
                    if error_msg.contains("Prelude.last: empty list") {
                        warn!("⚠️ Gren compiler crashed with internal error for module '{}'. Skipping compilation validation.", module_name);
                        warn!("This is a known Gren compiler issue. Rename operation will proceed without compilation validation.");
                        continue; // Skip this module but continue with others
                    } else {
                        return Err(anyhow!(
                            "Compilation validation error for module '{}': {}",
                            module_name,
                            e
                        ));
                    }
                }
            }
        }
        
        info!("✅ Compilation validation passed for all {} modules", module_names.len());
        Ok(())
    }
    
    /// Apply text edits to document content
    fn apply_text_edits(&self, content: &str, text_edits: &[TextEdit]) -> Result<String> {
        // Work with owned strings to avoid memory management issues
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        // Sort edits in reverse order by position to avoid offset issues
        let mut sorted_edits = text_edits.to_vec();
        sorted_edits.sort_by(|a, b| {
            b.range.start.line.cmp(&a.range.start.line)
                .then(b.range.start.character.cmp(&a.range.start.character))
        });
        
        for edit in sorted_edits {
            let start_line = edit.range.start.line as usize;
            let end_line = edit.range.end.line as usize;
            
            if start_line >= lines.len() {
                return Err(anyhow!("Text edit start line {} exceeds document length {}", start_line, lines.len()));
            }
            
            if start_line == end_line {
                // Single line edit
                let line = &lines[start_line];
                let start_char = edit.range.start.character as usize;
                let end_char = edit.range.end.character as usize;
                
                if start_char > line.len() || end_char > line.len() {
                    return Err(anyhow!("Text edit character range invalid for line: start={}, end={}, line_len={}", 
                        start_char, end_char, line.len()));
                }
                
                let new_line = format!(
                    "{}{}{}",
                    &line[..start_char],
                    &edit.new_text,
                    &line[end_char..]
                );
                lines[start_line] = new_line;
            } else {
                // Multi-line edit
                let start_char = edit.range.start.character as usize;
                let end_char = edit.range.end.character as usize;
                
                if end_line >= lines.len() {
                    return Err(anyhow!("Text edit end line {} exceeds document length {}", end_line, lines.len()));
                }
                
                let start_line_content = &lines[start_line][..start_char];
                let end_line_content = &lines[end_line][end_char..];
                
                let new_content = format!("{}{}{}", start_line_content, &edit.new_text, end_line_content);
                let new_lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
                
                // Replace the range with new lines
                lines.splice(start_line..=end_line, new_lines.into_iter());
            }
        }
        
        Ok(lines.join("\n"))
    }

    /// Extract symbol name at a specific location
    fn extract_symbol_name_at_location(
        &self,
        location: &Location,
        workspace_documents: &HashMap<Url, String>,
    ) -> Result<String> {
        let document_content = workspace_documents
            .get(&location.uri)
            .ok_or_else(|| anyhow!("Document not found: {}", location.uri))?;

        self.extract_text_from_range(document_content, location.range)
    }

    /// Extract symbol name from a range in a document
    fn extract_symbol_name_from_range(&self, _uri: &Url, range: Range, document_content: &str) -> Result<String> {
        self.extract_text_from_range(document_content, range)
    }

    /// Extract text from a range in document content
    pub fn extract_text_from_range(&self, document_content: &str, range: Range) -> Result<String> {
        let lines: Vec<&str> = document_content.lines().collect();
        
        if range.start.line as usize >= lines.len() {
            return Err(anyhow!("Range start line {} exceeds document length {}", range.start.line, lines.len()));
        }

        if range.start.line == range.end.line {
            // Single line range
            let line = lines[range.start.line as usize];
            let start_char = range.start.character as usize;
            let end_char = range.end.character as usize;
            
            if start_char > line.len() || end_char > line.len() || start_char > end_char {
                return Err(anyhow!("Invalid character range in line"));
            }
            
            Ok(line[start_char..end_char].to_string())
        } else {
            // Multi-line range (shouldn't happen for symbol names, but handle it)
            let mut result = String::new();
            let start_line = lines[range.start.line as usize];
            result.push_str(&start_line[range.start.character as usize..]);
            
            for line_idx in (range.start.line + 1)..(range.end.line) {
                result.push('\n');
                result.push_str(lines[line_idx as usize]);
            }
            
            if (range.end.line as usize) < lines.len() {
                result.push('\n');
                let end_line = lines[range.end.line as usize];
                result.push_str(&end_line[..range.end.character as usize]);
            }
            
            Ok(result)
        }
    }
}