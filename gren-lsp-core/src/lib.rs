pub mod analysis;
pub mod compiler;
pub mod compiler_diagnostics;
pub mod diagnostics;
pub mod document;
pub mod parser;
pub mod symbol;
pub mod workspace;

pub use analysis::AnalysisEngine;
pub use compiler::{CompilerDiagnostic, GrenCompiler};
pub use compiler_diagnostics::{compiler_diagnostics_to_lsp, group_diagnostics_by_uri, merge_diagnostics};
pub use diagnostics::parse_errors_to_diagnostics;
pub use document::Document;
pub use parser::{ParseError, Parser};
pub use symbol::{Symbol, SymbolExtractor, SymbolIndex};
pub use workspace::{Workspace, WorkspaceStats};
