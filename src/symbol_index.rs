use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, SqlitePool};
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::*;
use tracing::{debug, info};

/// Represents a symbol in the index
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Symbol {
    pub id: Option<i64>,
    pub name: String,
    pub kind: i32, // Maps to SymbolKind as integer
    pub uri: String,
    pub range_start_line: i32,
    pub range_start_char: i32,
    pub range_end_line: i32,
    pub range_end_char: i32,
    pub container: Option<String>,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub created_at: Option<String>,
}

impl Symbol {
    /// Create a new symbol
    pub fn new(
        name: String,
        kind: SymbolKind,
        uri: &Url,
        range: Range,
        container: Option<String>,
        signature: Option<String>,
        documentation: Option<String>,
    ) -> Self {
        Self {
            id: None,
            name,
            kind: symbol_kind_to_i32(kind),
            uri: uri.to_string(),
            range_start_line: range.start.line as i32,
            range_start_char: range.start.character as i32,
            range_end_line: range.end.line as i32,
            range_end_char: range.end.character as i32,
            container,
            signature,
            documentation,
            created_at: None,
        }
    }

    /// Convert to LSP SymbolInformation
    pub fn to_symbol_information(&self) -> Result<SymbolInformation> {
        let uri = Url::parse(&self.uri)
            .map_err(|e| anyhow!("Invalid URI in symbol {}: {}", self.name, e))?;
        
        let range = Range {
            start: Position {
                line: self.range_start_line as u32,
                character: self.range_start_char as u32,
            },
            end: Position {
                line: self.range_end_line as u32,
                character: self.range_end_char as u32,
            },
        };

        Ok(SymbolInformation {
            name: self.name.clone(),
            kind: i32_to_symbol_kind(self.kind),
            location: Location { uri, range },
            container_name: self.container.clone(),
            tags: None,
            deprecated: None,
        })
    }

    /// Convert to LSP DocumentSymbol
    pub fn to_document_symbol(&self) -> Result<DocumentSymbol> {
        let range = Range {
            start: Position {
                line: self.range_start_line as u32,
                character: self.range_start_char as u32,
            },
            end: Position {
                line: self.range_end_line as u32,
                character: self.range_end_char as u32,
            },
        };

        Ok(DocumentSymbol {
            name: self.name.clone(),
            detail: self.signature.clone(),
            kind: i32_to_symbol_kind(self.kind),
            range,
            selection_range: range, // Same as range for now
            children: None,
            tags: None,
            deprecated: None,
        })
    }
}

/// Manages the SQLite-based symbol index
pub struct SymbolIndex {
    pool: Pool<Sqlite>,
    workspace_root: PathBuf,
}

impl SymbolIndex {
    /// Create a new symbol index
    pub async fn new(database_path: &Path, workspace_root: PathBuf) -> Result<Self> {
        // Ensure the database directory exists
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create database directory: {}", e))?;
        }

        let database_url = format!("sqlite:{}", database_path.display());
        info!("Initializing symbol index database at {}", database_url);

        let pool = SqlitePool::connect(&database_url).await
            .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

        let index = Self {
            pool,
            workspace_root,
        };

        // Initialize database schema
        index.initialize_schema().await?;

