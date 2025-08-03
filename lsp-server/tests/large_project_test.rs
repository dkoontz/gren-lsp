use anyhow::Result;
use gren_lsp::symbol_index::SymbolIndex;
use std::time::{Duration, Instant};
use tower_lsp::lsp_types::*;
use tracing::info;

/// Critical performance test: Verify 150+ file handling capability
#[tokio::test]
async fn test_large_project_150_files() -> Result<()> {
    let test_start = Instant::now();
    
    // Create in-memory database for testing
    let index = SymbolIndex::new_in_memory(std::env::temp_dir()).await?;
    info!("✅ Created symbol index");
    
    // Generate 150 realistic Gren files
    let files = generate_realistic_gren_files(150);
    info!("✅ Generated {} test files", files.len());
    
    // INDEX ALL FILES - This is the critical test
    let indexing_start = Instant::now();
    for (i, (uri, content)) in files.iter().enumerate() {
        index.index_file(uri, content).await?;
        
        if i % 25 == 0 {
            info!("📈 Indexed {} files...", i + 1);
        }
    }
    let indexing_duration = indexing_start.elapsed();
    
    // VERIFY: Indexing completed within reasonable time (Epic 3 Story 3 requirement)
    assert!(indexing_duration < Duration::from_secs(30), 
            "FAIL: Indexing {} files took {:?}, exceeding reasonable limit", 
            files.len(), indexing_duration);
    info!("✅ INDEXING PERFORMANCE: {} files indexed in {:?}", files.len(), indexing_duration);
    
    // VERIFY: All files were actually indexed
    let stats = index.get_stats().await?;
    assert_eq!(stats.file_count, 150, "Not all files were indexed");
    assert!(stats.symbol_count > 1000, "Should have extracted substantial symbols");
    info!("✅ DATABASE STATS: {} files, {} symbols, {} imports, {} references", 
          stats.file_count, stats.symbol_count, stats.import_count, stats.reference_count);
    
    // TEST SYMBOL LOOKUP PERFORMANCE
    let lookup_tests = ["commonFunction", "SharedType", "calculateValue", "processData"];
    for symbol_name in lookup_tests {
        let lookup_start = Instant::now();
        let symbols = index.find_symbols_by_name(symbol_name).await?;
        let lookup_duration = lookup_start.elapsed();
        
        // VERIFY: Symbol lookup under 200ms (Epic 3 Story 3 requirement)
        assert!(lookup_duration < Duration::from_millis(200), 
                "Symbol lookup for '{}' took {:?}, exceeding 200ms target", 
                symbol_name, lookup_duration);
        
        info!("🔍 Symbol '{}': {} results in {:?}", symbol_name, symbols.len(), lookup_duration);
    }
    
    // TEST REFERENCE FINDING PERFORMANCE
    let reference_start = Instant::now();
    let references = index.find_references("commonFunction").await?;
    let reference_duration = reference_start.elapsed();
    
    // VERIFY: Reference finding under 200ms (Epic 3 Story 3 requirement)
    assert!(reference_duration < Duration::from_millis(200), 
            "Reference finding took {:?}, exceeding 200ms target", reference_duration);
    info!("🔗 References: {} found in {:?}", references.len(), reference_duration);
    
    // TEST CONCURRENT OPERATIONS
    let concurrent_start = Instant::now();
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let index_clone = index.clone();
        let symbol_name = format!("testSymbol_{}", i % 3);
        
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let _result = index_clone.find_symbols_by_name(&symbol_name).await;
            start.elapsed()
        });
        handles.push(handle);
    }
    
    let results = futures::future::join_all(handles).await;
    let concurrent_duration = concurrent_start.elapsed();
    
    // VERIFY: Concurrent operations don't block excessively
    assert!(concurrent_duration < Duration::from_secs(2), 
            "Concurrent operations took {:?}, suggesting blocking", concurrent_duration);
    
    for (i, result) in results.iter().enumerate() {
        let duration = result.as_ref().unwrap();
        assert!(duration < &Duration::from_millis(100), 
                "Concurrent operation {} took {:?}", i, duration);
    }
    info!("⚡ Concurrent operations: 10 completed in {:?}", concurrent_duration);
    
    // CLEANUP
    index.close().await;
    
    let total_duration = test_start.elapsed();
    info!("🎉 LARGE PROJECT TEST PASSED: 150 files handled successfully in {:?}", total_duration);
    info!("📊 FINAL STATS: {} files, {} symbols, indexing: {:?}, total: {:?}", 
          stats.file_count, stats.symbol_count, indexing_duration, total_duration);
    
    Ok(())
}

