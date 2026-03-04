//! Build System — Build Graph, Parallel Compilation, Content-Addressed Cache
//!
//! Provides a DAG-based build graph for compilation units, content-addressed
//! caching (SHA-256 keyed), work-stealing parallel scheduling, critical path
//! analysis, build profiling, and remote compilation protocol primitives.

use std::collections::{HashMap, HashSet, VecDeque};

// ── Content Hash ─────────────────────────────────────────────────────

/// SHA-256-based content hash for cache keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Compute SHA-256 hash of bytes (portable, no-dependency implementation).
    pub fn compute(data: &[u8]) -> Self {
        // SHA-256 constants
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
            0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
            0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
            0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
            0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
            0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
            0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
            0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
        ];

        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
            0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];

        // Padding
        let bit_len = (data.len() as u64) * 8;
        let mut padded = data.to_vec();
        padded.push(0x80);
        while (padded.len() % 64) != 56 {
            padded.push(0);
        }
        padded.extend_from_slice(&bit_len.to_be_bytes());

        // Process 512-bit blocks
        for chunk in padded.chunks_exact(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    chunk[i * 4],
                    chunk[i * 4 + 1],
                    chunk[i * 4 + 2],
                    chunk[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7)
                    ^ w[i - 15].rotate_right(18)
                    ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17)
                    ^ w[i - 2].rotate_right(19)
                    ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }

            let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
                (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);

                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }

            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }

        let mut result = [0u8; 32];
        for (i, &val) in h.iter().enumerate() {
            result[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
        }
        ContentHash(result)
    }

    /// Hex representation.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Parse from hex string.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(ContentHash(bytes))
    }
}

// ── Build Unit ───────────────────────────────────────────────────────

/// Status of a build unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStatus {
    Pending,
    Ready,
    Building,
    Succeeded,
    Failed,
    Cached,
    Skipped,
}

/// A single compilation unit in the build graph.
#[derive(Debug, Clone)]
pub struct BuildUnit {
    pub id: usize,
    pub name: String,
    /// Source content hash for cache lookup.
    pub content_hash: ContentHash,
    /// Dependencies (IDs of units that must build first).
    pub dependencies: Vec<usize>,
    /// Build status.
    pub status: BuildStatus,
    /// Time to build (nanoseconds), once completed.
    pub build_time_ns: u64,
    /// Output artifact hash (if succeeded).
    pub output_hash: Option<ContentHash>,
    /// Error message (if failed).
    pub error: Option<String>,
}

impl BuildUnit {
    pub fn new(id: usize, name: &str, source: &[u8]) -> Self {
        Self {
            id,
            name: name.to_string(),
            content_hash: ContentHash::compute(source),
            dependencies: Vec::new(),
            status: BuildStatus::Pending,
            build_time_ns: 0,
            output_hash: None,
            error: None,
        }
    }

    pub fn add_dependency(&mut self, dep_id: usize) {
        if !self.dependencies.contains(&dep_id) {
            self.dependencies.push(dep_id);
        }
    }

    /// Check if all dependencies have succeeded/cached.
    pub fn deps_satisfied(&self, units: &[BuildUnit]) -> bool {
        self.dependencies.iter().all(|&dep_id| {
            units.get(dep_id).map_or(false, |u| {
                matches!(u.status, BuildStatus::Succeeded | BuildStatus::Cached | BuildStatus::Skipped)
            })
        })
    }
}

// ── Build Graph ──────────────────────────────────────────────────────

/// DAG of build units with topological ordering and scheduling.
#[derive(Debug, Clone)]
pub struct BuildGraph {
    pub units: Vec<BuildUnit>,
    unit_by_name: HashMap<String, usize>,
}

impl BuildGraph {
    pub fn new() -> Self {
        Self {
            units: Vec::new(),
            unit_by_name: HashMap::new(),
        }
    }

    /// Add a build unit. Returns its ID.
    pub fn add_unit(&mut self, name: &str, source: &[u8]) -> usize {
        let id = self.units.len();
        let unit = BuildUnit::new(id, name, source);
        self.unit_by_name.insert(name.to_string(), id);
        self.units.push(unit);
        id
    }

    /// Add a dependency edge: `from` depends on `to`.
    pub fn add_dependency(&mut self, from: usize, to: usize) -> bool {
        if from >= self.units.len() || to >= self.units.len() || from == to {
            return false;
        }
        self.units[from].add_dependency(to);
        true
    }

