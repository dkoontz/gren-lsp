use crate::document::Document;
use anyhow::Result;
use lsp_types::*;
use std::collections::HashMap;
use tracing::info;

pub struct Workspace {
    root_uri: Option<Url>,
    documents: HashMap<Url, Document>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            root_uri: None,
            documents: HashMap::new(),
        }
    }

    pub fn set_root(&mut self, root_uri: Url) {
        info!("Setting workspace root: {}", root_uri);
        self.root_uri = Some(root_uri);
    }

    pub fn open_document(&mut self, text_document: TextDocumentItem) {
        let document = Document::new(text_document);
        self.documents.insert(document.uri().clone(), document);
    }

    pub fn update_document(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;
        
        if let Some(document) = self.documents.get_mut(&uri) {
            document.apply_changes(params.content_changes)?;
        }
        
        Ok(())
    }

    pub fn close_document(&mut self, uri: Url) {
        self.documents.remove(&uri);
    }

    pub fn get_document(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }
}