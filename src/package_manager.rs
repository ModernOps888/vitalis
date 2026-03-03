//! Vitalis Package Manager — dependency resolution, registry, and package management.
//!
//! Provides:
//! - Package manifest parsing (vitalis.toml)
//! - Dependency resolution with semantic versioning
//! - Package registry client for publishing/downloading
//! - Local cache management
//! - Lockfile generation for reproducible builds
//!
//! A Vitalis package (crate) has this structure:
//! ```text
//! my_package/
//!   vitalis.toml        # Package manifest
//!   vitalis.lock        # Lockfile (generated)
//!   src/
//!     main.sl           # Entry point
//!     lib.sl            # Library root
//!   tests/
//!     test_main.sl
//! ```

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

// ─── Semantic Version ───────────────────────────────────────────────────

/// Semantic version: major.minor.patch
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
}

impl SemVer {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch, pre: None }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('v');
        let parts: Vec<&str> = s.splitn(2, '-').collect();
        let version_part = parts[0];
        let pre = parts.get(1).map(|p| p.to_string());

        let nums: Vec<u32> = version_part.split('.')
            .filter_map(|n| n.parse().ok())
            .collect();

        match nums.len() {
            1 => Some(Self { major: nums[0], minor: 0, patch: 0, pre }),
            2 => Some(Self { major: nums[0], minor: nums[1], patch: 0, pre }),
            3 => Some(Self { major: nums[0], minor: nums[1], patch: nums[2], pre }),
            _ => None,
        }
    }

    /// Check if this version is compatible with a requirement.
    /// Uses caret (^) semantics by default: ^1.2.3 means >=1.2.3, <2.0.0
    pub fn compatible_with(&self, req: &VersionReq) -> bool {
        match req {
            VersionReq::Exact(v) => self == v,
            VersionReq::Caret(v) => {
                if v.major > 0 {
                    self.major == v.major && self >= v
                } else if v.minor > 0 {
                    self.major == 0 && self.minor == v.minor && self >= v
                } else {
                    self == v
                }
            }
            VersionReq::Tilde(v) => {
                self.major == v.major && self.minor == v.minor && self.patch >= v.patch
            }
            VersionReq::Range { min, max } => self >= min && self < max,
            VersionReq::Any => true,
        }
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

/// Version requirement specifier.
#[derive(Debug, Clone, PartialEq)]
pub enum VersionReq {
    /// Exact: "=1.2.3"
    Exact(SemVer),
    /// Caret (default): "^1.2.3" or "1.2.3"
    Caret(SemVer),
    /// Tilde: "~1.2.3"
    Tilde(SemVer),
    /// Range: ">=1.0.0, <2.0.0"
    Range { min: SemVer, max: SemVer },
    /// Any version: "*"
    Any,
}

impl VersionReq {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "*" {
            return Some(VersionReq::Any);
        }
        if let Some(v) = s.strip_prefix('=') {
            return SemVer::parse(v).map(VersionReq::Exact);
        }
        if let Some(v) = s.strip_prefix('~') {
            return SemVer::parse(v).map(VersionReq::Tilde);
        }
        if let Some(v) = s.strip_prefix('^') {
            return SemVer::parse(v).map(VersionReq::Caret);
        }
        // Default: caret semantics
        SemVer::parse(s).map(VersionReq::Caret)
    }
}

// ─── Package Manifest ───────────────────────────────────────────────────

/// A package manifest (vitalis.toml).
#[derive(Debug, Clone)]
pub struct PackageManifest {
    pub name: String,
    pub version: SemVer,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
    pub entry_point: Option<String>,
}

impl PackageManifest {
    pub fn new(name: &str, version: SemVer) -> Self {
        Self {
            name: name.to_string(),
            version,
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            entry_point: Some("src/main.sl".to_string()),
        }
    }

    /// Add a dependency.
    pub fn add_dependency(&mut self, dep: Dependency) {
        self.dependencies.push(dep);
    }

