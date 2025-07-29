use anyhow::Result;
use lsp_types::*;
use rusqlite::{Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container_name: Option<String>,
    pub type_signature: Option<String>,
    pub documentation: Option<String>,
}

pub struct SymbolIndex {
    connection: Connection,
}

impl SymbolIndex {
    pub fn new() -> Result<Self> {
        // TODO: Use proper data directory
        let db_path = PathBuf::from("gren-lsp-symbols.db");
        let connection = Connection::open(db_path)?;
        
        // Create tables
        connection.execute(
            "CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                file_uri TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                start_character INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                end_character INTEGER NOT NULL,
                container_name TEXT,
                type_signature TEXT,
                documentation TEXT
            )",
            [],
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name)",
            [],
        )?;

        Ok(Self { connection })
    }

    pub fn index_symbol(&mut self, symbol: &Symbol) -> SqlResult<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO symbols 
            (name, kind, file_uri, start_line, start_character, end_line, end_character, 
             container_name, type_signature, documentation)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            [
                &symbol.name,
                &format!("{:?}", symbol.kind),
                &symbol.location.uri.to_string(),
                &symbol.location.range.start.line.to_string(),
                &symbol.location.range.start.character.to_string(),
                &symbol.location.range.end.line.to_string(),
                &symbol.location.range.end.character.to_string(),
                &symbol.container_name.as_ref().unwrap_or(&String::new()),
                &symbol.type_signature.as_ref().unwrap_or(&String::new()),
                &symbol.documentation.as_ref().unwrap_or(&String::new()),
            ],
        )?;
        Ok(())
    }

    pub fn find_symbol(&self, _name: &str) -> SqlResult<Vec<Symbol>> {
        // TODO: Implement symbol search
        Ok(vec![])
    }
}