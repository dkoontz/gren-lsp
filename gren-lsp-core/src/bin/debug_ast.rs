// Debug tool to examine tree-sitter AST structure
use gren_lsp_core::Parser;

fn main() -> anyhow::Result<()> {
    let gren_source = r#"
module Main exposing (..)

length : String -> Int
length str = String.length str

add : Int -> Int -> Int
add x y = x + y
"#;

    let mut parser = Parser::new()?;
    let tree = parser.parse(gren_source)?.unwrap();
    
    println!("=== Tree-sitter AST Structure ===");
    print_tree(&tree.root_node(), gren_source.as_bytes(), 0);
    
    Ok(())
}

fn print_tree(node: &tree_sitter::Node, source: &[u8], depth: usize) {
    let indent = "  ".repeat(depth);
    let node_text = node.utf8_text(source).unwrap_or("<invalid utf8>");
    let node_text_short = if node_text.len() > 80 {
        format!("{}...", &node_text[..77])
    } else {
        node_text.replace('\n', "\\n").replace('\r', "")
    };
    
    println!("{}({}) \"{}\"", indent, node.kind(), node_text_short);
    
    // Only recurse for non-leaf nodes to avoid too much output
    if node.child_count() > 0 {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_tree(&child, source, depth + 1);
        }
    }
}