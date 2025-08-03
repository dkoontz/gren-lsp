use anyhow::Result;
use lru::LruCache;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::{Location, Url};
use tracing::{debug, info};

/// Performance-optimized cache for find references operations
#[derive(Debug)]
pub struct ReferenceCache {
    cache: LruCache<ReferenceCacheKey, CachedReferenceResult>,
    hit_count: u64,
    miss_count: u64,
}

/// Cache key for reference lookups
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ReferenceCacheKey {
    symbol_name: String,
    workspace_version: u64, // Invalidation key based on workspace changes
}

/// Cached reference result with timestamp for TTL
#[derive(Debug, Clone)]
pub struct CachedReferenceResult {
    references: Vec<Location>,
    created_at: Instant,
    ttl: Duration,
}

impl CachedReferenceResult {
    pub fn new(references: Vec<Location>, ttl: Duration) -> Self {
        Self {
            references,
            created_at: Instant::now(),
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    pub fn get_references(&self) -> &Vec<Location> {
        &self.references
    }
}

impl ReferenceCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            hit_count: 0,
            miss_count: 0,
        }
    }

    pub fn get(&mut self, key: &ReferenceCacheKey) -> Option<CachedReferenceResult> {
        if let Some(result) = self.cache.get(key) {
            if !result.is_expired() {
                self.hit_count += 1;
                return Some(result.clone());
            } else {
                // Remove expired entry will be handled below
            }
        }
        
        // Check for expired entry and remove it
        if self.cache.contains(key) {
            self.cache.pop(key);
        }
        
        self.miss_count += 1;
        None
    }

    pub fn put(&mut self, key: ReferenceCacheKey, result: CachedReferenceResult) {
        self.cache.put(key, result);
    }

    pub fn invalidate_workspace(&mut self, workspace_version: u64) {
        // Remove all entries with older workspace versions
        let keys_to_remove: Vec<_> = self.cache
            .iter()
            .filter(|(key, _)| key.workspace_version < workspace_version)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in keys_to_remove {
            self.cache.pop(&key);
        }
        
        debug!("Invalidated reference cache for workspace version {}", workspace_version);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.hit_count = 0;
        self.miss_count = 0;
    }

    pub fn get_stats(&self) -> CacheStats {
        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            (self.hit_count as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        CacheStats {
            capacity: self.cache.cap().get(),
            size: self.cache.len(),
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate,
        }
    }
}

/// Cache for parsed tree-sitter trees to avoid re-parsing
#[derive(Debug)]
pub struct ParseTreeCache {
    cache: LruCache<ParseTreeCacheKey, CachedParseTree>,
    hit_count: u64,
    miss_count: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParseTreeCacheKey {
    uri: Url,
    content_hash: u64,
}

impl Hash for ParseTreeCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uri.hash(state);
        self.content_hash.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct CachedParseTree {
    tree: tree_sitter::Tree,
    created_at: Instant,
}

impl ParseTreeCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            hit_count: 0,
            miss_count: 0,
        }
    }

    pub fn get(&mut self, key: &ParseTreeCacheKey) -> Option<tree_sitter::Tree> {
        if let Some(cached) = self.cache.get(key) {
            self.hit_count += 1;
            return Some(cached.tree.clone());
        }
        self.miss_count += 1;
        None
    }

    pub fn put(&mut self, key: ParseTreeCacheKey, tree: tree_sitter::Tree) {
        let cached = CachedParseTree {
            tree,
            created_at: Instant::now(),
        };
        self.cache.put(key, cached);
    }

    pub fn get_stats(&self) -> CacheStats {
        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            (self.hit_count as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        CacheStats {
            capacity: self.cache.cap().get(),
            size: self.cache.len(),
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub capacity: usize,
    pub size: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
}

/// Performance manager that coordinates all caching and optimization
#[derive(Debug)]
pub struct PerformanceManager {
    reference_cache: Arc<RwLock<ReferenceCache>>,
    parse_tree_cache: Arc<RwLock<ParseTreeCache>>,
    workspace_version: Arc<RwLock<u64>>,
}

impl PerformanceManager {
    pub fn new(reference_cache_size: usize, parse_tree_cache_size: usize) -> Self {
        Self {
            reference_cache: Arc::new(RwLock::new(ReferenceCache::new(reference_cache_size))),
            parse_tree_cache: Arc::new(RwLock::new(ParseTreeCache::new(parse_tree_cache_size))),
            workspace_version: Arc::new(RwLock::new(0)),
        }
    }

    /// Increment workspace version to invalidate caches when files change
    pub async fn increment_workspace_version(&self) {
        let mut version = self.workspace_version.write().await;
        *version += 1;
        let new_version = *version;
        drop(version);

        // Invalidate reference cache
        let mut ref_cache = self.reference_cache.write().await;
        ref_cache.invalidate_workspace(new_version);
        
        debug!("Incremented workspace version to {}", new_version);
    }

    /// Get current workspace version
    pub async fn get_workspace_version(&self) -> u64 {
        *self.workspace_version.read().await
    }

    /// Cache reference lookup result
    pub async fn cache_references(&self, symbol_name: String, references: Vec<Location>, ttl: Duration) {
        let workspace_version = self.get_workspace_version().await;
        let key = ReferenceCacheKey {
            symbol_name,
            workspace_version,
        };
        let result = CachedReferenceResult::new(references, ttl);
        
        let mut cache = self.reference_cache.write().await;
        cache.put(key, result);
    }

    /// Get cached reference result
    pub async fn get_cached_references(&self, symbol_name: &str) -> Option<Vec<Location>> {
        let workspace_version = self.get_workspace_version().await;
        let key = ReferenceCacheKey {
            symbol_name: symbol_name.to_string(),
            workspace_version,
        };
        
        let mut cache = self.reference_cache.write().await;
        cache.get(&key).map(|result| result.get_references().clone())
    }

    /// Cache parsed tree
    pub async fn cache_parse_tree(&self, uri: Url, content_hash: u64, tree: tree_sitter::Tree) {
        let key = ParseTreeCacheKey { uri, content_hash };
        let mut cache = self.parse_tree_cache.write().await;
        cache.put(key, tree);
    }

    /// Get cached parsed tree
    pub async fn get_cached_parse_tree(&self, uri: &Url, content_hash: u64) -> Option<tree_sitter::Tree> {
        let key = ParseTreeCacheKey {
            uri: uri.clone(),
            content_hash,
        };
        let mut cache = self.parse_tree_cache.write().await;
        cache.get(&key)
    }

    /// Get comprehensive performance statistics
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let ref_stats = self.reference_cache.read().await.get_stats();
        let tree_stats = self.parse_tree_cache.read().await.get_stats();
        let workspace_version = self.get_workspace_version().await;

        PerformanceStats {
            reference_cache: ref_stats,
            parse_tree_cache: tree_stats,
            workspace_version,
        }
    }

    /// Clear all caches (useful for testing and memory management)
    pub async fn clear_all_caches(&self) {
        let mut ref_cache = self.reference_cache.write().await;
        ref_cache.clear();
        
        let mut tree_cache = self.parse_tree_cache.write().await;
        tree_cache.cache.clear();
        
        info!("Cleared all performance caches");
    }
}

/// Comprehensive performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub reference_cache: CacheStats,
    pub parse_tree_cache: CacheStats,
    pub workspace_version: u64,
}

