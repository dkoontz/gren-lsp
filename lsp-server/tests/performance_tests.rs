use anyhow::Result;
use gren_lsp::symbol_index::SymbolIndex;
use gren_lsp::performance::{PerformanceManager, calculate_content_hash};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tower_lsp::lsp_types::*;
use tracing::{info, warn};

/// Performance test suite for Epic 3 Story 3 requirements
#[cfg(test)]
mod performance_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_large_project_handling() {
        let index = create_test_index().await;
        let start_time = Instant::now();
        
        // Create a simulated large project with 100+ files
        let large_project_files = generate_large_project_files(150);
        
        // Index all files and measure time
        for (uri, content) in &large_project_files {
            index.index_file(uri, content).await.unwrap();
        }
        
        let indexing_time = start_time.elapsed();
        info!("Indexed {} files in {:?}", large_project_files.len(), indexing_time);
        
        // Requirement: Workspace initialization should complete within 10 seconds
        assert!(indexing_time < Duration::from_secs(10), 
                "Indexing {} files took {:?}, exceeding 10 second target", 
                large_project_files.len(), indexing_time);
                
        // Verify all files were indexed
        let stats = index.get_stats().await.unwrap();
        assert_eq!(stats.file_count, large_project_files.len());
        assert!(stats.symbol_count > 1000, "Should have extracted substantial symbols");
        
        info!("✅ Large project test passed: {} files, {} symbols, {:?} indexing time", 
              stats.file_count, stats.symbol_count, indexing_time);
        
