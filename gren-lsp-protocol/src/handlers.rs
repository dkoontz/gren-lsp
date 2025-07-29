use gren_lsp_core::Workspace;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::{Error, Result};

pub struct Handlers {
    workspace: Arc<RwLock<Workspace>>,
}

impl Handlers {
    pub fn new(workspace: Arc<RwLock<Workspace>>) -> Self {
        Self { workspace }
    }

    pub async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // TODO: Implement hover
        Ok(None)
    }

    pub async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // TODO: Implement completion
        Ok(None)
    }

    pub async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        // TODO: Implement goto definition
        Ok(None)
    }

    pub async fn find_references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        // TODO: Implement find references
        Ok(None)
    }

    pub async fn document_symbols(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        // TODO: Implement document symbols
        Ok(None)
    }

    pub async fn workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        // TODO: Implement workspace symbols
        Ok(None)
    }

    pub async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        // TODO: Implement code actions
        Ok(None)
    }

    pub async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        // TODO: Implement rename
        Ok(None)
    }
}