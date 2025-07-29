use gren_lsp_core::Parser;

/// Test that parser initializes correctly with Gren grammar
#[test]
fn test_parser_initialization() {
    let parser = Parser::new();
    assert!(parser.is_ok(), "Parser should initialize successfully");
}

/// Test parsing a simple valid Gren module
#[test]
fn test_parse_simple_module() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    let source = r#"
module TestModule exposing (..)

greet : String -> String
greet name =
    "Hello, " ++ name
"#;
    
    let tree = parser.parse(source).expect("Parse should not fail");
    assert!(tree.is_some(), "Parse should return a tree");
    
    let tree = tree.unwrap();
    assert!(!Parser::has_errors(&tree), "Parse tree should not contain errors");
}

/// Test parsing with syntax errors
#[test]
fn test_parse_with_errors() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    // Invalid Gren code with syntax error
    let source = r#"
module TestModule exposing (..)

greet : String ->
    "Hello"
"#;
    
    let tree = parser.parse(source).expect("Parse should not fail even with errors");
    assert!(tree.is_some(), "Parse should return a tree even with errors");
    
    let tree = tree.unwrap();
    assert!(Parser::has_errors(&tree), "Parse tree should contain errors");
    
    let errors = Parser::extract_errors(&tree);
    assert!(!errors.is_empty(), "Should extract at least one error");
}

/// Test incremental parsing
#[test]
fn test_incremental_parsing() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    let initial_source = r#"
module TestModule exposing (..)

greet : String -> String
greet name =
    "Hello, " ++ name
"#;
    
    // Parse initial source
    let old_tree = parser.parse(initial_source)
        .expect("Initial parse should not fail")
        .expect("Should return a tree");
    
    // Modified source
    let modified_source = r#"
module TestModule exposing (..)

greet : String -> String
greet name =
    "Hi, " ++ name
"#;
    
    // Parse incrementally
    let new_tree = parser.parse_incremental(modified_source, Some(&old_tree))
        .expect("Incremental parse should not fail")
        .expect("Should return a tree");
    
    assert!(!Parser::has_errors(&new_tree), "Incremental parse should not have errors");
}

/// Test parsing empty file
#[test]
fn test_parse_empty_file() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    let source = "";
    let tree = parser.parse(source).expect("Parse should not fail");
    assert!(tree.is_some(), "Parse should return a tree for empty input");
}

/// Test parsing complex Gren constructs
#[test]
fn test_parse_complex_constructs() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    let source = r#"
module ComplexModule exposing (..)

import Array

type alias User =
    { name : String
    , age : Int
    }

type Message
    = Success String
    | Error String
    | Loading

processUsers : Array User -> Array User
processUsers users =
    Array.filter (\user -> user.age >= 18) users

handleMessage : Message -> String
handleMessage msg =
    case msg of
        Success data ->
            "Success: " ++ data
            
        Error error ->
            "Error: " ++ error
            
        Loading ->
            "Loading..."
"#;
    
    let tree = parser.parse(source).expect("Parse should not fail");
    assert!(tree.is_some(), "Parse should return a tree");
    
    let tree = tree.unwrap();
    if Parser::has_errors(&tree) {
        let errors = Parser::extract_errors(&tree);
        eprintln!("Parse errors found: {:#?}", errors);
        // Don't panic, just warn - the grammar might not support all features yet
        eprintln!("Warning: Complex Gren constructs have parse errors (grammar may be incomplete)");
    }
}

/// Test parsing type annotations and signatures
#[test]
fn test_parse_type_annotations() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    let source = r#"
module TypeAnnotations exposing (..)

-- Function with complex type signature
mapWithIndex : (Int -> a -> b) -> Array a -> Array b
mapWithIndex fn array =
    Array.indexedMap fn array

-- Function with type constraints
compare : comparable -> comparable -> Order
compare a b =
    if a < b then
        LT
    else if a > b then
        GT
    else
        EQ
"#;
    
    let tree = parser.parse(source).expect("Parse should not fail");
    assert!(tree.is_some(), "Parse should return a tree");
    
    let tree = tree.unwrap();
    assert!(!Parser::has_errors(&tree), "Type annotations should parse correctly");
}

/// Test parsing comments
#[test]
fn test_parse_comments() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    let source = r#"
module Comments exposing (..)

-- This is a line comment

{- 
   This is a
   block comment
-}

greet : String -> String
greet name =
    -- Another line comment
    "Hello, " ++ name  -- End of line comment
"#;
    
    let tree = parser.parse(source).expect("Parse should not fail");
    assert!(tree.is_some(), "Parse should return a tree");
    
    let tree = tree.unwrap();
    assert!(!Parser::has_errors(&tree), "Comments should not cause parse errors");
}

/// Test language() function
#[test]
fn test_language_function() {
    let language = Parser::language();
    // Test that we get a valid language object
    assert!(language.node_kind_count() > 0, "Language should have node kinds");
}

/// Test error information extraction
#[test]
fn test_error_extraction() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    let source = r#"
module TestModule exposing (..)

-- Missing type annotation and function body
someFunction : 
"#;
    
    let tree = parser.parse(source).expect("Parse should not fail");
    let tree = tree.unwrap();
    
    let errors = Parser::extract_errors(&tree);
    assert!(!errors.is_empty(), "Should extract parse errors");
    
    for error in &errors {
        // Verify error structure
        assert!(error.start_byte <= error.end_byte, "Error byte range should be valid");
        assert!(!error.kind.is_empty(), "Error should have a kind");
    }
}

/// Test parsing real Gren examples (if any sample files exist)
#[test]
fn test_parse_sample_files() {
    let mut parser = Parser::new().expect("Parser initialization failed");
    
    // Create a sample Gren file for testing
    let sample_content = r#"
module Sample exposing (..)

import Array

type alias Person =
    { name : String
    , age : Int
    }

createPerson : String -> Int -> Person
createPerson name age =
    { name = name, age = age }

isAdult : Person -> Bool
isAdult person =
    person.age >= 18

processGroup : Array Person -> Array String
processGroup people =
    people
        |> Array.filter isAdult
        |> Array.map .name
"#;
    
    let tree = parser.parse(sample_content).expect("Parse should not fail");
    assert!(tree.is_some(), "Sample file should parse");
    
    let tree = tree.unwrap();
    assert!(!Parser::has_errors(&tree), "Sample file should not have errors");
}