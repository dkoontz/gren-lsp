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
pub mod find_references;
pub mod document_symbols;
pub mod code_actions;
pub mod workspace_symbols;
pub mod rename;
pub mod performance;
pub mod module_rename;
pub mod file_operations;
pub mod import_rewriter;
pub mod workspace_protocol;

#[cfg(test)]
mod completion_integration_tests;

#[cfg(test)]
mod hover_integration_tests;

#[cfg(test)]
mod goto_definition_integration_tests;

#[cfg(test)]
mod document_symbols_integration_tests;

#[cfg(test)]
mod code_actions_integration_tests;

#[cfg(test)]
mod workspace_symbols_integration_tests;

#[cfg(test)]
mod rename_integration_tests;

#[cfg(test)]
mod rename_basic_tests;

#[cfg(test)]
mod rename_comprehensive_tests;

#[cfg(test)]
mod module_rename_integration_tests;