//! Incremental Compilation & Caching
//!
//! Hash-based compilation caching:
//! - Computes content hashes of source files
//! - Caches parse trees, IR modules, and compiled objects
//! - Invalidates cache entries when source changes
//! - Tracks dependency graphs to cascade invalidation

use std::collections::HashMap;

/// Content hash (simplified — using a fast string hash)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash(u64);

impl ContentHash {
    /// Compute hash from source text using FNV-1a
    pub fn from_source(source: &str) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in source.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        ContentHash(hash)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// The state of a cached compilation unit
#[derive(Debug, Clone)]
pub enum CacheState {
    /// Fresh — compiled output matches source
    Fresh,
    /// Stale — source has changed since last compile
    Stale,
    /// Missing — no cache entry
    Missing,
}

/// A cached module entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub module_name: String,
    pub source_hash: ContentHash,
    pub dependencies: Vec<String>,
    pub compiled_at: u64,  // epoch-style counter
}

/// Incremental compilation cache
pub struct IncrementalCache {
    entries: HashMap<String, CacheEntry>,
    epoch: u64,
    hit_count: u64,
    miss_count: u64,
}

impl IncrementalCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            epoch: 0,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Check if a module needs recompilation
    pub fn check(&mut self, name: &str, source: &str) -> CacheState {
        let current_hash = ContentHash::from_source(source);

        match self.entries.get(name) {
            Some(entry) if entry.source_hash == current_hash => {
                self.hit_count += 1;
                CacheState::Fresh
            }
            Some(_) => {
                self.miss_count += 1;
                CacheState::Stale
            }
            None => {
                self.miss_count += 1;
                CacheState::Missing
            }
        }
    }

    /// Mark a module as freshly compiled
    pub fn put(&mut self, name: &str, source: &str, deps: Vec<String>) {
        self.epoch += 1;
        self.entries.insert(name.to_string(), CacheEntry {
            module_name: name.to_string(),
            source_hash: ContentHash::from_source(source),
            dependencies: deps,
            compiled_at: self.epoch,
        });
    }

    /// Invalidate a module and its dependents
    pub fn invalidate(&mut self, name: &str) -> Vec<String> {
        let mut invalidated = Vec::new();

        if self.entries.remove(name).is_some() {
            invalidated.push(name.to_string());
        }

        // Find and invalidate all modules that depend on this one
        let dependents: Vec<String> = self.entries.iter()
            .filter(|(_, entry)| entry.dependencies.contains(&name.to_string()))
            .map(|(key, _)| key.clone())
            .collect();

        for dep in dependents {
            if self.entries.remove(&dep).is_some() {
                invalidated.push(dep);
            }
        }

        invalidated
    }

    /// Get all cached module names
    pub fn cached_modules(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// Get cache stats
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            hits: self.hit_count,
            misses: self.miss_count,
            epoch: self.epoch,
        }
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.epoch = 0;
        self.hit_count = 0;
        self.miss_count = 0;
    }

    /// Get dependencies for a module
    pub fn dependencies(&self, name: &str) -> Option<&[String]> {
        self.entries.get(name).map(|e| e.dependencies.as_slice())
    }

    /// Check if any dependency is stale
    pub fn has_stale_deps(&self, name: &str, sources: &HashMap<String, String>) -> bool {
        if let Some(entry) = self.entries.get(name) {
            for dep in &entry.dependencies {
                if let Some(source) = sources.get(dep) {
                    let current = ContentHash::from_source(source);
                    if let Some(dep_entry) = self.entries.get(dep) {
                        if dep_entry.source_hash != current {
                            return true;
                        }
                    } else {
                        return true; // dep not in cache
                    }
                }
            }
        }
        false
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub epoch: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

/// Dependency graph for incremental builds
pub struct DepGraph {
    edges: HashMap<String, Vec<String>>,  // module -> its dependencies
}

impl DepGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.edges.entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    pub fn dependencies_of(&self, module: &str) -> &[String] {
        self.edges.get(module).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Topological sort — returns modules in build order.
    /// Returns Err if there's a cycle.
    pub fn topo_sort(&self) -> Result<Vec<String>, String> {
        let mut visited: HashMap<String, bool> = HashMap::new();
        let mut order = Vec::new();

        // Collect all nodes (both keys and dependency targets)
        let mut all_nodes: Vec<String> = Vec::new();
        for (key, deps) in &self.edges {
            if !all_nodes.contains(key) {
                all_nodes.push(key.clone());
            }
            for dep in deps {
                if !all_nodes.contains(dep) {
                    all_nodes.push(dep.clone());
                }
            }
        }

        for node in &all_nodes {
            if !visited.contains_key(node.as_str()) {
                self.dfs_topo(node, &mut visited, &mut order)?;
            }
        }

        // Post-order DFS naturally gives leaves (no-deps) first
        Ok(order)
    }

    fn dfs_topo(
        &self,
        node: &str,
        visited: &mut HashMap<String, bool>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        if let Some(&in_progress) = visited.get(node) {
            if in_progress {
                return Err(format!("Dependency cycle detected involving '{}'", node));
            }
            return Ok(());
        }

        visited.insert(node.to_string(), true);  // mark in-progress

        for dep in self.dependencies_of(node) {
            self.dfs_topo(dep, visited, order)?;
        }

        visited.insert(node.to_string(), false);  // mark done
        order.push(node.to_string());
        Ok(())
    }

    /// Get all modules
    pub fn modules(&self) -> Vec<&str> {
        self.edges.keys().map(|s| s.as_str()).collect()
    }

    /// Find reverse dependencies (what depends on a given module)
    pub fn reverse_deps(&self, module: &str) -> Vec<&str> {
        self.edges.iter()
            .filter(|(_, deps)| deps.iter().any(|d| d == module))
            .map(|(key, _)| key.as_str())
            .collect()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_deterministic() {
        let h1 = ContentHash::from_source("fn main() { 42 }");
        let h2 = ContentHash::from_source("fn main() { 42 }");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_content_hash_changes() {
        let h1 = ContentHash::from_source("fn main() { 42 }");
        let h2 = ContentHash::from_source("fn main() { 43 }");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_cache_miss_on_new() {
        let mut cache = IncrementalCache::new();
        let state = cache.check("main", "fn main() { 42 }");
        assert!(matches!(state, CacheState::Missing));
    }

    #[test]
    fn test_cache_hit_after_put() {
        let mut cache = IncrementalCache::new();
        cache.put("main", "fn main() { 42 }", vec![]);
        let state = cache.check("main", "fn main() { 42 }");
        assert!(matches!(state, CacheState::Fresh));
    }

    #[test]
    fn test_cache_stale_after_change() {
        let mut cache = IncrementalCache::new();
        cache.put("main", "fn main() { 42 }", vec![]);
        let state = cache.check("main", "fn main() { 43 }");
        assert!(matches!(state, CacheState::Stale));
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = IncrementalCache::new();
        cache.put("utils", "fn helper() -> i64 { 1 }", vec![]);
        cache.put("main", "fn main() { helper() }", vec!["utils".to_string()]);
        let invalidated = cache.invalidate("utils");
        assert!(invalidated.contains(&"utils".to_string()));
        assert!(invalidated.contains(&"main".to_string()));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = IncrementalCache::new();
        cache.put("a", "code a", vec![]);
        cache.put("b", "code b", vec![]);
        cache.clear();
        assert_eq!(cache.cached_modules().len(), 0);
    }

    #[test]
    fn test_cache_stats_initial() {
        let cache = IncrementalCache::new();
        let stats = cache.stats();
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_stats_tracking() {
        let mut cache = IncrementalCache::new();
        cache.put("main", "code", vec![]);
        cache.check("main", "code");       // hit
        cache.check("main", "new code");   // miss (stale)
        cache.check("other", "x");         // miss (missing)
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = IncrementalCache::new();
        cache.put("a", "code_a", vec![]);
        cache.check("a", "code_a");   // hit
        cache.check("a", "code_a");   // hit
        cache.check("b", "code_b");   // miss
        assert!((cache.stats().hit_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_hit_rate_empty() {
        let cache = IncrementalCache::new();
        assert_eq!(cache.stats().hit_rate(), 0.0);
    }

    #[test]
    fn test_dep_graph_simple() {
        let mut g = DepGraph::new();
        g.add_dependency("main", "utils");
        assert_eq!(g.dependencies_of("main"), &["utils"]);
    }

    #[test]
    fn test_dep_graph_topo_sort() {
        let mut g = DepGraph::new();
        g.add_dependency("main", "utils");
        g.add_dependency("utils", "core");
        let order = g.topo_sort().unwrap();
        // utils depends on core, so core must come before utils
        let utils_pos = order.iter().position(|x| x == "utils").unwrap();
        let core_pos = order.iter().position(|x| x == "core").unwrap();
        assert!(core_pos < utils_pos, "core ({}) should come before utils ({})", core_pos, utils_pos);
    }

    #[test]
    fn test_dep_graph_cycle_detection() {
        let mut g = DepGraph::new();
        g.add_dependency("a", "b");
        g.add_dependency("b", "c");
        g.add_dependency("c", "a");
        let result = g.topo_sort();
        assert!(result.is_err());
    }

    #[test]
    fn test_dep_graph_reverse_deps() {
        let mut g = DepGraph::new();
        g.add_dependency("main", "utils");
        g.add_dependency("tests", "utils");
        let rdeps = g.reverse_deps("utils");
        assert!(rdeps.contains(&"main"));
        assert!(rdeps.contains(&"tests"));
    }

    #[test]
    fn test_dep_graph_no_deps() {
        let g = DepGraph::new();
        assert!(g.dependencies_of("nonexistent").is_empty());
    }

    #[test]
    fn test_cache_dependencies() {
        let mut cache = IncrementalCache::new();
        cache.put("main", "fn main() {}", vec!["lib".to_string(), "utils".to_string()]);
        let deps = cache.dependencies("main").unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"lib".to_string()));
    }

    #[test]
    fn test_cache_epoch_incrementing() {
        let mut cache = IncrementalCache::new();
        cache.put("a", "code", vec![]);
        cache.put("b", "code", vec![]);
        let stats = cache.stats();
        assert_eq!(stats.epoch, 2);
    }

    #[test]
    fn test_hash_empty_string() {
        let h = ContentHash::from_source("");
        assert_ne!(h.value(), 0);
    }

    #[test]
    fn test_cache_invalidation_no_deps() {
        let mut cache = IncrementalCache::new();
        cache.put("standalone", "code", vec![]);
        let invalidated = cache.invalidate("standalone");
        assert_eq!(invalidated.len(), 1);
        assert_eq!(invalidated[0], "standalone");
    }

    #[test]
    fn test_stale_deps_detection() {
        let mut cache = IncrementalCache::new();
        cache.put("utils", "original", vec![]);
        cache.put("main", "fn main() {}", vec!["utils".to_string()]);

        let mut sources = HashMap::new();
        sources.insert("utils".to_string(), "modified".to_string());

        assert!(cache.has_stale_deps("main", &sources));
    }
}
