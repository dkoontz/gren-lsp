use anyhow::Result;
use tree_sitter::{Parser as TreeSitterParser, Tree};

pub struct Parser {
    parser: TreeSitterParser,
}

impl Parser {
    pub fn new() -> Result<Self> {
        let parser = TreeSitterParser::new();
        
        // TODO: Set Gren language when tree-sitter-gren is available
        // parser.set_language(tree_sitter_gren::language())?;
        
        Ok(Self { parser })
    }

    pub fn parse(&mut self, source: &str) -> Result<Option<Tree>> {
        Ok(self.parser.parse(source, None))
    }

    pub fn parse_incremental(
        &mut self,
        source: &str,
        old_tree: Option<&Tree>,
    ) -> Result<Option<Tree>> {
        Ok(self.parser.parse(source, old_tree))
    }
}