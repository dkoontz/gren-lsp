pub mod lsp_service;
pub mod document_manager;
pub mod compiler_interface;
pub mod diagnostics;
pub mod symbol_index;
pub mod tree_sitter_queries;
pub mod gren_language;
pub mod completion;
pub mod scope_analysis;
pub mod hover;
pub mod goto_definition;

#[cfg(test)]
mod completion_integration_tests;

#[cfg(test)]
mod hover_integration_tests;

#[cfg(test)]
mod goto_definition_integration_tests;