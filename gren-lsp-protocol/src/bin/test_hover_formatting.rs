// Test hover formatting to debug the markdown issue
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

    println!("🔍 Testing hover formatting");

    let workspace = Arc::new(RwLock::new(Workspace::new()?));
    let handlers = Handlers::new(workspace.clone());

    // Create a Task module that defines the Task type
    let task_content = r#"module Task exposing (Task, succeed)

{-| A task represents an operation that can succeed or fail. -}
type Task x a = Task (() -> Result a x)

{-| Creates a task that always succeeds with the given value.

This is useful when you need to convert a regular value into a Task.
-}
succeed : a -> Task x a
succeed value = 
    Task (always (Ok value))
"#;

    // Create a main file that uses Task.succeed
    let main_content = r#"module Main exposing (..)

import Task

doSomething : String -> Task String String
doSomething input = 
    Task.succeed input
"#;

    // Create the Task module document
    let task_uri = Url::parse("file:///test/Task.gren")?;
    let task_doc = TextDocumentItem {
        uri: task_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: task_content.to_string(),
    };

    // Create the Main module document
    let main_uri = Url::parse("file:///test/Main.gren")?;
    let main_doc = TextDocumentItem {
        uri: main_uri.clone(),
        language_id: "gren".to_string(),
        version: 1,
        text: main_content.to_string(),
    };

    // Add both documents to workspace
    {
        let mut workspace = workspace.write().await;
        workspace.open_document(task_doc)?;
        workspace.open_document(main_doc)?;
    }

    println!("📁 Workspace setup complete");

    // Test hover on 'Task.succeed' in the Main file (qualified call)
    println!("🎯 Testing hover on 'Task.succeed' function (qualified call)");

    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 5,      // Line with "Task.succeed input"
                character: 9, // Position on "succeed" (after "Task.")
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    match handlers.hover(hover_params).await {
        Ok(Some(hover)) => {
            println!("✅ Found hover content:");
            if let HoverContents::Markup(content) = hover.contents {
                println!("===== RAW MARKDOWN CONTENT =====");
                println!("{}", content.value);
                println!("==================================");

                // Let's also break it down line by line to see the structure
                println!("\n===== LINE BY LINE BREAKDOWN =====");
                for (i, line) in content.value.lines().enumerate() {
                    println!("{:2}: '{}'", i + 1, line);
                }
                println!("====================================");

                // Check for specific formatting issues
                let lines: Vec<&str> = content.value.lines().collect();

                if let Some(header_line) = lines.first() {
                    if header_line.contains("**") && header_line.contains("*") {
                        println!("✅ Header has markdown formatting: {}", header_line);
                    } else {
                        println!("❌ Header missing markdown formatting: {}", header_line);
                    }
                }

                let has_code_block = content.value.contains("```gren");
                if has_code_block {
                    println!("✅ Contains gren code block");
                } else {
                    println!("❌ Missing gren code block");
                }

                let has_from_module = content.value.contains("*from module");
                if has_from_module {
                    println!("✅ Contains module information with formatting");
                } else {
                    println!("❌ Missing or incorrectly formatted module information");
                }
            }
        }
        Ok(None) => {
            println!("❌ No hover content found");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\n🧪 Hover formatting test complete!");
    Ok(())
}
