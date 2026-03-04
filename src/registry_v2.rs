//! Package registry v2 for Vitalis.
//!
//! Advanced package ecosystem infrastructure:
//! - **Publishing**: Build, validate, sign, and upload packages
//! - **SemVer enforcement**: API diff detection for breaking changes
//! - **Security advisories**: CVE tracking and dependency auditing
//! - **License compliance**: SPDX-based license checking
//! - **Yanking**: Remove broken versions safely
//! - **Namespace governance**: Scoped packages with ownership

use std::collections::{HashMap, HashSet, BTreeMap};

// ── Package Identity ────────────────────────────────────────────────

/// Scoped package name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageName {
    pub scope: Option<String>,
    pub name: String,
}

impl PackageName {
    pub fn new(name: &str) -> Self {
        Self { scope: None, name: name.to_string() }
    }

    pub fn scoped(scope: &str, name: &str) -> Self {
        Self { scope: Some(scope.to_string()), name: name.to_string() }
    }

    pub fn full_name(&self) -> String {
        match &self.scope {
            Some(s) => format!("@{}/{}", s, self.name),
            None => self.name.clone(),
        }
    }
}

/// A semantic version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

    pub fn with_pre(mut self, pre: &str) -> Self {
        self.pre = Some(pre.to_string());
        self
    }

    pub fn parse(s: &str) -> Option<Self> {
        let (version_part, pre) = if let Some(idx) = s.find('-') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 { return None; }
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
            pre,
        })
    }

    pub fn to_string(&self) -> String {
        match &self.pre {
            Some(pre) => format!("{}.{}.{}-{}", self.major, self.minor, self.patch, pre),
            None => format!("{}.{}.{}", self.major, self.minor, self.patch),
        }
    }

    /// Check if this version satisfies a version requirement.
    pub fn satisfies(&self, req: &VersionReq) -> bool {
        match req {
            VersionReq::Exact(v) => self == v,
            VersionReq::Compatible(v) => {
                self.major == v.major && (self.minor > v.minor || (self.minor == v.minor && self.patch >= v.patch))
            }
            VersionReq::Range { min, max } => self >= min && self <= max,
            VersionReq::Any => true,
        }
    }
}

/// Version requirement.
#[derive(Debug, Clone, PartialEq)]
pub enum VersionReq {
    Exact(SemVer),
    Compatible(SemVer),
    Range { min: SemVer, max: SemVer },
    Any,
}

// ── API Surface ─────────────────────────────────────────────────────

/// A public API item (for breaking change detection).
#[derive(Debug, Clone, PartialEq)]
pub enum ApiItem {
    Function { name: String, params: Vec<String>, return_type: String },
    Struct { name: String, fields: Vec<(String, String)> },
    Enum { name: String, variants: Vec<String> },
    Trait { name: String, methods: Vec<String> },
    Constant { name: String, typ: String },
    TypeAlias { name: String, definition: String },
}

/// API change kind.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiChange {
    Added(ApiItem),
    Removed(ApiItem),
    Modified { old: ApiItem, new: ApiItem },
}

impl ApiChange {
    pub fn is_breaking(&self) -> bool {
        match self {
            ApiChange::Removed(_) => true,
            ApiChange::Modified { .. } => true,
            ApiChange::Added(_) => false,
        }
    }
}

/// Diff two API surfaces to find changes.
pub fn diff_api(old: &[ApiItem], new: &[ApiItem]) -> Vec<ApiChange> {
    let mut changes = Vec::new();
    let old_names: HashMap<String, &ApiItem> = old.iter().map(|i| (api_item_name(i), i)).collect();
    let new_names: HashMap<String, &ApiItem> = new.iter().map(|i| (api_item_name(i), i)).collect();

    for (name, item) in &old_names {
        if let Some(new_item) = new_names.get(name) {
            if item != new_item {
                changes.push(ApiChange::Modified {
                    old: (*item).clone(),
                    new: (*new_item).clone(),
                });
            }
        } else {
            changes.push(ApiChange::Removed((*item).clone()));
        }
    }

    for (name, item) in &new_names {
        if !old_names.contains_key(name) {
            changes.push(ApiChange::Added((*item).clone()));
        }
    }

    changes
}