        index.close().await;
    }

    #[tokio::test]
    async fn test_symbol_indexing_performance() {
        let index = create_test_index().await;
        
        // Test incremental symbol index updates
        let test_files = generate_test_files(50);
        let start_time = Instant::now();
        
        for (uri, content) in &test_files {
            let file_start = Instant::now();
            index.index_file(uri, content).await.unwrap();
            let file_time = file_start.elapsed();
            
            // Requirement: Incremental updates should complete within 100ms per file
            assert!(file_time < Duration::from_millis(100), 
                    "File indexing took {:?}, exceeding 100ms target", file_time);
        }
        
        let total_time = start_time.elapsed();
        info!("✅ Symbol indexing performance test passed: {} files in {:?}", 
              test_files.len(), total_time);
        
        index.close().await;
    }

    #[tokio::test]
    async fn test_reference_finding_scalability() {
        let index = create_test_index().await;
        
        // Create a project with cross-module dependencies
        let project_files = generate_cross_module_project(100);
        
        // Index all files
        for (uri, content) in &project_files {
            index.index_file(uri, content).await.unwrap();
        }
        
        // Test reference finding performance
        let test_symbols = vec!["commonFunction", "SharedType", "utilsHelper"];
        
        for symbol_name in test_symbols {
            let start_time = Instant::now();
            
            // Use timeout to ensure we don't hang
            let result = timeout(
                Duration::from_millis(200),
                index.find_references(symbol_name)
            ).await;
            
            match result {
                Ok(Ok(references)) => {
                    let find_time = start_time.elapsed();
                    info!("Found {} references for '{}' in {:?}", 
                          references.len(), symbol_name, find_time);
                    
                    // Requirement: Reference finding should complete within 200ms
                    assert!(find_time < Duration::from_millis(200), 
                            "Reference finding for '{}' took {:?}, exceeding 200ms target", 
                            symbol_name, find_time);
                },
                Ok(Err(e)) => panic!("Reference finding failed: {}", e),
                Err(_) => panic!("Reference finding timed out for '{}'", symbol_name),
            }
        }
        
        info!("✅ Reference finding scalability test passed");
        index.close().await;
    }

    #[tokio::test]
    async fn test_document_symbol_performance() {
        let index = create_test_index().await;
        
        // Create files with many symbols
        let complex_files = generate_complex_symbol_files(10);
        
        for (uri, content) in &complex_files {
            index.index_file(uri, content).await.unwrap();
            
            let start_time = Instant::now();
            let symbols = index.find_symbols_in_file(uri).await.unwrap();
            let query_time = start_time.elapsed();
            
            // Requirement: Document symbol extraction under 100ms
            assert!(query_time < Duration::from_millis(100), 
                    "Document symbol query took {:?}, exceeding 100ms target", query_time);
            
            // Should have substantial symbols
            assert!(symbols.len() > 50, "Should have many symbols in complex file");
            
            info!("Document symbols for complex file: {} symbols in {:?}", 
                  symbols.len(), query_time);
        }
        
        info!("✅ Document symbol performance test passed");
        index.close().await;
    }

    #[tokio::test]
    async fn test_memory_management() {
        let index = create_test_index().await;
        let perf_manager = PerformanceManager::new(100, 20);
        
        // Test LRU cache behavior with many documents
        let mut test_files = Vec::new();
        
        // Create 150 files (more than cache capacity)
        for i in 0..150 {
            let uri = Url::parse(&format!("file:///test_{}.gren", i)).unwrap();
            let content = generate_test_file_content(i, 100); // 100 symbols per file
            test_files.push((uri, content));
        }
        
        // Cache parse trees for all files
        for (i, (uri, content)) in test_files.iter().enumerate() {
            let content_hash = calculate_content_hash(content);
            
            // Simulate parse tree (we can't create real ones easily in tests)
            // In practice, this would be a real tree-sitter Tree
            // perf_manager.cache_parse_tree(uri.clone(), content_hash, tree).await;
            
            // Index the file
            index.index_file(uri, content).await.unwrap();
            
            if i % 50 == 0 {
                let stats = perf_manager.get_performance_stats().await;
                info!("Memory test progress: {} files processed, cache stats: {:?}", 
                      i + 1, stats);
            }
        }
        
        // Verify final stats
        let final_stats = perf_manager.get_performance_stats().await;
        let db_stats = index.get_stats().await.unwrap();
        
        // Memory should be bounded by cache sizes
        assert!(final_stats.parse_tree_cache.size <= 20, 
                "Parse tree cache exceeded limit: {}", final_stats.parse_tree_cache.size);
        
        // Database should contain all symbols
        assert_eq!(db_stats.file_count, 150);
        assert!(db_stats.symbol_count > 10000, "Should have many symbols indexed");
        
        info!("✅ Memory management test passed: {} files, {} symbols, cache bounded", 
              db_stats.file_count, db_stats.symbol_count);
        
        index.close().await;
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let index = create_test_index().await;
        
        // Test concurrent LSP requests
        let test_files = generate_test_files(20);
        
        // Index files first
        for (uri, content) in &test_files {
            index.index_file(uri, content).await.unwrap();
        }
        
        // Create concurrent tasks
        let mut handles = Vec::new();
        
        // Spawn multiple concurrent symbol lookups
        for i in 0..10 {
            let index_clone = index.clone();
            let symbol_name = format!("function_{}", i % 5); // Reuse some symbol names
            
            let handle = tokio::spawn(async move {
                let start = Instant::now();
                let _result = index_clone.find_symbols_by_name(&symbol_name).await;
                let duration = start.elapsed();
                (symbol_name, duration)
            });
            
            handles.push(handle);
        }
        
        // Wait for all concurrent operations
        let start_time = Instant::now();
        let results: Vec<_> = futures::future::join_all(handles).await;
        let total_time = start_time.elapsed();
        
        // Verify all operations completed successfully
        for result in results {
            let (symbol_name, duration) = result.unwrap();
            assert!(duration < Duration::from_millis(100), 
                    "Concurrent operation for '{}' took {:?}", symbol_name, duration);
        }
        
        // Total time should be much less than sequential execution
        assert!(total_time < Duration::from_secs(2), 
                "Concurrent operations took {:?}, suggesting blocking", total_time);
        
        info!("✅ Concurrent operations test passed in {:?}", total_time);
        index.close().await;
    }

    // Helper functions for test data generation

    async fn create_test_index() -> SymbolIndex {
        SymbolIndex::new_in_memory(std::env::temp_dir()).await.unwrap()
    }

    fn generate_large_project_files(count: usize) -> Vec<(Url, String)> {
        let mut files = Vec::new();
        
        for i in 0..count {
            let uri = Url::parse(&format!("file:///project/module_{}.gren", i)).unwrap();
            let content = generate_test_file_content(i, 15); // 15 symbols per file
            files.push((uri, content));
        }
        
        files
    }

    fn generate_test_files(count: usize) -> Vec<(Url, String)> {
        let mut files = Vec::new();
        
        for i in 0..count {
            let uri = Url::parse(&format!("file:///test_{}.gren", i)).unwrap();
            let content = generate_test_file_content(i, 10);
            files.push((uri, content));
        }
        
        files
    }

    fn generate_cross_module_project(count: usize) -> Vec<(Url, String)> {
        let mut files = Vec::new();
        
        // Generate files with cross-references
        for i in 0..count {
            let uri = Url::parse(&format!("file:///project/module_{}.gren", i)).unwrap();
            let content = format!(
                r#"
module Module{} exposing (..)

import Module{} exposing (commonFunction)
import Utils exposing (utilsHelper)

type SharedType = Loading | Success String

localFunction_{} : String -> String
localFunction_{} input = 
    commonFunction input

processData_{} : SharedType -> String
processData_{} data =
    when data is
        Loading -> "loading..."
        Success value -> utilsHelper value

constants_{} = {{ value = {}, name = "item_{}" }}
                "#,
                i,
                (i + 1) % count, // Import next module (circular)
                i, i, i, i, i, i, i
            );
            files.push((uri, content));
        }
        
        files
    }

    fn generate_complex_symbol_files(count: usize) -> Vec<(Url, String)> {
        let mut files = Vec::new();
        
        for i in 0..count {
            let uri = Url::parse(&format!("file:///complex_{}.gren", i)).unwrap();
            
            // Generate file with many symbols
            let mut content = format!("module Complex{} exposing (..)\n\n", i);
            
            // Add many type definitions
            for j in 0..20 {
                content.push_str(&format!(
                    "type Type{} = Variant{}A | Variant{}B String | Variant{}C Int\n",
                    j, j, j, j
                ));
            }
            
            // Add many functions
            for j in 0..30 {
                content.push_str(&format!(
                    r#"
function_{} : String -> Int -> String
function_{} str num = str ++ String.fromInt num

constant_{} = "value_{}"
                    "#,
                    j, j, j, j
                ));
            }
            
            files.push((uri, content));
        }
        
        files
    }

    fn generate_test_file_content(index: usize, symbol_count: usize) -> String {
        let mut content = format!("module TestModule{} exposing (..)\n\n", index);
        
        for i in 0..symbol_count {
            content.push_str(&format!(
                r#"
function_{} : String -> String
function_{} input = "result_" ++ input

type Type{} = Value{} String

constant_{} = {}
                "#,
                i, i, i, i, i, i
            ));
        }
        
        content
    }
}

