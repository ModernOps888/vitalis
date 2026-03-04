//! Distributed Build — package registry, distributed compilation,
//! content-addressed cache, hermetic builds, dependency vulnerability scanning.
//!
//! Extends `package_manager.rs` with multi-node compilation, a registry
//! protocol, lockfile pinning, and sandboxed hermetic build environments.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Package Registry ────────────────────────────────────────────────────

/// Package metadata in the registry.
#[derive(Debug, Clone)]
pub struct RegistryPackage {
    pub name: String,
    pub versions: Vec<PackageVersion>,
    pub owner: String,
    pub description: String,
    pub license: String,
    pub repository: String,
    pub keywords: Vec<String>,
}

/// A specific version of a package.
#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub version: String,
    pub checksum: String,
    pub dependencies: Vec<Dependency>,
    pub size_bytes: u64,
    pub yanked: bool,
    pub published_at: u64,
    pub rust_version: Option<String>,
}

/// Dependency specification.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub optional: bool,
    pub features: Vec<String>,
}

/// Registry client for package resolution.
#[derive(Debug, Clone)]
pub struct RegistryClient {
    pub registry_url: String,
    pub packages: HashMap<String, RegistryPackage>,
    pub cache_dir: String,
}

impl RegistryClient {
    pub fn new(url: &str, cache_dir: &str) -> Self {
        RegistryClient {
            registry_url: url.to_string(),
            packages: HashMap::new(),
            cache_dir: cache_dir.to_string(),
        }
    }

    /// Register a package in the local index.
    pub fn publish(&mut self, pkg: RegistryPackage) -> Result<(), String> {
        if pkg.name.is_empty() { return Err("Package name cannot be empty".into()); }
        if pkg.versions.is_empty() { return Err("Must have at least one version".into()); }
        self.packages.insert(pkg.name.clone(), pkg);
        Ok(())
    }

    /// Look up a package by name.
    pub fn lookup(&self, name: &str) -> Option<&RegistryPackage> {
        self.packages.get(name)
    }

    /// Find latest non-yanked version.
    pub fn latest_version(&self, name: &str) -> Option<&PackageVersion> {
        self.packages.get(name).and_then(|p| {
            p.versions.iter().rev().find(|v| !v.yanked)
        })
    }

    /// Resolve dependencies for a package version.
    pub fn resolve(&self, name: &str, version: &str) -> Result<Vec<ResolvedDep>, String> {
        let pkg = self.packages.get(name).ok_or_else(|| format!("Package {} not found", name))?;
        let ver = pkg.versions.iter().find(|v| v.version == version)
            .ok_or_else(|| format!("Version {} not found for {}", version, name))?;

        let mut resolved = Vec::new();
        for dep in &ver.dependencies {
            if let Some(dep_pkg) = self.latest_version(&dep.name) {
                resolved.push(ResolvedDep {
                    name: dep.name.clone(),
                    version: dep_pkg.version.clone(),
                    checksum: dep_pkg.checksum.clone(),
                });
            } else if !dep.optional {
                return Err(format!("Required dependency {} not found", dep.name));
            }
        }
        Ok(resolved)
    }
}

/// A resolved dependency with exact version.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: String,
    pub checksum: String,
}

// ── Vulnerability Scanning ──────────────────────────────────────────────

/// Known vulnerability entry.
#[derive(Debug, Clone)]
pub struct Vulnerability {
    pub id: String,
    pub package: String,
    pub affected_versions: Vec<String>,
    pub severity: VulnSeverity,
    pub description: String,
    pub patched_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VulnSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Vulnerability database.
#[derive(Debug, Clone)]
pub struct VulnDatabase {
    pub advisories: Vec<Vulnerability>,
}

impl VulnDatabase {
    pub fn new() -> Self { VulnDatabase { advisories: Vec::new() } }

    pub fn add_advisory(&mut self, vuln: Vulnerability) {
        self.advisories.push(vuln);
    }

    /// Scan dependencies for known vulnerabilities.
    pub fn scan(&self, deps: &[ResolvedDep]) -> Vec<VulnMatch> {
        let mut matches = Vec::new();
        for dep in deps {
            for adv in &self.advisories {
                if adv.package == dep.name && adv.affected_versions.contains(&dep.version) {
                    matches.push(VulnMatch {
                        dependency: dep.clone(),
                        vulnerability: adv.clone(),
                    });
                }
            }
        }
        matches
    }
}

/// A dependency matched against a vulnerability.
#[derive(Debug, Clone)]
pub struct VulnMatch {
    pub dependency: ResolvedDep,
    pub vulnerability: Vulnerability,
}

// ── Lockfile ────────────────────────────────────────────────────────────

/// Lockfile for reproducible builds.
#[derive(Debug, Clone)]
pub struct Lockfile {
    pub version: u32,
    pub entries: Vec<LockEntry>,
    pub checksum: String,
}

/// A single locked dependency.
#[derive(Debug, Clone)]
pub struct LockEntry {
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub source: String,
}

impl Lockfile {
    pub fn new() -> Self {
        Lockfile { version: 1, entries: Vec::new(), checksum: String::new() }
    }

