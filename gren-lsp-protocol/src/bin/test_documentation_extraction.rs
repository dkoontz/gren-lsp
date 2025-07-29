// Test documentation comment extraction
use gren_lsp_core::Workspace;
use gren_lsp_protocol::handlers::Handlers;
use lsp_types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    println!("🔍 Testing documentation comment extraction");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create a test file with documentation comments
    let test_content = r#"module Test exposing (..)

{-| A simple add function that adds two integers.

This function takes two integers and returns their sum.
It's a basic arithmetic operation.

    add 2 3 == 5
    add 0 5 == 5
    add (-1) 1 == 0
-}
add : Int -> Int -> Int
add x y = x + y

{-| A person type alias with name and age fields.

This represents a person in our system.
-}
type alias Person = 
    { name : String
    , age : Int
    }

{-| Different status types for our application.

The status can be either active or inactive.
-}
type Status 
    = Active
    | Inactive

{-| Check if a person is an adult.

This function returns True if the person is 18 or older.
-}
isAdult : Person -> Bool
isAdult person = person.age >= 18

-- This function has no documentation
multiply : Int -> Int -> Int  
multiply a b = a * b
"#;

    let test_uri = Url::parse("file:///test/Test.gren")?;
    let test_doc = TextDocumentItem {
        uri: test_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: test_content.to_string(),
    };

    // Add document to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(test_doc)?;
    }

    println!("\n📁 Workspace setup complete");

    // Test 1: Hover on documented function "add"
    println!("\n🎯 Test 1: Hover on DOCUMENTED function 'add'");

    let hover_params1 = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: test_uri.clone(),
            },
            position: Position {
                line: 12,     // Line with "add : Int -> Int -> Int"
                character: 0, // Position on "add"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.hover(hover_params1).await {
        Ok(Some(hover)) => {
            println!("✅ Found hover content for add:");
            if let HoverContents::Markup(content) = hover.contents {
                println!("---");
                println!("{}", content.value);
                println!("---");

                // Verify it contains expected information
                if content.value.contains("**add**") && content.value.contains("function") {
                    println!("✅ Contains function name and type");
                } else {
                    println!("❌ Missing function name or type");
                }

                if content.value.contains("Int -> Int -> Int") {
                    println!("✅ Contains type signature");
                } else {
                    println!("❌ Missing type signature");
                }

                if content.value.contains("simple add function") {
                    println!("✅ Contains documentation");
                } else {
                    println!("❌ Missing documentation");
                }
            }
        }
        Ok(None) => {
            println!("❌ No hover content found for add");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 2: Hover on documented type alias "Person"
    println!("\n🎯 Test 2: Hover on DOCUMENTED type alias 'Person'");

    let hover_params2 = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: test_uri.clone(),
            },
            position: Position {
                line: 19,      // Line with "type alias Person ="
                character: 11, // Position on "Person"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.hover(hover_params2).await {
        Ok(Some(hover)) => {
            println!("✅ Found hover content for Person:");
            if let HoverContents::Markup(content) = hover.contents {
                println!("---");
                println!("{}", content.value);
                println!("---");

                if content.value.contains("person type alias") {
                    println!("✅ Contains documentation");
                } else {
                    println!("❌ Missing documentation");
                }
            }
        }
        Ok(None) => {
            println!("❌ No hover content found for Person");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    // Test 3: Hover on undocumented function "multiply"
    println!("\n🎯 Test 3: Hover on UNDOCUMENTED function 'multiply'");

    let hover_params3 = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: test_uri.clone(),
            },
            position: Position {
                line: 39,     // Line with "multiply : Int -> Int -> Int"
                character: 0, // Position on "multiply"
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.hover(hover_params3).await {
        Ok(Some(hover)) => {
            println!("✅ Found hover content for multiply:");
            if let HoverContents::Markup(content) = hover.contents {
                println!("---");
                println!("{}", content.value);
                println!("---");

                if content.value.contains("**multiply**") && content.value.contains("function") {
                    println!("✅ Contains function name and type");
                } else {
                    println!("❌ Missing function name or type");
                }

                if content.value.contains("Int -> Int -> Int") {
                    println!("✅ Contains type signature");
                } else {
                    println!("❌ Missing type signature");
                }

                // Should NOT contain documentation (no doc comment)
                if content.value.lines().count() <= 4 {
                    println!("✅ No documentation (as expected)");
                } else {
                    println!("❌ Unexpected documentation found");
                }
            }
        }
        Ok(None) => {
            println!("❌ No hover content found for multiply");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Documentation extraction test complete!");
    Ok(())
}