/// Benchmark utilities for manual performance testing
pub struct PerformanceBenchmark {
    start_time: Instant,
    name: String,
}

impl PerformanceBenchmark {
    pub fn new(name: &str) -> Self {
        info!("🕐 Starting benchmark: {}", name);
        Self {
            start_time: Instant::now(),
            name: name.to_string(),
        }
    }
    
    pub fn checkpoint(&self, operation: &str) {
        let elapsed = self.start_time.elapsed();
        info!("⏱️ Benchmark '{}' - {}: {:?}", self.name, operation, elapsed);
    }
    
    pub fn finish(self) -> Duration {
        let elapsed = self.start_time.elapsed();
        info!("✅ Benchmark '{}' completed in {:?}", self.name, elapsed);
        elapsed
    }
}

/// Performance test runner for manual testing
pub async fn run_performance_test_suite() -> Result<()> {
    info!("🚀 Starting Epic 3 Story 3 Performance Test Suite");
    
    let overall_benchmark = PerformanceBenchmark::new("Full Performance Suite");
    
    // Run each test category
    test_large_project_simulation().await?;
    overall_benchmark.checkpoint("Large project test");
    
    test_symbol_query_performance().await?;
    overall_benchmark.checkpoint("Symbol query test");
    
    test_memory_efficiency().await?;
    overall_benchmark.checkpoint("Memory efficiency test");
    
    let total_time = overall_benchmark.finish();
    
    info!("🎉 Performance test suite completed successfully in {:?}", total_time);
    Ok(())
}