/// Generate realistic Gren files with cross-module dependencies
fn generate_realistic_gren_files(count: usize) -> Vec<(Url, String)> {
    let mut files = Vec::new();
    
    for i in 0..count {
        let uri = Url::parse(&format!("file:///project/module_{}.gren", i)).unwrap();
        let content = generate_gren_module_content(i, count);
        files.push((uri, content));
    }
    
    files
}

/// Generate realistic Gren module content with symbols and imports
fn generate_gren_module_content(module_id: usize, total_modules: usize) -> String {
    let mut content = format!("module Module{} exposing (..)\n\n", module_id);
    
    // Add imports to other modules (creating cross-references)
    let import_count = std::cmp::min(3, total_modules / 10);
    for j in 0..import_count {
        let import_target = (module_id + j + 1) % total_modules;
        content.push_str(&format!(
            "import Module{} exposing (commonFunction)\n", 
            import_target
        ));
    }
    
    if import_count > 0 {
        content.push_str("import Dict as Dictionary\n");
        content.push_str("import Array\n\n");
    } else {
        content.push_str("\n");
    }
    
    // Add type definitions
    content.push_str(&format!(
        r#"type Status{} = Loading | Success String | Error String

type alias Config{} = {{ debug : Bool, timeout : Int }}

type SharedType = Data{} String | Empty{}
        "#,
        module_id, module_id, module_id, module_id
    ));
    
    // Add function declarations and definitions
    content.push_str(&format!(
        r#"
commonFunction : String -> String
commonFunction input = 
    "Module{}_" ++ input

calculateValue{} : Int -> Int -> String
calculateValue{} a b = 
    String.fromInt (a + b)

processData{} : Config{} -> String -> String  
processData{} config input =
    if config.debug then
        "DEBUG: " ++ commonFunction input
    else
        calculateValue{} 42 (String.length input)

helperFunction{} : Array String -> String
helperFunction{} arr =
    Array.foldl String.append "" arr

constantValue{} : Int
constantValue{} = {}

recordExample{} : {{ name : String, value : Int }}
recordExample{} = {{ name = "Module{}", value = constantValue{} }}
        "#,
        module_id,
        module_id, module_id,
        module_id, module_id, module_id,
        module_id,
        module_id, module_id,
        module_id, module_id, module_id * 10 + 1,
        module_id, module_id, module_id, module_id
    ));
    
    // Add some pattern matching and when expressions
    content.push_str(&format!(
        r#"
processStatus{} : Status{} -> String
processStatus{} status =
    when status is
        Loading -> "loading..."
        Success msg -> "success: " ++ msg
        Error err -> "error: " ++ err

updateConfig{} : Config{} -> Config{}
updateConfig{} config = 
    {{ config | timeout = config.timeout + 1 }}
        "#,
        module_id, module_id, module_id,
        module_id, module_id, module_id, module_id
    ));
    
    content
}

/// Test memory efficiency with bounded caches
#[tokio::test] 
async fn test_memory_bounded_caches() -> Result<()> {
    use gren_lsp::performance::PerformanceManager;
    
    let perf_manager = PerformanceManager::new(50, 10); // Small caches for testing
    
    // Fill reference cache beyond capacity
    for i in 0..100 {
        let symbol_name = format!("symbol_{}", i);
        let references = vec![]; // Empty for test
        perf_manager.cache_references(symbol_name, references, Duration::from_secs(60)).await;
    }
    
    let stats = perf_manager.get_performance_stats().await;
    
    // VERIFY: Cache size is bounded
    assert!(stats.reference_cache.size <= 50, 
            "Reference cache size {} exceeds limit 50", stats.reference_cache.size);
    
    info!("✅ MEMORY TEST: Cache bounded at {} items (limit: 50)", stats.reference_cache.size);
    
    Ok(())
}

/// Test incremental file updates don't break performance
#[tokio::test]
async fn test_incremental_updates_performance() -> Result<()> {
    let index = SymbolIndex::new_in_memory(std::env::temp_dir()).await?;
    
    // Index initial set of files
    let initial_files = generate_realistic_gren_files(50);
    for (uri, content) in &initial_files {
        index.index_file(uri, content).await?;
    }
    
    // Update files incrementally and measure performance
    for (i, (uri, _)) in initial_files.iter().enumerate().take(10) {
        let updated_content = generate_gren_module_content(i + 1000, 1050); // Different content
        
        let update_start = Instant::now();
        index.index_file(uri, &updated_content).await?;
        let update_duration = update_start.elapsed();
        
        // VERIFY: Incremental updates are fast (Epic 3 Story 3 requirement)
        assert!(update_duration < Duration::from_millis(100), 
                "File update took {:?}, exceeding 100ms target", update_duration);
    }
    
    let final_stats = index.get_stats().await?;
    assert_eq!(final_stats.file_count, 50, "File count should remain the same");
    
    index.close().await;
    info!("✅ INCREMENTAL UPDATES: All updates completed within 100ms target");
    
    Ok(())
}