fn api_item_name(item: &ApiItem) -> String {
    match item {
        ApiItem::Function { name, .. } | ApiItem::Struct { name, .. } |
        ApiItem::Enum { name, .. } | ApiItem::Trait { name, .. } |
        ApiItem::Constant { name, .. } | ApiItem::TypeAlias { name, .. } => name.clone(),
    }
}

// ── Security Advisory ───────────────────────────────────────────────

/// A security advisory.
#[derive(Debug, Clone)]
pub struct Advisory {
    pub id: String,
    pub package: PackageName,
    pub affected_versions: Vec<VersionReq>,
    pub severity: AdvisorySeverity,
    pub title: String,
    pub description: String,
    pub patched_version: Option<SemVer>,
}

/// Advisory severity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdvisorySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Dependency audit result.
#[derive(Debug, Clone)]
pub struct AuditResult {
    pub package: PackageName,
    pub version: SemVer,
    pub advisories: Vec<Advisory>,
    pub license: Option<String>,
    pub license_compatible: bool,
}

// ── Package Registry ────────────────────────────────────────────────

/// A published package version.
#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub name: PackageName,
    pub version: SemVer,
    pub dependencies: Vec<(PackageName, VersionReq)>,
    pub checksum: String,
    pub license: String,
    pub yanked: bool,
    pub api_surface: Vec<ApiItem>,
    pub published_at: u64,
}

/// The package registry.
pub struct Registry {
    packages: HashMap<String, Vec<PackageVersion>>,
    advisories: Vec<Advisory>,
    allowed_licenses: HashSet<String>,
    owners: HashMap<String, Vec<String>>,
}

