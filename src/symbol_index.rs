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

    /// Get workspace root
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
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
}