    pub fn add_entry(&mut self, entry: LockEntry) {
        self.entries.push(entry);
        self.recompute_checksum();
    }

    pub fn find(&self, name: &str) -> Option<&LockEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    fn recompute_checksum(&mut self) {
        let mut hash: u64 = 0xcbf29ce484222325;
        for entry in &self.entries {
            for b in entry.name.bytes().chain(entry.version.bytes()).chain(entry.checksum.bytes()) {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        self.checksum = format!("{:016x}", hash);
    }

    /// Serialize to TOML-like text.
    pub fn to_text(&self) -> String {
        let mut out = format!("# vitalis.lock v{}\n# checksum: {}\n\n", self.version, self.checksum);
        for entry in &self.entries {
            out.push_str(&format!("[[package]]\nname = \"{}\"\nversion = \"{}\"\nchecksum = \"{}\"\nsource = \"{}\"\n\n",
                entry.name, entry.version, entry.checksum, entry.source));
        }
        out
    }
}

// ── Content-Addressed Cache ─────────────────────────────────────────────

/// Content-addressed compilation cache.
#[derive(Debug, Clone)]
pub struct ContentCache {
    pub entries: HashMap<String, CacheEntry>,
    pub max_size_bytes: u64,
    pub current_size_bytes: u64,
    pub hits: u64,
    pub misses: u64,
}

/// A cached compilation artifact.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub artifact_hash: String,
    pub size_bytes: u64,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
}

impl ContentCache {
    pub fn new(max_size_bytes: u64) -> Self {
        ContentCache {
            entries: HashMap::new(),
            max_size_bytes,
            current_size_bytes: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Compute content hash for an input (FNV-1a).
    pub fn content_hash(data: &[u8]) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in data {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }

    /// Look up a cached artifact.
    pub fn get(&mut self, key: &str) -> Option<&CacheEntry> {
        if self.entries.contains_key(key) {
            self.hits += 1;
            if let Some(entry) = self.entries.get_mut(key) {
                entry.access_count += 1;
                entry.last_accessed += 1;
            }
            self.entries.get(key)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store a compilation artifact.
    pub fn put(&mut self, key: String, artifact_hash: String, size_bytes: u64) {
        // Evict if necessary
        while self.current_size_bytes + size_bytes > self.max_size_bytes && !self.entries.is_empty() {
            self.evict_lru();
        }

        self.current_size_bytes += size_bytes;
        self.entries.insert(key.clone(), CacheEntry {
            key,
            artifact_hash,
            size_bytes,
            created_at: 0,
            last_accessed: 0,
            access_count: 0,
        });
    }

    /// Evict least-recently-used entry.
    fn evict_lru(&mut self) {
        if let Some(key) = self.entries.iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(k, _)| k.clone())
        {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_size_bytes = self.current_size_bytes.saturating_sub(entry.size_bytes);
            }
        }
    }

    /// Cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

// ── Distributed Compilation ─────────────────────────────────────────────

/// Node in a distributed compilation cluster.
#[derive(Debug, Clone)]
pub struct BuildNode {
    pub id: String,
    pub address: String,
    pub capacity: u32,       // max concurrent jobs
    pub running_jobs: u32,
    pub completed_jobs: u64,
    pub status: NodeStatus,
    pub platform: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    Available,
    Busy,
    Offline,
    Draining,
}

/// Distributed build coordinator.
#[derive(Debug, Clone)]
pub struct BuildCluster {
    pub nodes: Vec<BuildNode>,
    pub job_queue: Vec<BuildJob>,
    pub completed_jobs: Vec<CompletedJob>,
    pub cache: ContentCache,
}

/// A unit of work for distributed compilation.
#[derive(Debug, Clone)]
pub struct BuildJob {
    pub id: u64,
    pub source_hash: String,
    pub target: String,
    pub priority: u32,
    pub dependencies: Vec<String>,
}

/// Result of a completed build job.
#[derive(Debug, Clone)]
pub struct CompletedJob {
    pub job_id: u64,
    pub node_id: String,
    pub artifact_hash: String,
    pub duration_ms: u64,
    pub success: bool,
}

impl BuildCluster {
    pub fn new(cache_size: u64) -> Self {
        BuildCluster {
            nodes: Vec::new(),
            job_queue: Vec::new(),
            completed_jobs: Vec::new(),
            cache: ContentCache::new(cache_size),
        }
    }

    /// Add a build node to the cluster.
    pub fn add_node(&mut self, node: BuildNode) {
        self.nodes.push(node);
    }

    /// Submit a build job.
    pub fn submit_job(&mut self, job: BuildJob) -> u64 {
        let id = job.id;
        self.job_queue.push(job);
        id
    }

    /// Schedule next job to an available node. Returns (job_id, node_id).
    pub fn schedule_next(&mut self) -> Option<(u64, String)> {
        // Sort by priority (highest first)
        self.job_queue.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Find available node with least load
        let node_idx = self.nodes.iter().position(|n| {
            n.status == NodeStatus::Available && n.running_jobs < n.capacity
        });

        if let Some(ni) = node_idx {
            if let Some(job) = self.job_queue.first() {
                let job_id = job.id;
                let node_id = self.nodes[ni].id.clone();
                self.nodes[ni].running_jobs += 1;
                if self.nodes[ni].running_jobs >= self.nodes[ni].capacity {
                    self.nodes[ni].status = NodeStatus::Busy;
                }
                self.job_queue.remove(0);
                return Some((job_id, node_id));
            }
        }
        None
    }

    /// Mark a job as completed.
    pub fn complete_job(&mut self, job_id: u64, node_id: &str, artifact_hash: String, duration_ms: u64, success: bool) {
        // Update node
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
            node.running_jobs = node.running_jobs.saturating_sub(1);
            node.completed_jobs += 1;
            if node.status == NodeStatus::Busy && node.running_jobs < node.capacity {
                node.status = NodeStatus::Available;
            }
        }

        // Cache artifact if successful
        if success {
            self.cache.put(format!("job_{}", job_id), artifact_hash.clone(), 1024);
        }

        self.completed_jobs.push(CompletedJob { job_id, node_id: node_id.to_string(), artifact_hash, duration_ms, success });
    }

    /// Get cluster utilization.
    pub fn utilization(&self) -> f64 {
        let total_capacity: u32 = self.nodes.iter().map(|n| n.capacity).sum();
        let total_running: u32 = self.nodes.iter().map(|n| n.running_jobs).sum();
        if total_capacity == 0 { 0.0 } else { total_running as f64 / total_capacity as f64 }
    }
}

// ── Hermetic Build Environment ──────────────────────────────────────────

/// Hermetic build configuration for reproducibility.
#[derive(Debug, Clone)]
pub struct HermeticBuild {
    pub toolchain_version: String,
    pub target_triple: String,
    pub env_vars: HashMap<String, String>,
    pub sandbox_mode: SandboxMode,
    pub allowed_paths: Vec<String>,
    pub deterministic_seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SandboxMode {
    None,
    Filesystem,  // Restrict filesystem access
    Full,        // Restrict filesystem + network + process
}

impl HermeticBuild {
    pub fn new(toolchain: &str, target: &str) -> Self {
        HermeticBuild {
            toolchain_version: toolchain.to_string(),
            target_triple: target.to_string(),
            env_vars: HashMap::new(),
            sandbox_mode: SandboxMode::Filesystem,
            allowed_paths: Vec::new(),
            deterministic_seed: 42,
        }
    }

    /// Compute fingerprint for cache keying.
    pub fn fingerprint(&self) -> String {
        let mut data = Vec::new();
        data.extend(self.toolchain_version.as_bytes());
        data.extend(self.target_triple.as_bytes());
        let mut sorted_vars: Vec<_> = self.env_vars.iter().collect();
        sorted_vars.sort_by_key(|(k, _)| k.clone());
        for (k, v) in sorted_vars {
            data.extend(k.as_bytes());
            data.extend(v.as_bytes());
        }
        ContentCache::content_hash(&data)
    }

    /// Validate build environment matches the hermetic spec.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.toolchain_version.is_empty() {
            errors.push("Toolchain version must be specified".into());
        }
        if self.target_triple.is_empty() {
            errors.push("Target triple must be specified".into());
        }
        errors
    }
}

// ── FFI ─────────────────────────────────────────────────────────────────

static BUILD_STORES: Mutex<Option<HashMap<i64, BuildCluster>>> = Mutex::new(None);

fn build_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, BuildCluster>>> {
    BUILD_STORES.lock().unwrap()
}

fn next_build_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_distbuild_create(cache_size: i64) -> i64 {
    let id = next_build_id();
    let cluster = BuildCluster::new(cache_size as u64);
    let mut store = build_store();
    store.get_or_insert_with(HashMap::new).insert(id, cluster);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_distbuild_add_node(id: i64, capacity: i64) -> i64 {
    let mut store = build_store();
    if let Some(cluster) = store.as_mut().and_then(|s| s.get_mut(&id)) {
        let node_id = format!("node_{}", cluster.nodes.len());
        cluster.add_node(BuildNode {
            id: node_id, address: "localhost".into(), capacity: capacity as u32,
            running_jobs: 0, completed_jobs: 0, status: NodeStatus::Available,
            platform: "x86_64-unknown-linux-gnu".into(),
        });
        cluster.nodes.len() as i64
    } else { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_distbuild_utilization(id: i64) -> f64 {
    let store = build_store();
    store.as_ref().and_then(|s| s.get(&id))
        .map(|c| c.utilization())
        .unwrap_or(-1.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_distbuild_free(id: i64) {
    let mut store = build_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_publish() {
        let mut client = RegistryClient::new("https://registry.vitalis.dev", "/tmp/cache");
        let pkg = RegistryPackage {
            name: "math_utils".into(),
            versions: vec![PackageVersion {
                version: "1.0.0".into(), checksum: "abc123".into(),
                dependencies: vec![], size_bytes: 1024, yanked: false,
                published_at: 1000, rust_version: None,
            }],
            owner: "dev".into(), description: "Math utilities".into(),
            license: "MIT".into(), repository: "https://github.com/test/math".into(),
            keywords: vec!["math".into()],
        };
        assert!(client.publish(pkg).is_ok());
        assert!(client.lookup("math_utils").is_some());
    }

    #[test]
    fn test_registry_latest_version() {
        let mut client = RegistryClient::new("https://reg.dev", "/cache");
        let pkg = RegistryPackage {
            name: "lib".into(),
            versions: vec![
                PackageVersion { version: "1.0.0".into(), checksum: "a".into(), dependencies: vec![], size_bytes: 100, yanked: false, published_at: 0, rust_version: None },
                PackageVersion { version: "2.0.0".into(), checksum: "b".into(), dependencies: vec![], size_bytes: 200, yanked: true, published_at: 0, rust_version: None },
                PackageVersion { version: "1.5.0".into(), checksum: "c".into(), dependencies: vec![], size_bytes: 150, yanked: false, published_at: 0, rust_version: None },
            ],
            owner: "dev".into(), description: "".into(), license: "MIT".into(), repository: "".into(), keywords: vec![],
        };
        client.publish(pkg).unwrap();
        let latest = client.latest_version("lib").unwrap();
        assert_eq!(latest.version, "1.5.0"); // 2.0.0 is yanked
    }

    #[test]
    fn test_resolve_dependencies() {
        let mut client = RegistryClient::new("https://reg.dev", "/cache");
        client.publish(RegistryPackage {
            name: "dep_a".into(),
            versions: vec![PackageVersion { version: "1.0.0".into(), checksum: "aa".into(), dependencies: vec![], size_bytes: 100, yanked: false, published_at: 0, rust_version: None }],
            owner: "dev".into(), description: "".into(), license: "MIT".into(), repository: "".into(), keywords: vec![],
        }).unwrap();
        client.publish(RegistryPackage {
            name: "my_pkg".into(),
            versions: vec![PackageVersion {
                version: "0.1.0".into(), checksum: "bb".into(), size_bytes: 200, yanked: false, published_at: 0, rust_version: None,
                dependencies: vec![Dependency { name: "dep_a".into(), version_req: "^1.0".into(), optional: false, features: vec![] }],
            }],
            owner: "dev".into(), description: "".into(), license: "MIT".into(), repository: "".into(), keywords: vec![],
        }).unwrap();
        let resolved = client.resolve("my_pkg", "0.1.0").unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "dep_a");
    }

    #[test]
    fn test_vuln_scan() {
        let mut db = VulnDatabase::new();
        db.add_advisory(Vulnerability {
            id: "VTLS-2025-001".into(), package: "crypto_lib".into(),
            affected_versions: vec!["0.9.0".into(), "1.0.0".into()],
            severity: VulnSeverity::Critical,
            description: "Buffer overflow in key derivation".into(),
            patched_version: Some("1.0.1".into()),
        });

        let deps = vec![ResolvedDep { name: "crypto_lib".into(), version: "1.0.0".into(), checksum: "x".into() }];
        let matches = db.scan(&deps);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].vulnerability.severity, VulnSeverity::Critical);
    }

    #[test]
    fn test_lockfile() {
        let mut lock = Lockfile::new();
        lock.add_entry(LockEntry { name: "pkg_a".into(), version: "1.0.0".into(), checksum: "abc".into(), source: "registry".into() });
        lock.add_entry(LockEntry { name: "pkg_b".into(), version: "2.1.0".into(), checksum: "def".into(), source: "registry".into() });
        assert_eq!(lock.entries.len(), 2);
        assert!(lock.find("pkg_a").is_some());
        assert!(!lock.checksum.is_empty());
        let text = lock.to_text();
        assert!(text.contains("pkg_a"));
        assert!(text.contains("2.1.0"));
    }

    #[test]
    fn test_content_cache() {
        let mut cache = ContentCache::new(4096);
        cache.put("key1".into(), "hash1".into(), 1024);
        assert!(cache.get("key1").is_some());
        assert!(cache.get("key2").is_none());
        assert_eq!(cache.hits, 1);
        assert_eq!(cache.misses, 1);
        assert!((cache.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = ContentCache::new(2048);
        cache.put("key1".into(), "a".into(), 1024);
        cache.put("key2".into(), "b".into(), 1024);
        assert_eq!(cache.entries.len(), 2);
        cache.put("key3".into(), "c".into(), 1024); // Should evict key1
        assert_eq!(cache.entries.len(), 2);
    }

    #[test]
    fn test_content_hash() {
        let h1 = ContentCache::content_hash(b"hello world");
        let h2 = ContentCache::content_hash(b"hello world");
        let h3 = ContentCache::content_hash(b"different");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_build_cluster() {
        let mut cluster = BuildCluster::new(1024 * 1024);
        cluster.add_node(BuildNode {
            id: "n1".into(), address: "host1:8080".into(), capacity: 4,
            running_jobs: 0, completed_jobs: 0, status: NodeStatus::Available,
            platform: "x86_64-linux".into(),
        });
        assert_eq!(cluster.utilization(), 0.0);

        cluster.submit_job(BuildJob { id: 1, source_hash: "abc".into(), target: "x86".into(), priority: 10, dependencies: vec![] });
        let result = cluster.schedule_next();
        assert!(result.is_some());
        assert!(cluster.utilization() > 0.0);
    }

    #[test]
    fn test_cluster_complete_job() {
        let mut cluster = BuildCluster::new(1024 * 1024);
        cluster.add_node(BuildNode {
            id: "n1".into(), address: "host1".into(), capacity: 4,
            running_jobs: 0, completed_jobs: 0, status: NodeStatus::Available, platform: "linux".into(),
        });
        cluster.submit_job(BuildJob { id: 1, source_hash: "a".into(), target: "x86".into(), priority: 5, dependencies: vec![] });
        cluster.schedule_next();
        cluster.complete_job(1, "n1", "artifact_hash".into(), 500, true);
        assert_eq!(cluster.completed_jobs.len(), 1);
        assert_eq!(cluster.nodes[0].completed_jobs, 1);
    }

    #[test]
    fn test_hermetic_build() {
        let mut build = HermeticBuild::new("0.40.0", "x86_64-unknown-linux-gnu");
        build.env_vars.insert("OPT_LEVEL".into(), "3".into());
        let fp = build.fingerprint();
        assert!(!fp.is_empty());
        assert!(build.validate().is_empty()); // should be valid
    }

    #[test]
    fn test_hermetic_fingerprint_determinism() {
        let mut b1 = HermeticBuild::new("1.0", "x86_64");
        b1.env_vars.insert("A".into(), "1".into());
        let mut b2 = HermeticBuild::new("1.0", "x86_64");
        b2.env_vars.insert("A".into(), "1".into());
        assert_eq!(b1.fingerprint(), b2.fingerprint());
    }

    #[test]
    fn test_sandbox_modes() {
        let b = HermeticBuild::new("1.0", "aarch64");
        assert_eq!(b.sandbox_mode, SandboxMode::Filesystem);
    }

    #[test]
    fn test_ffi_distbuild() {
        let id = vitalis_distbuild_create(1024 * 1024);
        assert!(id > 0);
        let n = vitalis_distbuild_add_node(id, 8);
        assert_eq!(n, 1);
        let util = vitalis_distbuild_utilization(id);
        assert!((util - 0.0).abs() < 0.01);
        vitalis_distbuild_free(id);
    }
}