    /// Generate a TOML-like string representation.
    pub fn to_toml(&self) -> String {
        let mut s = String::new();
        s.push_str("[package]\n");
        s.push_str(&format!("name = \"{}\"\n", self.name));
        s.push_str(&format!("version = \"{}\"\n", self.version));
        if let Some(ref desc) = self.description {
            s.push_str(&format!("description = \"{}\"\n", desc));
        }
        if !self.authors.is_empty() {
            let authors: Vec<String> = self.authors.iter()
                .map(|a| format!("\"{}\"", a))
                .collect();
            s.push_str(&format!("authors = [{}]\n", authors.join(", ")));
        }
        if let Some(ref lic) = self.license {
            s.push_str(&format!("license = \"{}\"\n", lic));
        }
        s.push('\n');

        if !self.dependencies.is_empty() {
            s.push_str("[dependencies]\n");
            for dep in &self.dependencies {
                s.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version_req));
            }
            s.push('\n');
        }

        s
    }
}

/// A dependency declaration.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub registry: Option<String>,
    pub git: Option<String>,
    pub path: Option<PathBuf>,
    pub optional: bool,
}

impl Dependency {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version_req: version.to_string(),
            registry: None,
            git: None,
            path: None,
            optional: false,
        }
    }

    pub fn from_git(name: &str, url: &str) -> Self {
        Self {
            name: name.to_string(),
            version_req: "*".to_string(),
            registry: None,
            git: Some(url.to_string()),
            path: None,
            optional: false,
        }
    }

    pub fn from_path(name: &str, path: &Path) -> Self {
        Self {
            name: name.to_string(),
            version_req: "*".to_string(),
            registry: None,
            git: None,
            path: Some(path.to_path_buf()),
            optional: false,
        }
    }
}

// ─── Lockfile ───────────────────────────────────────────────────────────

/// Lockfile entry — pinned version of a resolved dependency.
#[derive(Debug, Clone)]
pub struct LockEntry {
    pub name: String,
    pub version: SemVer,
    pub checksum: Option<String>,
    pub source: String,
}

/// The lockfile (vitalis.lock).
#[derive(Debug, Clone, Default)]
pub struct Lockfile {
    pub entries: Vec<LockEntry>,
}

impl Lockfile {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add(&mut self, entry: LockEntry) {
        self.entries.push(entry);
    }

    pub fn find(&self, name: &str) -> Option<&LockEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    pub fn to_string_repr(&self) -> String {
        let mut s = String::from("# vitalis.lock — auto-generated, do not edit\n\n");
        for entry in &self.entries {
            s.push_str(&format!("[[package]]\nname = \"{}\"\nversion = \"{}\"\n",
                entry.name, entry.version));
            if let Some(ref cs) = entry.checksum {
                s.push_str(&format!("checksum = \"{}\"\n", cs));
            }
            s.push_str(&format!("source = \"{}\"\n\n", entry.source));
        }
        s
    }
}

// ─── Package Registry ───────────────────────────────────────────────────

/// An entry in the package registry.
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub versions: Vec<SemVer>,
    pub description: String,
    pub downloads: u64,
}

/// The package registry (local simulation).
#[derive(Debug, Default)]
pub struct Registry {
    packages: HashMap<String, RegistryEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish a package version to the registry.
    pub fn publish(&mut self, name: &str, version: SemVer, description: &str) -> bool {
        let entry = self.packages.entry(name.to_string())
            .or_insert_with(|| RegistryEntry {
                name: name.to_string(),
                versions: Vec::new(),
                description: description.to_string(),
                downloads: 0,
            });

        if entry.versions.contains(&version) {
            return false; // Already published
        }

        entry.versions.push(version);
        entry.versions.sort();
        true
    }

    /// Search for packages matching a query.
    pub fn search(&self, query: &str) -> Vec<&RegistryEntry> {
        self.packages.values()
            .filter(|e| e.name.contains(query) || e.description.contains(query))
            .collect()
    }

    /// Get the latest version that satisfies a requirement.
    pub fn resolve(&self, name: &str, req: &VersionReq) -> Option<SemVer> {
        let entry = self.packages.get(name)?;
        entry.versions.iter()
            .rev()
            .find(|v| v.compatible_with(req))
            .cloned()
    }

    /// Download (increment counter) a package.
    pub fn download(&mut self, name: &str) -> bool {
        if let Some(entry) = self.packages.get_mut(name) {
            entry.downloads += 1;
            true
        } else {
            false
        }
    }

