use anyhow::Result;
use lsp_textdocument::FullTextDocument;
use lsp_types::*;

pub struct Document {
    text_document: FullTextDocument,
    uri: Url,
}

impl Document {
    pub fn new(text_document_item: TextDocumentItem) -> Self {
        let uri = text_document_item.uri.clone();
        let text_document = FullTextDocument::new(
            text_document_item.language_id,
            text_document_item.version,
            text_document_item.text,
        );

        Self { text_document, uri }
    }

    pub fn uri(&self) -> &Url {
        &self.uri
    }

    pub fn version(&self) -> i32 {
        self.text_document.version()
    }

    pub fn text(&self) -> &str {
        self.text_document.get_content(None)
    }

    pub fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        let new_version = self.version() + 1;
        self.text_document.update(&changes, new_version);
        Ok(())
    }

    pub fn position_to_offset(&self, position: Position) -> Option<usize> {
        Some(self.text_document.offset_at(position) as usize)
    }

    pub fn offset_to_position(&self, offset: usize) -> Option<Position> {
        Some(self.text_document.position_at(offset as u32))
    }
}