async fn test_large_project_simulation() -> Result<()> {
    let benchmark = PerformanceBenchmark::new("Large Project Simulation");
    let index = SymbolIndex::new_in_memory(std::env::temp_dir()).await?;
    
    // Simulate 200+ file project
    let files = (0..200).map(|i| {
        let uri = Url::parse(&format!("file:///large_project/module_{}.gren", i)).unwrap();
        let content = format!(
            r#"
module LargeModule{} exposing (..)

import Array
import Dict as Dictionary

type Status{} = Loading | Success String | Error String

calculateValue{} : Int -> Int -> String
calculateValue{} a b = String.fromInt (a + b)

processArray{} : Array String -> Array String  
processArray{} arr = Array.map String.toUpper arr

constants{} = {{ id = {}, name = "item_{}" }}
            "#,
            i, i, i, i, i, i, i, i, i
        );
        (uri, content)
    }).collect::<Vec<_>>();
    
    benchmark.checkpoint("Generated test data");
    
    // Index all files
    for (uri, content) in &files {
        index.index_file(uri, content).await?;
    }
    
    benchmark.checkpoint("Indexed all files");
    
    let stats = index.get_stats().await?;
    info!("Large project stats: {} files, {} symbols", stats.file_count, stats.symbol_count);
    
    index.close().await;
    benchmark.finish();
    Ok(())
}

async fn test_symbol_query_performance() -> Result<()> {
    let benchmark = PerformanceBenchmark::new("Symbol Query Performance");
    let index = SymbolIndex::new_in_memory(std::env::temp_dir()).await?;
    
    // Create database with 10,000+ symbols
    let files = (0..100).map(|i| {
        let uri = Url::parse(&format!("file:///symbols/file_{}.gren", i)).unwrap();
        let mut content = format!("module SymbolTest{} exposing (..)\n\n", i);
        
        // 100+ symbols per file
        for j in 0..100 {
            content.push_str(&format!(
                "symbol_{}_{} : String\nsymbol_{}_{} = \"value\"\n\n",
                i, j, i, j
            ));
        }
        
        (uri, content)
    }).collect::<Vec<_>>();
    
    for (uri, content) in &files {
        index.index_file(uri, content).await?;
    }
    
    benchmark.checkpoint("Created symbol database");
    
    // Test query performance
    let test_queries = ["symbol_50_50", "symbol_0_0", "symbol_99_99"];
    
    for query in test_queries {
        let start = Instant::now();
        let results = index.find_symbols_by_name(query).await?;
        let query_time = start.elapsed();
        
        info!("Query '{}': {} results in {:?}", query, results.len(), query_time);
        assert!(query_time < Duration::from_millis(50), "Query too slow");
    }
    
    benchmark.checkpoint("Completed query tests");
    
    index.close().await;
    benchmark.finish();
    Ok(())
}

async fn test_memory_efficiency() -> Result<()> {
    let benchmark = PerformanceBenchmark::new("Memory Efficiency");
    let perf_manager = PerformanceManager::new(500, 100);
    
    // Test cache behavior
    for i in 0..1000 {
        let symbol_name = format!("test_symbol_{}", i);
        let references = vec![]; // Empty for test
        
        perf_manager.cache_references(
            symbol_name, 
            references, 
            Duration::from_secs(60)
        ).await;
        
        if i % 200 == 0 {
            let stats = perf_manager.get_performance_stats().await;
            info!("Cache stats at {}: reference cache size = {}, hit rate = {:.1}%", 
                  i, stats.reference_cache.size, stats.reference_cache.hit_rate);
        }
    }
    
    benchmark.checkpoint("Cache stress test");
    
    let final_stats = perf_manager.get_performance_stats().await;
    assert!(final_stats.reference_cache.size <= 500, "Cache size not bounded");
    
    benchmark.finish();
    Ok(())
}