        info!("Symbol index initialized successfully");
        Ok(index)
    }

    /// Initialize the database schema
    pub async fn initialize_schema(&self) -> Result<()> {
        debug!("Initializing database schema");

        // Create symbols table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                kind INTEGER NOT NULL,
                uri TEXT NOT NULL,
                range_start_line INTEGER NOT NULL,
                range_start_char INTEGER NOT NULL,
                range_end_line INTEGER NOT NULL,
                range_end_char INTEGER NOT NULL,
                container TEXT,
                signature TEXT,
                documentation TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to create symbols table: {}", e))?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create name index: {}", e))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_uri ON symbols(uri)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create uri index: {}", e))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create kind index: {}", e))?;

        // Create imports table for cross-module resolution
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS imports (
                id INTEGER PRIMARY KEY,
                source_uri TEXT NOT NULL,
                imported_module TEXT NOT NULL,
                imported_symbols TEXT, -- JSON array of imported symbol names
                alias_name TEXT,
                exposing_all BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to create imports table: {}", e))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_imports_source_uri ON imports(source_uri)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create imports source_uri index: {}", e))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_imports_module ON imports(imported_module)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create imports module index: {}", e))?;

        debug!("Database schema initialized successfully");
        Ok(())
    }

    /// Add a symbol to the index
    pub async fn add_symbol(&self, symbol: &Symbol) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO symbols (name, kind, uri, range_start_line, range_start_char, 
                               range_end_line, range_end_char, container, signature, documentation)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&symbol.name)
        .bind(symbol.kind)
        .bind(&symbol.uri)
        .bind(symbol.range_start_line)
        .bind(symbol.range_start_char)
        .bind(symbol.range_end_line)
        .bind(symbol.range_end_char)
        .bind(&symbol.container)
        .bind(&symbol.signature)
        .bind(&symbol.documentation)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to insert symbol '{}': {}", symbol.name, e))?;

        Ok(result.last_insert_rowid())
    }

    /// Add multiple symbols efficiently
    pub async fn add_symbols(&self, symbols: &[Symbol]) -> Result<()> {
        if symbols.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await
            .map_err(|e| anyhow!("Failed to begin transaction: {}", e))?;

        for symbol in symbols {
            sqlx::query(
                r#"
                INSERT INTO symbols (name, kind, uri, range_start_line, range_start_char, 
                                   range_end_line, range_end_char, container, signature, documentation)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )
            .bind(&symbol.name)
            .bind(symbol.kind)
            .bind(&symbol.uri)
            .bind(symbol.range_start_line)
            .bind(symbol.range_start_char)
            .bind(symbol.range_end_line)
            .bind(symbol.range_end_char)
            .bind(&symbol.container)
            .bind(&symbol.signature)
            .bind(&symbol.documentation)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Failed to insert symbol '{}': {}", symbol.name, e))?;
        }

        tx.commit().await
            .map_err(|e| anyhow!("Failed to commit symbol batch: {}", e))?;

        debug!("Added {} symbols to index", symbols.len());
        Ok(())
    }

    /// Find symbols by name
    pub async fn find_symbols_by_name(&self, name: &str) -> Result<Vec<Symbol>> {
        let symbols = sqlx::query_as::<_, Symbol>(
            "SELECT * FROM symbols WHERE name = ?1 ORDER BY created_at DESC"
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to query symbols by name '{}': {}", name, e))?;

        Ok(symbols)
    }

    /// Find symbols by partial name match (for completion)
    pub async fn find_symbols_by_prefix(&self, prefix: &str, limit: i32) -> Result<Vec<Symbol>> {
        let pattern = format!("{}%", prefix);
        let symbols = sqlx::query_as::<_, Symbol>(
            "SELECT * FROM symbols WHERE name LIKE ?1 ORDER BY name LIMIT ?2"
        )
        .bind(pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to query symbols by prefix '{}': {}", prefix, e))?;

        Ok(symbols)
    }

    /// Find symbols in a specific file
    pub async fn find_symbols_in_file(&self, uri: &Url) -> Result<Vec<Symbol>> {
        let symbols = sqlx::query_as::<_, Symbol>(
            "SELECT * FROM symbols WHERE uri = ?1 ORDER BY range_start_line, range_start_char"
        )
        .bind(uri.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to query symbols in file '{}': {}", uri, e))?;

        Ok(symbols)
    }

    /// Find symbols of a specific kind
    pub async fn find_symbols_by_kind(&self, kind: SymbolKind) -> Result<Vec<Symbol>> {
        let symbols = sqlx::query_as::<_, Symbol>(
            "SELECT * FROM symbols WHERE kind = ?1 ORDER BY name"
        )
        .bind(symbol_kind_to_i32(kind))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to query symbols by kind: {}", e))?;

        Ok(symbols)
    }

    /// Remove all symbols for a specific file (for incremental updates)
    pub async fn remove_symbols_for_file(&self, uri: &Url) -> Result<u64> {
        let result = sqlx::query("DELETE FROM symbols WHERE uri = ?1")
            .bind(uri.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to remove symbols for file '{}': {}", uri, e))?;

        debug!("Removed {} symbols for file {}", result.rows_affected(), uri);
        Ok(result.rows_affected())
    }

    /// Index all symbols from file content using tree-sitter parsing
    pub async fn index_file(&self, uri: &Url, content: &str) -> Result<()> {
        use crate::tree_sitter_queries::GrenQueryEngine;
        use crate::gren_language;
        
        debug!("Indexing symbols for file: {}", uri);
        
        // Parse the file with tree-sitter
        let language = gren_language::language()?;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language)?;
        
        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse file: {}", uri))?;
            
        // Create query engine and extract symbols
        let query_engine = GrenQueryEngine::new()?;
        let symbols = query_engine.extract_symbols(uri, &tree, content)?;
        
        // Update symbols for this file
        self.update_symbols_for_file(uri, &symbols).await?;
        
        // Extract and store import information
        let imports = query_engine.extract_imports(uri, &tree, content)?;
        self.update_imports_for_file(uri, &imports).await?;
        
        debug!("Indexed file: {} - added {} symbols and {} imports", 
               uri, symbols.len(), imports.len());
        
        Ok(())
    }

    /// Update symbols for a file (remove old, add new)
    pub async fn update_symbols_for_file(&self, uri: &Url, symbols: &[Symbol]) -> Result<()> {
        let mut tx = self.pool.begin().await
            .map_err(|e| anyhow!("Failed to begin transaction: {}", e))?;

        // Remove existing symbols for this file
        sqlx::query("DELETE FROM symbols WHERE uri = ?1")
            .bind(uri.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Failed to remove old symbols for file '{}': {}", uri, e))?;

        // Add new symbols
        for symbol in symbols {
            sqlx::query(
                r#"
                INSERT INTO symbols (name, kind, uri, range_start_line, range_start_char, 
                                   range_end_line, range_end_char, container, signature, documentation)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )
            .bind(&symbol.name)
            .bind(symbol.kind)
            .bind(&symbol.uri)
            .bind(symbol.range_start_line)
            .bind(symbol.range_start_char)
            .bind(symbol.range_end_line)
            .bind(symbol.range_end_char)
            .bind(&symbol.container)
            .bind(&symbol.signature)
            .bind(&symbol.documentation)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Failed to insert symbol '{}': {}", symbol.name, e))?;
        }

        tx.commit().await
            .map_err(|e| anyhow!("Failed to commit symbol update: {}", e))?;

        info!("Updated symbols for file {}: {} symbols", uri, symbols.len());
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<IndexStats> {
        let symbol_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM symbols")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get symbol count: {}", e))?;

        let file_count: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT uri) FROM symbols")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get file count: {}", e))?;

        let import_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM imports")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to get import count: {}", e))?;

        Ok(IndexStats {
            symbol_count: symbol_count as usize,
            file_count: file_count as usize,
            import_count: import_count as usize,
        })
    }
    
    /// Update imports for a specific file
    pub async fn update_imports_for_file(&self, uri: &Url, imports: &[crate::tree_sitter_queries::ImportInfo]) -> Result<()> {
        let mut tx = self.pool.begin().await
            .map_err(|e| anyhow!("Failed to begin transaction: {}", e))?;
            
        // Remove existing imports for this file
        sqlx::query("DELETE FROM imports WHERE source_uri = ?1")
            .bind(uri.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Failed to remove old imports for file '{}': {}", uri, e))?;
            
        // Add new imports
        for import in imports {
            let imported_symbols_json = import.imported_symbols
                .as_ref()
                .map(|symbols| serde_json::to_string(symbols).unwrap_or_default());
                
            sqlx::query(
                r#"
                INSERT INTO imports (source_uri, imported_module, imported_symbols, alias_name, exposing_all)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
            )
            .bind(&import.source_uri)
            .bind(&import.imported_module)
            .bind(imported_symbols_json)
            .bind(&import.alias_name)
            .bind(import.exposing_all)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!("Failed to add import: {}", e))?;
        }
        
        tx.commit().await
            .map_err(|e| anyhow!("Failed to commit imports transaction: {}", e))?;
            
        debug!("Updated {} imports for file {}", imports.len(), uri);
        Ok(())
    }

    /// Get workspace root
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }
    
    /// Find symbols that are available in a given file through imports
    /// This is the core cross-module resolution functionality
    pub async fn find_available_symbols(&self, file_uri: &Url, symbol_name: &str) -> Result<Vec<Symbol>> {
        let mut available_symbols = Vec::new();
        
        // 1. Find local symbols in the same file
        let local_symbols = self.find_symbols_in_file(file_uri).await?;
        for symbol in local_symbols {
            if symbol.name == symbol_name {
                available_symbols.push(symbol);
            }
        }
        
        // 2. Find symbols from imported modules
        let imports = self.get_imports_for_file(file_uri).await?;
        for import in imports {
            // If exposing all symbols from the module
            if import.exposing_all {
                let module_symbols = self.find_symbols_by_module(&import.imported_module, symbol_name).await?;
                available_symbols.extend(module_symbols);
            }
            // If specific symbols are imported
            else if let Some(imported_symbols_json) = &import.imported_symbols {
                if let Ok(imported_symbols) = serde_json::from_str::<Vec<String>>(imported_symbols_json) {
                    if imported_symbols.contains(&symbol_name.to_string()) {
                        let module_symbols = self.find_symbols_by_module(&import.imported_module, symbol_name).await?;
                        available_symbols.extend(module_symbols);
                    }
                }
            }
            // If using module alias (e.g., Dict.map where Dict is the alias)
            else if let Some(alias) = &import.alias_name {
                if symbol_name.starts_with(&format!("{}.", alias)) {
                    let actual_symbol_name = symbol_name.strip_prefix(&format!("{}.", alias)).unwrap();
                    let module_symbols = self.find_symbols_by_module(&import.imported_module, actual_symbol_name).await?;
                    available_symbols.extend(module_symbols);
                }
            }
        }
        
        debug!("Found {} available symbols named '{}' for file {}", 
               available_symbols.len(), symbol_name, file_uri);
        Ok(available_symbols)
    }
    
    /// Find symbols by module name (for cross-module resolution)
    async fn find_symbols_by_module(&self, module_name: &str, symbol_name: &str) -> Result<Vec<Symbol>> {
        let symbols = sqlx::query_as::<_, Symbol>(
            "SELECT * FROM symbols WHERE container = ?1 AND name = ?2"
        )
        .bind(module_name)
        .bind(symbol_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to find symbols by module: {}", e))?;
        
        Ok(symbols)
    }
    
    /// Get all imports for a specific file
    async fn get_imports_for_file(&self, file_uri: &Url) -> Result<Vec<ImportInfo>> {
        let imports = sqlx::query_as::<_, ImportInfo>(
            "SELECT * FROM imports WHERE source_uri = ?1"
        )
        .bind(file_uri.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to get imports for file: {}", e))?;
        
        Ok(imports)
    }
    
    /// Find all modules that expose a specific symbol (for reverse resolution)
    pub async fn find_modules_exposing_symbol(&self, symbol_name: &str) -> Result<Vec<String>> {
        let modules: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT container FROM symbols WHERE name = ?1 AND container IS NOT NULL"
        )
        .bind(symbol_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to find modules exposing symbol: {}", e))?;
        
        Ok(modules.into_iter().map(|(module,)| module).collect())
    }
    
    /// Find symbols that would be available for auto-completion in a file
    pub async fn find_completion_symbols(&self, file_uri: &Url, prefix: &str, limit: i32) -> Result<Vec<Symbol>> {
        let mut completion_symbols = Vec::new();
        
        // 1. Add local symbols that match prefix
        let local_symbols = self.find_symbols_in_file(file_uri).await?;
        for symbol in local_symbols {
            if symbol.name.starts_with(prefix) {
                completion_symbols.push(symbol);
            }
        }
        
        // 2. Add symbols from imports
        let imports = self.get_imports_for_file(file_uri).await?;
        for import in imports {
            if import.exposing_all {
                // Add all symbols from the module that match prefix
                let module_symbols = self.find_symbols_by_module_prefix(&import.imported_module, prefix, limit).await?;
                completion_symbols.extend(module_symbols);
            } else if let Some(imported_symbols_json) = &import.imported_symbols {
                // Add only specifically imported symbols that match prefix
                if let Ok(imported_symbols) = serde_json::from_str::<Vec<String>>(imported_symbols_json) {
                    for imported_symbol in imported_symbols {
                        if imported_symbol.starts_with(prefix) {
                            let symbols = self.find_symbols_by_module(&import.imported_module, &imported_symbol).await?;
                            completion_symbols.extend(symbols);
                        }
                    }
                }
            }
        }
        
        // Limit results and remove duplicates
        completion_symbols.sort_by(|a, b| a.name.cmp(&b.name));
        completion_symbols.dedup_by(|a, b| a.name == b.name && a.uri == b.uri);
        completion_symbols.truncate(limit as usize);
        
        Ok(completion_symbols)
    }
    
    /// Find symbols in a module that start with a prefix
    async fn find_symbols_by_module_prefix(&self, module_name: &str, prefix: &str, limit: i32) -> Result<Vec<Symbol>> {
        let symbols = sqlx::query_as::<_, Symbol>(
            "SELECT * FROM symbols WHERE container = ?1 AND name LIKE ?2 LIMIT ?3"
        )
        .bind(module_name)
        .bind(format!("{}%", prefix))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to find symbols by module prefix: {}", e))?;
        
        Ok(symbols)
    }

    /// Close the database pool
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Symbol index database closed");
    }
}