impl Registry {
    pub fn new() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert("MIT".to_string());
        allowed.insert("Apache-2.0".to_string());
        allowed.insert("BSD-3-Clause".to_string());
        allowed.insert("ISC".to_string());
        Self {
            packages: HashMap::new(),
            advisories: Vec::new(),
            allowed_licenses: allowed,
            owners: HashMap::new(),
        }
    }

    /// Publish a package version.
    pub fn publish(&mut self, pkg: PackageVersion) -> Result<(), String> {
        let full = pkg.name.full_name();

        // Check license.
        if !self.allowed_licenses.contains(&pkg.license) {
            return Err(format!("license '{}' not in allowed list", pkg.license));
        }

        // Check for duplicate version.
        if let Some(versions) = self.packages.get(&full) {
            if versions.iter().any(|v| v.version == pkg.version) {
                return Err(format!("version {} already published", pkg.version.to_string()));
            }

            // Check for breaking changes without major bump.
            if let Some(latest) = versions.last() {
                if pkg.version.major == latest.version.major {
                    let changes = diff_api(&latest.api_surface, &pkg.api_surface);
                    let breaking: Vec<_> = changes.iter().filter(|c| c.is_breaking()).collect();
                    if !breaking.is_empty() {
                        return Err(format!("{} breaking changes detected — bump major version", breaking.len()));
                    }
                }
            }
        }

        self.packages.entry(full).or_default().push(pkg);
        Ok(())
    }

    /// Yank a version.
    pub fn yank(&mut self, name: &PackageName, version: &SemVer) -> bool {
        let full = name.full_name();
        if let Some(versions) = self.packages.get_mut(&full) {
            if let Some(v) = versions.iter_mut().find(|v| v.version == *version) {
                v.yanked = true;
                return true;
            }
        }
        false
    }

    /// Resolve a dependency to a concrete version.
    pub fn resolve(&self, name: &PackageName, req: &VersionReq) -> Option<&PackageVersion> {
        let full = name.full_name();
        self.packages.get(&full)?
            .iter()
            .filter(|v| !v.yanked && v.version.satisfies(req))
            .max_by(|a, b| a.version.cmp(&b.version))
    }

    /// Audit a package for security issues.
    pub fn audit(&self, name: &PackageName, version: &SemVer) -> AuditResult {
        let matching: Vec<_> = self.advisories.iter()
            .filter(|a| a.package == *name)
            .filter(|a| a.affected_versions.iter().any(|r| version.satisfies(r)))
            .cloned()
            .collect();

        let full = name.full_name();
        let license = self.packages.get(&full)
            .and_then(|vs| vs.iter().find(|v| v.version == *version))
            .map(|v| v.license.clone());
        let compat = license.as_ref().map(|l| self.allowed_licenses.contains(l)).unwrap_or(false);

        AuditResult {
            package: name.clone(),
            version: version.clone(),
            advisories: matching,
            license,
            license_compatible: compat,
        }
    }

    /// Add a security advisory.
    pub fn add_advisory(&mut self, advisory: Advisory) {
        self.advisories.push(advisory);
    }

    /// Total packages.
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Total versions across all packages.
    pub fn version_count(&self) -> usize {
        self.packages.values().map(|vs| vs.len()).sum()
    }

    /// Add an allowed license.
    pub fn allow_license(&mut self, license: &str) {
        self.allowed_licenses.insert(license.to_string());
    }

    /// List all versions of a package.
    pub fn versions(&self, name: &PackageName) -> Vec<&SemVer> {
        self.packages.get(&name.full_name())
            .map(|vs| vs.iter().map(|v| &v.version).collect())
            .unwrap_or_default()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pkg(name: &str, major: u32, minor: u32, patch: u32) -> PackageVersion {
        PackageVersion {
            name: PackageName::new(name),
            version: SemVer::new(major, minor, patch),
            dependencies: vec![],
            checksum: "abc123".into(),
            license: "MIT".into(),
            yanked: false,
            api_surface: vec![],
            published_at: 0,
        }
    }

    #[test]
    fn test_package_name() {
        let p = PackageName::new("hello");
        assert_eq!(p.full_name(), "hello");
        let s = PackageName::scoped("org", "hello");
        assert_eq!(s.full_name(), "@org/hello");
    }

    #[test]
    fn test_semver_parse() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v, SemVer::new(1, 2, 3));
        let v2 = SemVer::parse("1.0.0-beta").unwrap();
        assert_eq!(v2.pre, Some("beta".into()));
    }

    #[test]
    fn test_semver_ordering() {
        assert!(SemVer::new(1, 0, 0) < SemVer::new(2, 0, 0));
        assert!(SemVer::new(1, 1, 0) < SemVer::new(1, 2, 0));
        assert!(SemVer::new(1, 1, 1) < SemVer::new(1, 1, 2));
    }

    #[test]
    fn test_version_satisfies_exact() {
        let v = SemVer::new(1, 2, 3);
        assert!(v.satisfies(&VersionReq::Exact(SemVer::new(1, 2, 3))));
        assert!(!v.satisfies(&VersionReq::Exact(SemVer::new(1, 2, 4))));
    }

    #[test]
    fn test_version_satisfies_compatible() {
        let v = SemVer::new(1, 3, 0);
        assert!(v.satisfies(&VersionReq::Compatible(SemVer::new(1, 2, 0))));
        assert!(!v.satisfies(&VersionReq::Compatible(SemVer::new(2, 0, 0))));
    }

    #[test]
    fn test_publish_and_resolve() {
        let mut reg = Registry::new();
        let pkg = make_pkg("hello", 1, 0, 0);
        reg.publish(pkg).unwrap();
        let resolved = reg.resolve(&PackageName::new("hello"), &VersionReq::Any).unwrap();
        assert_eq!(resolved.version, SemVer::new(1, 0, 0));
    }

    #[test]
    fn test_duplicate_version_rejected() {
        let mut reg = Registry::new();
        reg.publish(make_pkg("hello", 1, 0, 0)).unwrap();
        let result = reg.publish(make_pkg("hello", 1, 0, 0));
        assert!(result.is_err());
    }

    #[test]
    fn test_yank() {
        let mut reg = Registry::new();
        reg.publish(make_pkg("hello", 1, 0, 0)).unwrap();
        let name = PackageName::new("hello");
        assert!(reg.yank(&name, &SemVer::new(1, 0, 0)));
        assert!(reg.resolve(&name, &VersionReq::Any).is_none());
    }

    #[test]
    fn test_invalid_license() {
        let mut reg = Registry::new();
        let mut pkg = make_pkg("hello", 1, 0, 0);
        pkg.license = "AGPL-3.0".into();
        let result = reg.publish(pkg);
        assert!(result.is_err());
    }

    #[test]
    fn test_api_diff_no_changes() {
        let api = vec![ApiItem::Function { name: "foo".into(), params: vec![], return_type: "i32".into() }];
        let changes = diff_api(&api, &api);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_api_diff_breaking() {
        let old = vec![ApiItem::Function { name: "foo".into(), params: vec![], return_type: "i32".into() }];
        let new = vec![];
        let changes = diff_api(&old, &new);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_breaking());
    }

    #[test]
    fn test_api_diff_added() {
        let old = vec![];
        let new = vec![ApiItem::Function { name: "bar".into(), params: vec![], return_type: "void".into() }];
        let changes = diff_api(&old, &new);
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].is_breaking());
    }

    #[test]
    fn test_audit_clean() {
        let reg = Registry::new();
        let result = reg.audit(&PackageName::new("hello"), &SemVer::new(1, 0, 0));
        assert!(result.advisories.is_empty());
    }

    #[test]
    fn test_audit_with_advisory() {
        let mut reg = Registry::new();
        reg.publish(make_pkg("vuln", 1, 0, 0)).unwrap();
        reg.add_advisory(Advisory {
            id: "CVE-2025-001".into(),
            package: PackageName::new("vuln"),
            affected_versions: vec![VersionReq::Any],
            severity: AdvisorySeverity::High,
            title: "RCE".into(),
            description: "Remote code execution".into(),
            patched_version: Some(SemVer::new(1, 0, 1)),
        });
        let result = reg.audit(&PackageName::new("vuln"), &SemVer::new(1, 0, 0));
        assert_eq!(result.advisories.len(), 1);
    }

    #[test]
    fn test_registry_counts() {
        let mut reg = Registry::new();
        reg.publish(make_pkg("a", 1, 0, 0)).unwrap();
        reg.publish(make_pkg("a", 1, 1, 0)).unwrap();
        reg.publish(make_pkg("b", 1, 0, 0)).unwrap();
        assert_eq!(reg.package_count(), 2);
        assert_eq!(reg.version_count(), 3);
    }

    #[test]
    fn test_versions_list() {
        let mut reg = Registry::new();
        reg.publish(make_pkg("x", 1, 0, 0)).unwrap();
        reg.publish(make_pkg("x", 1, 1, 0)).unwrap();
        let vs = reg.versions(&PackageName::new("x"));
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn test_advisory_severity_ordering() {
        assert!(AdvisorySeverity::Low < AdvisorySeverity::Medium);
        assert!(AdvisorySeverity::Medium < AdvisorySeverity::High);
        assert!(AdvisorySeverity::High < AdvisorySeverity::Critical);
    }

    #[test]
    fn test_semver_to_string() {
        let v = SemVer::new(2, 1, 0);
        assert_eq!(v.to_string(), "2.1.0");
        let v2 = SemVer::new(1, 0, 0).with_pre("rc.1");
        assert_eq!(v2.to_string(), "1.0.0-rc.1");
    }
}