/// Background task manager for async symbol indexing
pub struct BackgroundTaskManager {
    _handle: tokio::task::JoinHandle<()>,
}

impl BackgroundTaskManager {
    pub fn new() -> Self {
        let handle = tokio::spawn(async {
            // Background task for periodic cache cleanup and optimization
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                debug!("Background task: Performing periodic maintenance");
                
                // Future: Add cleanup logic, memory optimization, etc.
            }
        });
        
        Self { _handle: handle }
    }
}

/// Utility function to calculate content hash for cache keys
pub fn calculate_content_hash(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Performance optimization utilities
pub mod utils {
    use super::*;
    
    /// Batch database operations for better performance
    pub async fn batch_execute<F, Fut, T>(
        items: Vec<T>,
        batch_size: usize,
        operation: F,
    ) -> Result<()>
    where
        F: Fn(Vec<T>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
        T: Clone,
    {
        for chunk in items.chunks(batch_size) {
            operation(chunk.to_vec()).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reference_cache() {
        let mut cache = ReferenceCache::new(10);
        let key = ReferenceCacheKey {
            symbol_name: "test".to_string(),
            workspace_version: 1,
        };
        
        // Cache miss
        assert!(cache.get(&key).is_none());
        
        // Add to cache
        let references = vec![];
        let result = CachedReferenceResult::new(references, Duration::from_secs(60));
        cache.put(key.clone(), result);
        
        // Cache hit
        assert!(cache.get(&key).is_some());
        
        let stats = cache.get_stats();
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
        assert_eq!(stats.size, 1);
    }

    #[tokio::test]
    async fn test_performance_manager() {
        let manager = PerformanceManager::new(10, 5);
        
        // Test workspace version increment
        let initial_version = manager.get_workspace_version().await;
        manager.increment_workspace_version().await;
        let new_version = manager.get_workspace_version().await;
        assert_eq!(new_version, initial_version + 1);
        
        // Test reference caching
        let references = vec![];
        manager.cache_references("test".to_string(), references, Duration::from_secs(60)).await;
        
        let cached = manager.get_cached_references("test").await;
        assert!(cached.is_some());
        
        // Test stats
        let stats = manager.get_performance_stats().await;
        assert_eq!(stats.workspace_version, new_version);
    }

    #[test]
    fn test_content_hash() {
        let content1 = "hello world";
        let content2 = "hello world";
        let content3 = "different content";
        
        assert_eq!(calculate_content_hash(content1), calculate_content_hash(content2));
        assert_ne!(calculate_content_hash(content1), calculate_content_hash(content3));
    }
}