    /// Look up a unit by name.
    pub fn unit_by_name(&self, name: &str) -> Option<usize> {
        self.unit_by_name.get(name).copied()
    }

    /// Detect cycles using Kahn's algorithm.
    pub fn has_cycle(&self) -> bool {
        let n = self.units.len();
        let mut in_degree = vec![0usize; n];
        for unit in &self.units {
            for &dep in &unit.dependencies {
                if dep < n {
                    // dep is depended on by unit, but in_degree tracks
                    // how many deps each unit has that must complete first
                }
            }
        }
        // Build adjacency: dependency → dependents
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for unit in &self.units {
            in_degree[unit.id] = unit.dependencies.len();
            for &dep in &unit.dependencies {
                if dep < n {
                    adj[dep].push(unit.id);
                }
            }
        }

        let mut queue: VecDeque<usize> = VecDeque::new();
        for i in 0..n {
            if in_degree[i] == 0 {
                queue.push_back(i);
            }
        }

        let mut visited = 0;
        while let Some(u) = queue.pop_front() {
            visited += 1;
            for &v in &adj[u] {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
        visited != n
    }

    /// Topological sort (Kahn's algorithm). Returns None if cyclic.
    pub fn topological_sort(&self) -> Option<Vec<usize>> {
        let n = self.units.len();
        let mut in_degree = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

        for unit in &self.units {
            in_degree[unit.id] = unit.dependencies.len();
            for &dep in &unit.dependencies {
                if dep < n {
                    adj[dep].push(unit.id);
                }
            }
        }

        let mut queue: VecDeque<usize> = VecDeque::new();
        for i in 0..n {
            if in_degree[i] == 0 {
                queue.push_back(i);
            }
        }

        let mut order = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            order.push(u);
            for &v in &adj[u] {
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }

        if order.len() == n {
            Some(order)
        } else {
            None
        }
    }

    /// Find units that are ready to build (all deps satisfied, status pending).
    pub fn ready_units(&self) -> Vec<usize> {
        self.units
            .iter()
            .filter(|u| u.status == BuildStatus::Pending && u.deps_satisfied(&self.units))
            .map(|u| u.id)
            .collect()
    }

    /// Mark a unit as building.
    pub fn start_build(&mut self, id: usize) {
        if id < self.units.len() {
            self.units[id].status = BuildStatus::Building;
        }
    }

    /// Mark a unit as succeeded with build time and output hash.
    pub fn complete_build(&mut self, id: usize, time_ns: u64, output: &[u8]) {
        if id < self.units.len() {
            self.units[id].status = BuildStatus::Succeeded;
            self.units[id].build_time_ns = time_ns;
            self.units[id].output_hash = Some(ContentHash::compute(output));
        }
    }

    /// Mark a unit as failed.
    pub fn fail_build(&mut self, id: usize, error: &str) {
        if id < self.units.len() {
            self.units[id].status = BuildStatus::Failed;
            self.units[id].error = Some(error.to_string());
        }
    }

    /// Mark a unit as cached (no rebuild needed).
    pub fn mark_cached(&mut self, id: usize) {
        if id < self.units.len() {
            self.units[id].status = BuildStatus::Cached;
        }
    }

    /// Total build time across all units.
    pub fn total_build_time_ns(&self) -> u64 {
        self.units.iter().map(|u| u.build_time_ns).sum()
    }

    /// Number of succeeded/cached units.
    pub fn completed_count(&self) -> usize {
        self.units
            .iter()
            .filter(|u| matches!(u.status, BuildStatus::Succeeded | BuildStatus::Cached))
            .count()
    }

    /// Number of failed units.
    pub fn failed_count(&self) -> usize {
        self.units
            .iter()
            .filter(|u| u.status == BuildStatus::Failed)
            .count()
    }

    /// Export build graph as DOT.
    pub fn to_dot(&self) -> String {
        let mut s = String::from("digraph build {\n  rankdir=TB;\n");
        for unit in &self.units {
            let color = match unit.status {
                BuildStatus::Succeeded => "#4ade80",
                BuildStatus::Cached => "#60a5fa",
                BuildStatus::Failed => "#f87171",
                BuildStatus::Building => "#fbbf24",
                _ => "#e5e7eb",
            };
            s.push_str(&format!(
                "  u{} [label=\"{}\" style=filled fillcolor=\"{}\" shape=box];\n",
                unit.id, unit.name, color
            ));
            for &dep in &unit.dependencies {
                s.push_str(&format!("  u{} -> u{};\n", dep, unit.id));
            }
        }
        s.push_str("}\n");
        s
    }
}

impl Default for BuildGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ── Build Cache ──────────────────────────────────────────────────────

/// Content-addressed compilation cache.
#[derive(Debug, Clone, Default)]
pub struct BuildCache {
    /// Maps content hash → cached output hash.
    entries: HashMap<ContentHash, CacheEntry>,
    pub hits: u64,
    pub misses: u64,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub output_hash: ContentHash,
    pub build_time_ns: u64,
    pub timestamp: u64,
}

impl BuildCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a result in the cache.
    pub fn store(&mut self, input_hash: ContentHash, output_hash: ContentHash, build_time_ns: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.entries.insert(
            input_hash,
            CacheEntry {
                output_hash,
                build_time_ns,
                timestamp: now,
            },
        );
    }

    /// Look up a cached result.
    pub fn lookup(&mut self, input_hash: &ContentHash) -> Option<&CacheEntry> {
        if let Some(entry) = self.entries.get(input_hash) {
            self.hits += 1;
            Some(entry)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Hit rate as percentage.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        (self.hits as f64 / total as f64) * 100.0
    }

    /// Number of entries.
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Evict entries older than `max_age_secs`.
    pub fn evict_older_than(&mut self, max_age_secs: u64) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let before = self.entries.len();
        self.entries
            .retain(|_, entry| now - entry.timestamp < max_age_secs);
        before - self.entries.len()
    }

    /// Time saved by cache hits (sum of cached build_time_ns).
    pub fn time_saved_ns(&self) -> u64 {
        self.entries.values().map(|e| e.build_time_ns).sum::<u64>() * self.hits / self.entries.len().max(1) as u64
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

// ── Work-Stealing Scheduler ──────────────────────────────────────────

/// Simple work-stealing scheduler for parallel compilation.
#[derive(Debug, Clone)]
pub struct WorkStealingScheduler {
    pub num_workers: usize,
    /// Per-worker queues.
    pub queues: Vec<VecDeque<usize>>,
    /// Which worker is assigned to each unit.
    pub assignments: HashMap<usize, usize>,
    pub completed: Vec<usize>,
}

impl WorkStealingScheduler {
    pub fn new(num_workers: usize) -> Self {
        Self {
            num_workers: num_workers.max(1),
            queues: vec![VecDeque::new(); num_workers.max(1)],
            assignments: HashMap::new(),
            completed: Vec::new(),
        }
    }

    /// Assign a unit to the least-loaded worker.
    pub fn schedule(&mut self, unit_id: usize) {
        let min_worker = (0..self.num_workers)
            .min_by_key(|&w| self.queues[w].len())
            .unwrap_or(0);
        self.queues[min_worker].push_back(unit_id);
        self.assignments.insert(unit_id, min_worker);
    }

    /// Get next work item for a specific worker. If local queue empty, steal.
    pub fn next_for_worker(&mut self, worker: usize) -> Option<usize> {
        // Try local queue first
        if let Some(id) = self.queues[worker].pop_front() {
            return Some(id);
        }
        // Try stealing from the busiest other worker
        let busiest = (0..self.num_workers)
            .filter(|&w| w != worker && !self.queues[w].is_empty())
            .max_by_key(|&w| self.queues[w].len());

        if let Some(victim) = busiest {
            self.queues[victim].pop_back() // Steal from the back
        } else {
            None
        }
    }

    /// Mark a unit as completed.
    pub fn complete(&mut self, unit_id: usize) {
        self.completed.push(unit_id);
        self.assignments.remove(&unit_id);
    }

    /// All work done?
    pub fn all_done(&self) -> bool {
        self.queues.iter().all(|q| q.is_empty()) && self.assignments.is_empty()
    }

    /// Total pending work items.
    pub fn pending_count(&self) -> usize {
        self.queues.iter().map(|q| q.len()).sum()
    }
}

// ── Critical Path Analysis ───────────────────────────────────────────

/// Analyzes the build graph to find the critical path (longest path).
pub struct CriticalPathAnalyzer;

impl CriticalPathAnalyzer {
    /// Find the critical path — the longest path through the build graph.
    /// Returns (path, total_time_ns).
    pub fn analyze(graph: &BuildGraph) -> (Vec<usize>, u64) {
        let n = graph.units.len();
        if n == 0 {
            return (vec![], 0);
        }

        let order = match graph.topological_sort() {
            Some(o) => o,
            None => return (vec![], 0), // Cyclic
        };

        // dist[i] = longest path ending at i
        let mut dist = vec![0u64; n];
        let mut prev = vec![None::<usize>; n];

        for &u in &order {
            let unit = &graph.units[u];
            let unit_time = unit.build_time_ns.max(1); // min 1ns to distinguish
            for &dep in &unit.dependencies {
                let through_dep = dist[dep] + unit_time;
                if through_dep > dist[u] {
                    dist[u] = through_dep;
                    prev[u] = Some(dep);
                }
            }
            if unit.dependencies.is_empty() {
                dist[u] = unit_time;
            }
        }

        // Find the node with the longest path
        let end = (0..n).max_by_key(|&i| dist[i]).unwrap_or(0);
        let total_time = dist[end];

        // Trace back
        let mut path = vec![end];
        let mut current = end;
        while let Some(p) = prev[current] {
            path.push(p);
            current = p;
        }
        path.reverse();

        (path, total_time)
    }

    /// Maximum parallelism available at each level.
    pub fn parallelism_profile(graph: &BuildGraph) -> Vec<usize> {
        let order = match graph.topological_sort() {
            Some(o) => o,
            None => return vec![],
        };

        let n = graph.units.len();
        let mut level = vec![0usize; n];

        for &u in &order {
            let max_dep_level = graph.units[u]
                .dependencies
                .iter()
                .map(|&d| level[d] + 1)
                .max()
                .unwrap_or(0);
            level[u] = max_dep_level;
        }

        let max_level = level.iter().copied().max().unwrap_or(0);
        let mut profile = vec![0usize; max_level + 1];
        for &l in &level {
            profile[l] += 1;
        }
        profile
    }
}

// ── Build Profile Report ─────────────────────────────────────────────

/// Build profiling report.
#[derive(Debug, Clone)]
pub struct BuildReport {
    pub total_units: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cached: usize,
    pub total_time_ns: u64,
    pub critical_path_ns: u64,
    pub cache_hit_rate: f64,
    pub parallelism: Vec<usize>,
    pub slowest_units: Vec<(String, u64)>,
}

impl BuildReport {
    pub fn generate(graph: &BuildGraph, cache: &BuildCache) -> Self {
        let (_, critical_ns) = CriticalPathAnalyzer::analyze(graph);
        let parallelism = CriticalPathAnalyzer::parallelism_profile(graph);

        let mut slowest: Vec<_> = graph
            .units
            .iter()
            .filter(|u| u.build_time_ns > 0)
            .map(|u| (u.name.clone(), u.build_time_ns))
            .collect();
        slowest.sort_by(|a, b| b.1.cmp(&a.1));
        slowest.truncate(10);

        Self {
            total_units: graph.units.len(),
            succeeded: graph
                .units
                .iter()
                .filter(|u| u.status == BuildStatus::Succeeded)
                .count(),
            failed: graph.failed_count(),
            cached: graph
                .units
                .iter()
                .filter(|u| u.status == BuildStatus::Cached)
                .count(),
            total_time_ns: graph.total_build_time_ns(),
            critical_path_ns: critical_ns,
            cache_hit_rate: cache.hit_rate(),
            parallelism,
            slowest_units: slowest,
        }
    }

    /// Format as human-readable text.
    pub fn to_text(&self) -> String {
        let mut s = String::from("=== Build Report ===\n");
        s.push_str(&format!(
            "Units: {} total, {} succeeded, {} cached, {} failed\n",
            self.total_units, self.succeeded, self.cached, self.failed
        ));
        s.push_str(&format!(
            "Total time: {:.2}ms, Critical path: {:.2}ms\n",
            self.total_time_ns as f64 / 1e6,
            self.critical_path_ns as f64 / 1e6
        ));
        s.push_str(&format!("Cache hit rate: {:.1}%\n", self.cache_hit_rate));

        if !self.parallelism.is_empty() {
            let max_par = self.parallelism.iter().max().unwrap_or(&0);
            s.push_str(&format!("Max parallelism: {}x\n", max_par));
        }

        if !self.slowest_units.is_empty() {
            s.push_str("Slowest units:\n");
            for (name, ns) in &self.slowest_units {
                s.push_str(&format!("  {} — {:.2}ms\n", name, *ns as f64 / 1e6));
            }
        }
        s
    }
}

// ── Remote Compilation Protocol ──────────────────────────────────────

/// A serializable compilation task for remote execution.
#[derive(Debug, Clone)]
pub struct RemoteTask {
    pub id: u64,
    pub unit_name: String,
    pub source_hash: ContentHash,
    pub dependencies: Vec<ContentHash>,
    pub target_triple: String,
    pub optimization_level: u8,
}

/// Response from a remote build node.
#[derive(Debug, Clone)]
pub enum RemoteResult {
    Success {
        task_id: u64,
        output_hash: ContentHash,
        time_ns: u64,
    },
    Failure {
        task_id: u64,
        error: String,
    },
    CacheHit {
        task_id: u64,
        output_hash: ContentHash,
    },
}

/// Simple remote build node registry.
#[derive(Debug, Clone, Default)]
pub struct BuildNodeRegistry {
    pub nodes: Vec<BuildNode>,
}

#[derive(Debug, Clone)]
pub struct BuildNode {
    pub id: usize,
    pub address: String,
    pub capacity: usize,
    pub current_load: usize,
    pub available: bool,
}

impl BuildNodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, address: &str, capacity: usize) -> usize {
        let id = self.nodes.len();
        self.nodes.push(BuildNode {
            id,
            address: address.to_string(),
            capacity,
            current_load: 0,
            available: true,
        });
        id
    }

    /// Find the best available node (lowest load relative to capacity).
    pub fn best_node(&self) -> Option<usize> {
        self.nodes
            .iter()
            .filter(|n| n.available && n.current_load < n.capacity)
            .min_by_key(|n| n.current_load * 100 / n.capacity.max(1))
            .map(|n| n.id)
    }

    /// Total available capacity.
    pub fn total_capacity(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.available)
            .map(|n| n.capacity.saturating_sub(n.current_load))
            .sum()
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Content Hash ─────────────────────────────────────────────────

    #[test]
    fn test_sha256_empty() {
        let hash = ContentHash::compute(b"");
        assert_eq!(
            hash.to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_hello() {
        let hash = ContentHash::compute(b"hello");
        assert_eq!(
            hash.to_hex(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sha256_deterministic() {
        let h1 = ContentHash::compute(b"test data");
        let h2 = ContentHash::compute(b"test data");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_sha256_different() {
        let h1 = ContentHash::compute(b"abc");
        let h2 = ContentHash::compute(b"abd");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_sha256_hex_roundtrip() {
        let hash = ContentHash::compute(b"roundtrip");
        let hex = hash.to_hex();
        let parsed = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(hash, parsed);
    }

    // ── Build Graph ──────────────────────────────────────────────────

    #[test]
    fn test_build_graph_add_units() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("main.sl", b"fn main() {}");
        let b = g.add_unit("lib.sl", b"fn helper() {}");
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(g.units.len(), 2);
    }

    #[test]
    fn test_build_graph_dependencies() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("main.sl", b"main");
        let b = g.add_unit("lib.sl", b"lib");
        g.add_dependency(a, b); // main depends on lib
        assert!(g.units[a].dependencies.contains(&b));
    }

    #[test]
    fn test_build_graph_topo_sort() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        let b = g.add_unit("b", b"b");
        let c = g.add_unit("c", b"c");
        g.add_dependency(c, b);
        g.add_dependency(b, a);
        let order = g.topological_sort().unwrap();
        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn test_build_graph_cycle_detection() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        let b = g.add_unit("b", b"b");
        g.add_dependency(a, b);
        g.add_dependency(b, a);
        assert!(g.has_cycle());
        assert!(g.topological_sort().is_none());
    }

    #[test]
    fn test_build_graph_no_cycle() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        let b = g.add_unit("b", b"b");
        g.add_dependency(b, a);
        assert!(!g.has_cycle());
    }

    #[test]
    fn test_build_graph_ready_units() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        let b = g.add_unit("b", b"b");
        g.add_dependency(b, a);
        let ready = g.ready_units();
        assert_eq!(ready, vec![a]); // Only 'a' has no deps
    }

    #[test]
    fn test_build_graph_workflow() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        let b = g.add_unit("b", b"b");
        g.add_dependency(b, a);

        // Build 'a'
        g.start_build(a);
        g.complete_build(a, 1000, b"output_a");
        assert_eq!(g.units[a].status, BuildStatus::Succeeded);

        // Now 'b' should be ready
        assert!(g.ready_units().contains(&b));
        g.start_build(b);
        g.complete_build(b, 2000, b"output_b");
        assert_eq!(g.completed_count(), 2);
    }

    #[test]
    fn test_build_graph_dot() {
        let mut g = BuildGraph::new();
        g.add_unit("main", b"main");
        let dot = g.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("main"));
    }

    // ── Build Cache ──────────────────────────────────────────────────

    #[test]
    fn test_cache_store_lookup() {
        let mut cache = BuildCache::new();
        let input = ContentHash::compute(b"source");
        let output = ContentHash::compute(b"artifact");
        cache.store(input.clone(), output.clone(), 5000);

        let entry = cache.lookup(&input).unwrap();
        assert_eq!(entry.output_hash, output);
        assert_eq!(entry.build_time_ns, 5000);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = BuildCache::new();
        let missing = ContentHash::compute(b"missing");
        assert!(cache.lookup(&missing).is_none());
        assert_eq!(cache.misses, 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = BuildCache::new();
        let h = ContentHash::compute(b"src");
        cache.store(h.clone(), ContentHash::compute(b"out"), 100);
        cache.lookup(&h); // hit
        cache.lookup(&ContentHash::compute(b"miss")); // miss
        assert!((cache.hit_rate() - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = BuildCache::new();
        let h = ContentHash::compute(b"x");
        cache.store(h, ContentHash::compute(b"y"), 100);
        cache.clear();
        assert_eq!(cache.size(), 0);
    }

    // ── Work Stealing ────────────────────────────────────────────────

    #[test]
    fn test_scheduler_basic() {
        let mut sched = WorkStealingScheduler::new(2);
        sched.schedule(0);
        sched.schedule(1);
        sched.schedule(2);
        assert_eq!(sched.pending_count(), 3);
    }

    #[test]
    fn test_scheduler_work_stealing() {
        let mut sched = WorkStealingScheduler::new(2);
        // Put all work on worker 0
        sched.queues[0].push_back(10);
        sched.queues[0].push_back(20);
        sched.queues[0].push_back(30);

        // Worker 1 steals
        let stolen = sched.next_for_worker(1);
        assert!(stolen.is_some());
    }

    #[test]
    fn test_scheduler_completion() {
        let mut sched = WorkStealingScheduler::new(1);
        sched.schedule(0);
        let item = sched.next_for_worker(0).unwrap();
        sched.complete(item);
        assert!(sched.all_done());
    }

    // ── Critical Path ────────────────────────────────────────────────

    #[test]
    fn test_critical_path() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        let b = g.add_unit("b", b"b");
        let c = g.add_unit("c", b"c");
        g.add_dependency(b, a);
        g.add_dependency(c, b);
        g.units[a].build_time_ns = 100;
        g.units[b].build_time_ns = 200;
        g.units[c].build_time_ns = 50;

        let (path, _time) = CriticalPathAnalyzer::analyze(&g);
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn test_parallelism_profile() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        let b = g.add_unit("b", b"b");
        let c = g.add_unit("c", b"c");
        // b and c both depend on a → level 0: [a], level 1: [b, c]
        g.add_dependency(b, a);
        g.add_dependency(c, a);
        let profile = CriticalPathAnalyzer::parallelism_profile(&g);
        assert_eq!(profile, vec![1, 2]); // 1 at level 0, 2 at level 1
    }

    // ── Build Report ─────────────────────────────────────────────────

    #[test]
    fn test_build_report() {
        let mut g = BuildGraph::new();
        let a = g.add_unit("a", b"a");
        g.complete_build(a, 5000, b"out");
        let cache = BuildCache::new();
        let report = BuildReport::generate(&g, &cache);
        assert_eq!(report.total_units, 1);
        assert_eq!(report.succeeded, 1);
        let text = report.to_text();
        assert!(text.contains("Build Report"));
    }

    // ── Remote Nodes ─────────────────────────────────────────────────

    #[test]
    fn test_node_registry() {
        let mut reg = BuildNodeRegistry::new();
        reg.add_node("192.168.1.1:8080", 8);
        reg.add_node("192.168.1.2:8080", 4);
        assert_eq!(reg.total_capacity(), 12);
        let best = reg.best_node().unwrap();
        assert_eq!(best, 0); // Both at 0 load, first chosen
    }

    #[test]
    fn test_node_load_balancing() {
        let mut reg = BuildNodeRegistry::new();
        reg.add_node("node1", 4);
        reg.add_node("node2", 4);
        reg.nodes[0].current_load = 3;
        reg.nodes[1].current_load = 1;
        let best = reg.best_node().unwrap();
        assert_eq!(best, 1); // node2 has lower load
    }
}