/// Statistics about the symbol index
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub symbol_count: usize,
    pub file_count: usize,
    pub import_count: usize,
}

/// Import information for cross-module resolution
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ImportInfo {
    pub id: Option<i64>,
    pub source_uri: String,
    pub imported_module: String,
    pub imported_symbols: Option<String>, // JSON array
    pub alias_name: Option<String>,
    pub exposing_all: bool,
    pub created_at: Option<String>,
}

impl ImportInfo {
    /// Create a new import info
    pub fn new(
        source_uri: &Url,
        imported_module: String,
        imported_symbols: Option<Vec<String>>,
        alias_name: Option<String>,
        exposing_all: bool,
    ) -> Self {
        let imported_symbols_json = imported_symbols
            .map(|symbols| serde_json::to_string(&symbols).unwrap_or_default());

        Self {
            id: None,
            source_uri: source_uri.to_string(),
            imported_module,
            imported_symbols: imported_symbols_json,
            alias_name,
            exposing_all,
            created_at: None,
        }
    }

    /// Get imported symbols as a Vec
    pub fn get_imported_symbols(&self) -> Vec<String> {
        self.imported_symbols
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default()
    }
}

/// Convert SymbolKind to integer for database storage
fn symbol_kind_to_i32(kind: SymbolKind) -> i32 {
    match kind {
        SymbolKind::FILE => 1,
        SymbolKind::MODULE => 2,
        SymbolKind::NAMESPACE => 3,
        SymbolKind::PACKAGE => 4,
        SymbolKind::CLASS => 5,
        SymbolKind::METHOD => 6,
        SymbolKind::PROPERTY => 7,
        SymbolKind::FIELD => 8,
        SymbolKind::CONSTRUCTOR => 9,
        SymbolKind::ENUM => 10,
        SymbolKind::INTERFACE => 11,
        SymbolKind::FUNCTION => 12,
        SymbolKind::VARIABLE => 13,
        SymbolKind::CONSTANT => 14,
        SymbolKind::STRING => 15,
        SymbolKind::NUMBER => 16,
        SymbolKind::BOOLEAN => 17,
        SymbolKind::ARRAY => 18,
        SymbolKind::OBJECT => 19,
        SymbolKind::KEY => 20,
        SymbolKind::NULL => 21,
        SymbolKind::ENUM_MEMBER => 22,
        SymbolKind::STRUCT => 23,
        SymbolKind::EVENT => 24,
        SymbolKind::OPERATOR => 25,
        SymbolKind::TYPE_PARAMETER => 26,
        _ => 0, // Unknown
    }
}

