// Debug script to examine tree-sitter AST
use gren_lsp_core::Parser;

fn main() -> anyhow::Result<()> {
    let gren_source = r#"
length : String -> Int
length str = String.length str
"#;

    let mut parser = Parser::new()?;
    let tree = parser.parse(gren_source)?.unwrap();
    
    println!("Tree structure:");
    print_tree(&tree.root_node(), gren_source.as_bytes(), 0);
    
    Ok(())
}

fn print_tree(node: &tree_sitter::Node, source: &[u8], depth: usize) {
    let indent = "  ".repeat(depth);
    let node_text = node.utf8_text(source).unwrap_or("<invalid utf8>");
    let node_text_short = if node_text.len() > 50 {
        format!("{}...", &node_text[..47])
    } else {
        node_text.replace('\n', "\\n")
    };
    
    println!("{}({}) \"{}\"", indent, node.kind(), node_text_short);
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(&child, source, depth + 1);
    }
}