    pub fn package_count(&self) -> usize {
        self.packages.len()
    }
}

// ─── Dependency Resolver ────────────────────────────────────────────────

/// Resolve all dependencies from a manifest against a registry.
pub fn resolve_dependencies(
    manifest: &PackageManifest,
    registry: &Registry,
) -> Result<Lockfile, String> {
    let mut lockfile = Lockfile::new();

    for dep in &manifest.dependencies {
        let req = VersionReq::parse(&dep.version_req)
            .ok_or_else(|| format!("Invalid version requirement: {}", dep.version_req))?;

        if let Some(ref _path) = dep.path {
            // Local path dependency — resolve without registry
            lockfile.add(LockEntry {
                name: dep.name.clone(),
                version: SemVer::new(0, 0, 0),
                checksum: None,
                source: format!("path:{}", _path.display()),
            });
        } else if let Some(ref git) = dep.git {
            // Git dependency
            lockfile.add(LockEntry {
                name: dep.name.clone(),
                version: SemVer::new(0, 0, 0),
                checksum: None,
                source: format!("git:{}", git),
            });
        } else {
            // Registry dependency
            let resolved = registry.resolve(&dep.name, &req)
                .ok_or_else(|| format!("No version of '{}' satisfies {}", dep.name, dep.version_req))?;

            lockfile.add(LockEntry {
                name: dep.name.clone(),
                version: resolved,
                checksum: None,
                source: "registry:vitalis.io".to_string(),
            });
        }
    }

    Ok(lockfile)
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parse() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v, SemVer::new(1, 2, 3));
    }

    #[test]
    fn test_semver_parse_short() {
        let v = SemVer::parse("1.2").unwrap();
        assert_eq!(v, SemVer::new(1, 2, 0));
        let v2 = SemVer::parse("3").unwrap();
        assert_eq!(v2, SemVer::new(3, 0, 0));
    }

    #[test]
    fn test_semver_parse_pre() {
        let v = SemVer::parse("1.0.0-beta").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.pre, Some("beta".to_string()));
    }

    #[test]
    fn test_semver_display() {
        assert_eq!(format!("{}", SemVer::new(1, 2, 3)), "1.2.3");
    }

    #[test]
    fn test_semver_ord() {
        let v1 = SemVer::new(1, 0, 0);
        let v2 = SemVer::new(1, 1, 0);
        let v3 = SemVer::new(2, 0, 0);
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_version_req_caret() {
        let req = VersionReq::parse("^1.2.3").unwrap();
        let v_ok = SemVer::new(1, 3, 0);
        let v_bad = SemVer::new(2, 0, 0);
        assert!(v_ok.compatible_with(&req));
        assert!(!v_bad.compatible_with(&req));
    }

    #[test]
    fn test_version_req_tilde() {
        let req = VersionReq::parse("~1.2.3").unwrap();
        let v_ok = SemVer::new(1, 2, 5);
        let v_bad = SemVer::new(1, 3, 0);
        assert!(v_ok.compatible_with(&req));
        assert!(!v_bad.compatible_with(&req));
    }

    #[test]
    fn test_version_req_exact() {
        let req = VersionReq::parse("=1.2.3").unwrap();
        assert!(SemVer::new(1, 2, 3).compatible_with(&req));
        assert!(!SemVer::new(1, 2, 4).compatible_with(&req));
    }

    #[test]
    fn test_version_req_any() {
        let req = VersionReq::parse("*").unwrap();
        assert!(SemVer::new(99, 99, 99).compatible_with(&req));
    }

    #[test]
    fn test_manifest_creation() {
        let mut m = PackageManifest::new("hello", SemVer::new(1, 0, 0));
        m.description = Some("A hello world package".into());
        m.add_dependency(Dependency::new("math_lib", "^1.0"));
        assert_eq!(m.name, "hello");
        assert_eq!(m.dependencies.len(), 1);
    }

    #[test]
    fn test_manifest_toml() {
        let m = PackageManifest::new("test_pkg", SemVer::new(0, 1, 0));
        let toml = m.to_toml();
        assert!(toml.contains("name = \"test_pkg\""));
        assert!(toml.contains("version = \"0.1.0\""));
    }

    #[test]
    fn test_dependency_from_git() {
        let dep = Dependency::from_git("cool_lib", "https://github.com/user/repo");
        assert!(dep.git.is_some());
        assert_eq!(dep.version_req, "*");
    }

    #[test]
    fn test_dependency_from_path() {
        let dep = Dependency::from_path("local_lib", Path::new("/home/user/local_lib"));
        assert!(dep.path.is_some());
    }

    #[test]
    fn test_lockfile() {
        let mut lock = Lockfile::new();
        lock.add(LockEntry {
            name: "math".into(),
            version: SemVer::new(1, 2, 0),
            checksum: Some("abc123".into()),
            source: "registry:vitalis.io".into(),
        });
        assert!(lock.find("math").is_some());
        assert!(lock.find("unknown").is_none());
    }

    #[test]
    fn test_lockfile_repr() {
        let mut lock = Lockfile::new();
        lock.add(LockEntry {
            name: "test".into(),
            version: SemVer::new(1, 0, 0),
            checksum: None,
            source: "registry:vitalis.io".into(),
        });
        let s = lock.to_string_repr();
        assert!(s.contains("name = \"test\""));
        assert!(s.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_registry_publish() {
        let mut reg = Registry::new();
        assert!(reg.publish("math", SemVer::new(1, 0, 0), "Math library"));
        assert!(reg.publish("math", SemVer::new(1, 1, 0), "Math library"));
        assert!(!reg.publish("math", SemVer::new(1, 0, 0), "Math library")); // Duplicate
        assert_eq!(reg.package_count(), 1);
    }

    #[test]
    fn test_registry_resolve() {
        let mut reg = Registry::new();
        reg.publish("math", SemVer::new(1, 0, 0), "Math");
        reg.publish("math", SemVer::new(1, 1, 0), "Math");
        reg.publish("math", SemVer::new(2, 0, 0), "Math");

        let req = VersionReq::Caret(SemVer::new(1, 0, 0));
        let resolved = reg.resolve("math", &req);
        assert_eq!(resolved, Some(SemVer::new(1, 1, 0))); // Latest ^1.x
    }

    #[test]
    fn test_registry_search() {
        let mut reg = Registry::new();
        reg.publish("math_core", SemVer::new(1, 0, 0), "Core math");
        reg.publish("string_utils", SemVer::new(1, 0, 0), "String utilities");

        let results = reg.search("math");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "math_core");
    }

    #[test]
    fn test_registry_download() {
        let mut reg = Registry::new();
        reg.publish("lib", SemVer::new(1, 0, 0), "A lib");
        assert!(reg.download("lib"));
        assert!(!reg.download("nonexistent"));
    }

    #[test]
    fn test_resolve_dependencies() {
        let mut reg = Registry::new();
        reg.publish("math", SemVer::new(1, 0, 0), "Math");
        reg.publish("math", SemVer::new(1, 2, 0), "Math");
        reg.publish("io", SemVer::new(0, 5, 0), "IO");

        let mut manifest = PackageManifest::new("my_app", SemVer::new(0, 1, 0));
        manifest.add_dependency(Dependency::new("math", "^1.0"));
        manifest.add_dependency(Dependency::new("io", "^0.5"));

        let lockfile = resolve_dependencies(&manifest, &reg).unwrap();
        assert_eq!(lockfile.entries.len(), 2);
        assert_eq!(lockfile.find("math").unwrap().version, SemVer::new(1, 2, 0));
    }

    #[test]
    fn test_resolve_missing_dep() {
        let reg = Registry::new();
        let mut manifest = PackageManifest::new("app", SemVer::new(0, 1, 0));
        manifest.add_dependency(Dependency::new("nonexistent", "1.0"));

        let result = resolve_dependencies(&manifest, &reg);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_path_dependency() {
        let reg = Registry::new();
        let mut manifest = PackageManifest::new("app", SemVer::new(0, 1, 0));
        manifest.add_dependency(Dependency::from_path("local", Path::new("../local_lib")));

        let lockfile = resolve_dependencies(&manifest, &reg).unwrap();
        assert_eq!(lockfile.entries.len(), 1);
        assert!(lockfile.entries[0].source.starts_with("path:"));
    }
}