/// Convert integer back to SymbolKind
fn i32_to_symbol_kind(kind: i32) -> SymbolKind {
    match kind {
        1 => SymbolKind::FILE,
        2 => SymbolKind::MODULE,
        3 => SymbolKind::NAMESPACE,
        4 => SymbolKind::PACKAGE,
        5 => SymbolKind::CLASS,
        6 => SymbolKind::METHOD,
        7 => SymbolKind::PROPERTY,
        8 => SymbolKind::FIELD,
        9 => SymbolKind::CONSTRUCTOR,
        10 => SymbolKind::ENUM,
        11 => SymbolKind::INTERFACE,
        12 => SymbolKind::FUNCTION,
        13 => SymbolKind::VARIABLE,
        14 => SymbolKind::CONSTANT,
        15 => SymbolKind::STRING,
        16 => SymbolKind::NUMBER,
        17 => SymbolKind::BOOLEAN,
        18 => SymbolKind::ARRAY,
        19 => SymbolKind::OBJECT,
        20 => SymbolKind::KEY,
        21 => SymbolKind::NULL,
        22 => SymbolKind::ENUM_MEMBER,
        23 => SymbolKind::STRUCT,
        24 => SymbolKind::EVENT,
        25 => SymbolKind::OPERATOR,
        26 => SymbolKind::TYPE_PARAMETER,
        _ => SymbolKind::VARIABLE, // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_symbol_index_creation() {
        // Use in-memory database for testing to avoid file system issues
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let workspace = std::env::temp_dir();

        let index = SymbolIndex {
            pool,
            workspace_root: workspace,
        };

        // Initialize schema manually for the in-memory database
        index.initialize_schema().await.unwrap();
        
        let stats = index.get_stats().await.unwrap();
        assert_eq!(stats.symbol_count, 0);
        assert_eq!(stats.file_count, 0);
        
        index.close().await;
    }

    #[tokio::test]
    async fn test_symbol_operations() {
        // Use in-memory database for testing to avoid file system issues
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let workspace = std::env::temp_dir();

        let index = SymbolIndex {
            pool,
            workspace_root: workspace,
        };

        // Initialize schema manually for the in-memory database
        index.initialize_schema().await.unwrap();
        
        let uri = Url::parse("file:///test.gren").unwrap();
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 10 },
        };
        
        let symbol = Symbol::new(
            "testFunction".to_string(),
            SymbolKind::FUNCTION,
            &uri,
            range,
            None,
            Some("testFunction : String".to_string()),
            Some("A test function".to_string()),
        );

        // Add symbol
        let id = index.add_symbol(&symbol).await.unwrap();
        assert!(id > 0);

        // Find by name
        let found = index.find_symbols_by_name("testFunction").await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "testFunction");

        // Find by prefix
        let found = index.find_symbols_by_prefix("test", 10).await.unwrap();
        assert_eq!(found.len(), 1);

        // Find in file
        let found = index.find_symbols_in_file(&uri).await.unwrap();
        assert_eq!(found.len(), 1);

        // Update stats
        let stats = index.get_stats().await.unwrap();
        assert_eq!(stats.symbol_count, 1);
        assert_eq!(stats.file_count, 1);

        index.close().await;
    }

    #[test]
    fn test_symbol_kind_conversion() {
        assert_eq!(symbol_kind_to_i32(SymbolKind::FUNCTION), 12);
        assert_eq!(i32_to_symbol_kind(12), SymbolKind::FUNCTION);
        
        assert_eq!(symbol_kind_to_i32(SymbolKind::VARIABLE), 13);
        assert_eq!(i32_to_symbol_kind(13), SymbolKind::VARIABLE);
    }

    #[test]
    fn test_symbol_lsp_conversion() {
        let uri = Url::parse("file:///test.gren").unwrap();
        let range = Range {
            start: Position { line: 1, character: 5 },
            end: Position { line: 1, character: 15 },
        };
        
        let symbol = Symbol::new(
            "myFunction".to_string(),
            SymbolKind::FUNCTION,
            &uri,
            range,
            Some("MyModule".to_string()),
            Some("myFunction : Int -> String".to_string()),
            Some("Converts int to string".to_string()),
        );

        let symbol_info = symbol.to_symbol_information().unwrap();
        assert_eq!(symbol_info.name, "myFunction");
        assert_eq!(symbol_info.kind, SymbolKind::FUNCTION);
        assert_eq!(symbol_info.location.uri, uri);
        assert_eq!(symbol_info.container_name, Some("MyModule".to_string()));

        let doc_symbol = symbol.to_document_symbol().unwrap();
        assert_eq!(doc_symbol.name, "myFunction");
        assert_eq!(doc_symbol.kind, SymbolKind::FUNCTION);
        assert_eq!(doc_symbol.detail, Some("myFunction : Int -> String".to_string()));
    }

    #[tokio::test]
    async fn test_file_indexing_integration() {
        // Use in-memory database for testing
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let workspace = std::env::temp_dir();
        let index = SymbolIndex {
            pool,
            workspace_root: workspace,
        };
        
        // Initialize schema
        index.initialize_schema().await.unwrap();
        
        // Sample Gren code
        let sample_code = r#"
module TestModule exposing (..)

import Array exposing (Array)
import Dict as Dictionary

type Status = Loading | Success String | Error

type alias User = { name : String, age : Int }

calculateAge : Int -> Int -> Int
calculateAge birthYear currentYear =
    currentYear - birthYear

processUser : User -> String
processUser user =
    "User " ++ user.name ++ " is " ++ String.fromInt user.age ++ " years old"

defaultUser : User
defaultUser = { name = "Anonymous", age = 0 }
        "#;
        
        let uri = Url::parse("file:///test-module.gren").unwrap();
        
        // Index the file
        index.index_file(&uri, sample_code).await.unwrap();
        
        // Verify symbols were extracted and stored
        let stats = index.get_stats().await.unwrap();
        assert!(stats.symbol_count > 0, "Should have extracted symbols");
        assert!(stats.import_count > 0, "Should have extracted imports");
        assert_eq!(stats.file_count, 1, "Should have one file indexed");
        
        // Check for specific symbols
        let functions = index.find_symbols_by_name("calculateAge").await.unwrap();
        assert!(!functions.is_empty(), "Should find calculateAge function");
        
        let types = index.find_symbols_by_name("Status").await.unwrap();
        assert!(!types.is_empty(), "Should find Status type");
        
        let aliases = index.find_symbols_by_name("User").await.unwrap();
        assert!(!aliases.is_empty(), "Should find User type alias");
        
        // Verify module symbols
        let modules = index.find_symbols_by_name("TestModule").await.unwrap();
        assert!(!modules.is_empty(), "Should find TestModule");
        
        // Test symbol search by prefix
        let user_symbols = index.find_symbols_by_prefix("user", 10).await.unwrap();
        assert!(!user_symbols.is_empty(), "Should find symbols starting with 'user'");
        
        println!("Integration test passed - extracted {} symbols and {} imports", 
                 stats.symbol_count, stats.import_count);
        
        index.close().await;
    }

    #[tokio::test]
    async fn test_cross_module_resolution() {
        // Use in-memory database for testing
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let workspace = std::env::temp_dir();
        let index = SymbolIndex {
            pool,
            workspace_root: workspace,
        };
        
        // Initialize schema
        index.initialize_schema().await.unwrap();
        
        // Create two Gren files to test cross-module resolution
        
        // File 1: Utils module
        let utils_code = r#"
module Utils exposing (helper, Config)

type alias Config = { debug : Bool }

helper : String -> String  
helper text = "Helper: " ++ text

privateFunction : Int -> Int
privateFunction x = x * 2
        "#;
        
        // File 2: Main module that imports Utils
        let main_code = r#"
module Main exposing (..)

import Utils exposing (helper, Config)
import Array as Arr

processData : Config -> String -> String
processData config input =
    if config.debug then
        helper input
    else
        input
        "#;
        
        let utils_uri = Url::parse("file:///utils.gren").unwrap();
        let main_uri = Url::parse("file:///main.gren").unwrap();
        
        // Index both files
        index.index_file(&utils_uri, utils_code).await.unwrap();
        index.index_file(&main_uri, main_code).await.unwrap();
        
        // Test 1: Find locally defined symbols in Main
        let local_symbols = index.find_available_symbols(&main_uri, "processData").await.unwrap();
        assert!(!local_symbols.is_empty(), "Should find processData in main file");
        
        // Debug: Check what symbols we have
        let all_symbols = index.find_symbols_by_name("helper").await.unwrap();
        println!("All symbols named 'helper': {:?}", all_symbols);
        
        let imports = index.get_imports_for_file(&main_uri).await.unwrap();
        println!("Imports for main file: {:?}", imports);
        
        // Test 2: Find imported symbols from Utils  
        let imported_helper = index.find_available_symbols(&main_uri, "helper").await.unwrap();
        println!("Available helper symbols in main: {:?}", imported_helper);
        assert!(!imported_helper.is_empty(), "Should find helper function from Utils import");
        
        let imported_config = index.find_available_symbols(&main_uri, "Config").await.unwrap();
        assert!(!imported_config.is_empty(), "Should find Config type from Utils import");
        
        // Test 3: Should NOT find non-imported symbols
        let private_fn = index.find_available_symbols(&main_uri, "privateFunction").await.unwrap();
        assert!(private_fn.is_empty(), "Should NOT find privateFunction (not imported)");
        
        // Test 4: Test module alias resolution (Array as Arr)
        // This would need qualified symbol names like "Arr.map" to work properly
        
        // Test 5: Find modules that expose a symbol
        let modules_with_helper = index.find_modules_exposing_symbol("helper").await.unwrap();
        assert!(modules_with_helper.contains(&"Utils".to_string()), "Utils should expose helper");
        
        // Test 6: Completion symbols
        let completions = index.find_completion_symbols(&main_uri, "h", 10).await.unwrap();
        assert!(!completions.is_empty(), "Should find symbols starting with 'h'");
        
        // Check that helper is in completions
        let has_helper = completions.iter().any(|s| s.name == "helper");
        assert!(has_helper, "Completions should include 'helper' from import");
        
        println!("Cross-module resolution test passed!");
        println!("Found {} local symbols, {} imported symbols", 
                 local_symbols.len(), imported_helper.len() + imported_config.len());
        
        index.close().await;
    }
}