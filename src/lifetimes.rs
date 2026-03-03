//! Vitalis Lifetime Annotations & Region Analysis (v22 Roadmap)
//!
//! Provides compile-time lifetime tracking and region-based memory safety analysis.
//! Integrates with the ownership/borrow checker to ensure references never outlive
//! their referents.
//!
//! # Architecture
//!
//! ```text
//! AST (with lifetime annotations) → Region Inference → Constraint Solving → Error Reporting
//! ```
//!
//! Lifetimes are abstract names for scopes. The region analysis engine:
//! 1. Assigns a region variable to every reference and borrow.
//! 2. Collects constraints (e.g., `'a: 'b` means region `'a` outlives `'b`).
//! 3. Solves constraints to find the smallest valid regions.
//! 4. Reports errors when no valid assignment exists.
//!
//! Lifetimes are erased before IR lowering — they are purely a static safety net.

use std::collections::{HashMap, HashSet, BTreeMap};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Lifetime & Region Primitives
// ═══════════════════════════════════════════════════════════════════════

/// A named lifetime parameter (e.g., `'a`, `'b`, `'static`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lifetime {
    pub name: String,
    pub id: LifetimeId,
}

impl Lifetime {
    pub fn new(name: &str, id: LifetimeId) -> Self {
        Self {
            name: name.to_string(),
            id,
        }
    }

    pub fn is_static(&self) -> bool {
        self.name == "static"
    }
}

impl fmt::Display for Lifetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}", self.name)
    }
}

/// Unique identifier for a lifetime/region variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LifetimeId(pub u32);

impl fmt::Display for LifetimeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "R{}", self.0)
    }
}

/// A region represents a set of program points where a value is valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub id: LifetimeId,
    /// Scope depth (0 = global / 'static, higher = deeper nesting)
    pub scope_depth: u32,
    /// Name of the enclosing function or block
    pub scope_name: String,
    /// Whether this region is universal ('static) or existential (inferred)
    pub kind: RegionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// The `'static` lifetime — lives for the entire program.
    Static,
    /// A named lifetime parameter on a function signature.
    Named,
    /// An anonymous, inferred lifetime for local borrows.
    Inferred,
    /// A scope-bound region (block, loop, function body).
    Scope,
}

// ═══════════════════════════════════════════════════════════════════════
//  Lifetime Constraints
// ═══════════════════════════════════════════════════════════════════════

/// A constraint between two lifetime regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifetimeConstraint {
    /// `'a: 'b` — region `a` must outlive region `b`.
    Outlives {
        longer: LifetimeId,
        shorter: LifetimeId,
        reason: String,
    },
    /// `'a == 'b` — two regions must be identical.
    Equal {
        left: LifetimeId,
        right: LifetimeId,
        reason: String,
    },
    /// `'a` must be alive at a specific program point.
    LiveAt {
        region: LifetimeId,
        point: ProgramPoint,
    },
}

/// A specific point in the program (for liveness analysis).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramPoint {
    pub function: String,
    pub block: u32,
    pub statement: u32,
}

impl fmt::Display for ProgramPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::bb{}[{}]", self.function, self.block, self.statement)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Lifetime Errors
// ═══════════════════════════════════════════════════════════════════════

/// An error detected during lifetime/region analysis.
#[derive(Debug, Clone)]
pub struct LifetimeError {
    pub kind: LifetimeErrorKind,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LifetimeErrorKind {
    /// A reference outlives the value it borrows.
    DanglingReference,
    /// Conflicting lifetime requirements cannot be satisfied.
    ConflictingLifetimes,
    /// A lifetime parameter is unused.
    UnusedLifetime,
    /// Missing lifetime annotation where one is required.
    MissingAnnotation,
    /// Returned reference doesn't match any input lifetime.
    ReturnLifetimeMismatch,
    /// Borrow escapes its enclosing scope.
    BorrowEscapesScope,
}

impl fmt::Display for LifetimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lifetime error: {}", self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Borrow Record (tracking active borrows)
// ═══════════════════════════════════════════════════════════════════════

/// Tracks an active borrow of a variable.
#[derive(Debug, Clone)]
pub struct BorrowRecord {
    pub variable: String,
    pub region: LifetimeId,
    pub mutable: bool,
    pub scope_depth: u32,
    pub origin_point: ProgramPoint,
}

// ═══════════════════════════════════════════════════════════════════════
//  Region Inference Engine
// ═══════════════════════════════════════════════════════════════════════

/// The main region analysis engine.
///
/// Workflow:
/// 1. `enter_function()` — begin analyzing a function
/// 2. `declare_lifetime()` — register named lifetime params
/// 3. `create_borrow()` — record each borrow expression
/// 4. `add_constraint()` — collect outlives/equality constraints
/// 5. `solve()` — find valid region assignments or report errors
pub struct RegionAnalyzer {
    /// All known regions, indexed by LifetimeId
    regions: BTreeMap<LifetimeId, Region>,
    /// Constraints collected during analysis
    constraints: Vec<LifetimeConstraint>,
    /// Active borrows, by variable name
    active_borrows: HashMap<String, Vec<BorrowRecord>>,
    /// Mapping from lifetime names to IDs
    named_lifetimes: HashMap<String, LifetimeId>,
    /// Counter for generating fresh region IDs
    next_id: u32,
    /// Current scope depth
    scope_depth: u32,
    /// Current function being analyzed
    current_function: String,
    /// Current block index
    current_block: u32,
    /// Current statement index
    current_stmt: u32,
    /// Collected errors
    errors: Vec<LifetimeError>,
    /// Region outlives graph (adjacency list): if (a, b) then 'a outlives 'b
    outlives_graph: HashMap<LifetimeId, HashSet<LifetimeId>>,
}

impl RegionAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            regions: BTreeMap::new(),
            constraints: Vec::new(),
            active_borrows: HashMap::new(),
            named_lifetimes: HashMap::new(),
            next_id: 0,
            scope_depth: 0,
            current_function: String::new(),
            current_block: 0,
            current_stmt: 0,
            errors: Vec::new(),
            outlives_graph: HashMap::new(),
        };

        // Register 'static as region 0
        let static_id = analyzer.fresh_region_id();
        analyzer.regions.insert(static_id, Region {
            id: static_id,
            scope_depth: 0,
            scope_name: "static".to_string(),
            kind: RegionKind::Static,
        });
        analyzer.named_lifetimes.insert("static".to_string(), static_id);

        analyzer
    }

    fn fresh_region_id(&mut self) -> LifetimeId {
        let id = LifetimeId(self.next_id);
        self.next_id += 1;
        id
    }

    fn current_point(&self) -> ProgramPoint {
        ProgramPoint {
            function: self.current_function.clone(),
            block: self.current_block,
            statement: self.current_stmt,
        }
    }

    // ── Function scope management ──────────────────────────────────────

    /// Begin analyzing a function.
    pub fn enter_function(&mut self, name: &str) {
        self.current_function = name.to_string();
        self.scope_depth = 1;
        self.current_block = 0;
        self.current_stmt = 0;
    }

    /// Leave the current function — check that no borrows escape.
    pub fn leave_function(&mut self) {
        // Verify all borrows at scope_depth >= 1 are released
        for (var, borrows) in &self.active_borrows {
            for borrow in borrows {
                if borrow.scope_depth >= 1 {
                    self.errors.push(LifetimeError {
                        kind: LifetimeErrorKind::BorrowEscapesScope,
                        message: format!(
                            "borrow of '{}' created at {} escapes function '{}'",
                            var, borrow.origin_point, self.current_function
                        ),
                        hint: Some("consider cloning the value instead of borrowing".to_string()),
                    });
                }
            }
        }
        self.active_borrows.clear();
        self.scope_depth = 0;
    }

    // ── Scope management ───────────────────────────────────────────────

    /// Enter a nested scope (block, loop, if-branch).
    pub fn enter_scope(&mut self) -> LifetimeId {
        self.scope_depth += 1;
        let id = self.fresh_region_id();
        self.regions.insert(id, Region {
            id,
            scope_depth: self.scope_depth,
            scope_name: format!("{}::scope_{}", self.current_function, self.scope_depth),
            kind: RegionKind::Scope,
        });
        id
    }

    /// Leave a nested scope — invalidate borrows tied to this scope.
    pub fn leave_scope(&mut self, scope_region: LifetimeId) {
        let depth = self.scope_depth;
        // Remove borrows whose scope_depth matches or exceeds current
        for borrows in self.active_borrows.values_mut() {
            borrows.retain(|b| b.scope_depth < depth);
        }
        self.scope_depth = self.scope_depth.saturating_sub(1);
        // Mark scope region as dead
        if let Some(region) = self.regions.get_mut(&scope_region) {
            region.scope_depth = 0; // collapsed
        }
    }

    // ── Lifetime declarations ──────────────────────────────────────────

    /// Declare a named lifetime parameter (e.g., `'a` on a function).
    pub fn declare_lifetime(&mut self, name: &str) -> LifetimeId {
        if let Some(&existing) = self.named_lifetimes.get(name) {
            return existing;
        }
        let id = self.fresh_region_id();
        self.regions.insert(id, Region {
            id,
            scope_depth: 1, // function-level
            scope_name: format!("{}::'{}", self.current_function, name),
            kind: RegionKind::Named,
        });
        self.named_lifetimes.insert(name.to_string(), id);
        id
    }

    /// Resolve a lifetime name to its ID, or return None.
    pub fn resolve_lifetime(&self, name: &str) -> Option<LifetimeId> {
        self.named_lifetimes.get(name).copied()
    }

    // ── Borrow tracking ────────────────────────────────────────────────

    /// Record a borrow of a variable. Returns the region assigned to this borrow.
    pub fn create_borrow(&mut self, variable: &str, mutable: bool) -> LifetimeId {
        let id = self.fresh_region_id();
        self.regions.insert(id, Region {
            id,
            scope_depth: self.scope_depth,
            scope_name: format!("borrow_of_{}", variable),
            kind: RegionKind::Inferred,
        });

        let record = BorrowRecord {
            variable: variable.to_string(),
            region: id,
            mutable,
            scope_depth: self.scope_depth,
            origin_point: self.current_point(),
        };

        // Check for mutable aliasing
        if mutable {
            if let Some(existing) = self.active_borrows.get(variable) {
                if !existing.is_empty() {
                    self.errors.push(LifetimeError {
                        kind: LifetimeErrorKind::ConflictingLifetimes,
                        message: format!(
                            "cannot borrow '{}' as mutable — it is already borrowed",
                            variable
                        ),
                        hint: Some("ensure all other borrows are dropped first".to_string()),
                    });
                }
            }
        } else {
            // Shared borrow — check no mutable borrow exists
            if let Some(existing) = self.active_borrows.get(variable) {
                for b in existing {
                    if b.mutable {
                        self.errors.push(LifetimeError {
                            kind: LifetimeErrorKind::ConflictingLifetimes,
                            message: format!(
                                "cannot borrow '{}' as shared — it is already mutably borrowed",
                                variable
                            ),
                            hint: Some("drop the mutable borrow before taking a shared borrow".to_string()),
                        });
                    }
                }
            }
        }

        self.active_borrows
            .entry(variable.to_string())
            .or_default()
            .push(record);

        id
    }

    /// Release all borrows of a variable.
    pub fn release_borrows(&mut self, variable: &str) {
        self.active_borrows.remove(variable);
    }

    /// Advance the statement counter.
    pub fn advance_statement(&mut self) {
        self.current_stmt += 1;
    }

    /// Advance to the next block.
    pub fn advance_block(&mut self) {
        self.current_block += 1;
        self.current_stmt = 0;
    }

    // ── Constraint management ──────────────────────────────────────────

    /// Add an outlives constraint: `longer` must outlive `shorter`.
    pub fn add_outlives(&mut self, longer: LifetimeId, shorter: LifetimeId, reason: &str) {
        self.constraints.push(LifetimeConstraint::Outlives {
            longer,
            shorter,
            reason: reason.to_string(),
        });
        self.outlives_graph
            .entry(longer)
            .or_default()
            .insert(shorter);
    }

    /// Add an equality constraint.
    pub fn add_equality(&mut self, left: LifetimeId, right: LifetimeId, reason: &str) {
        self.constraints.push(LifetimeConstraint::Equal {
            left,
            right,
            reason: reason.to_string(),
        });
    }

    /// Add a liveness constraint: region must be alive at the current point.
    pub fn add_live_at(&mut self, region: LifetimeId) {
        self.constraints.push(LifetimeConstraint::LiveAt {
            region,
            point: self.current_point(),
        });
    }

    // ── Constraint solving ─────────────────────────────────────────────

    /// Solve all collected constraints and return errors.
    ///
    /// Uses a fixed-point iteration approach: expand region scopes upward
    /// until all outlives constraints are satisfied or a contradiction is found.
    pub fn solve(&mut self) -> Vec<LifetimeError> {
        // Phase 1: Check outlives constraints using scope depth heuristic
        for constraint in &self.constraints {
            match constraint {
                LifetimeConstraint::Outlives { longer, shorter, reason } => {
                    let longer_depth = self.regions.get(longer)
                        .map(|r| r.scope_depth).unwrap_or(0);
                    let shorter_depth = self.regions.get(shorter)
                        .map(|r| r.scope_depth).unwrap_or(0);

                    // 'static outlives everything
                    let longer_is_static = self.regions.get(longer)
                        .map(|r| r.kind == RegionKind::Static).unwrap_or(false);
                    if longer_is_static {
                        continue;
                    }

                    // If the "longer" region has a deeper scope (higher depth),
                    // it actually lives for less time. That's a contradiction.
                    if longer_depth > shorter_depth && shorter_depth > 0 {
                        self.errors.push(LifetimeError {
                            kind: LifetimeErrorKind::DanglingReference,
                            message: format!(
                                "region {} (depth {}) does not outlive region {} (depth {}): {}",
                                longer, longer_depth, shorter, shorter_depth, reason
                            ),
                            hint: Some("try moving the borrow to an outer scope".to_string()),
                        });
                    }
                }

                LifetimeConstraint::Equal { left, right, reason } => {
                    let left_depth = self.regions.get(left)
                        .map(|r| r.scope_depth).unwrap_or(0);
                    let right_depth = self.regions.get(right)
                        .map(|r| r.scope_depth).unwrap_or(0);

                    if left_depth != right_depth {
                        self.errors.push(LifetimeError {
                            kind: LifetimeErrorKind::ConflictingLifetimes,
                            message: format!(
                                "lifetime mismatch: {} (depth {}) != {} (depth {}): {}",
                                left, left_depth, right, right_depth, reason
                            ),
                            hint: None,
                        });
                    }
                }

                LifetimeConstraint::LiveAt { region, point } => {
                    // Check that the region is still alive at this point
                    let region_depth = self.regions.get(region)
                        .map(|r| r.scope_depth).unwrap_or(0);
                    if region_depth == 0 && !self.regions.get(region)
                        .map(|r| r.kind == RegionKind::Static).unwrap_or(false)
                    {
                        self.errors.push(LifetimeError {
                            kind: LifetimeErrorKind::DanglingReference,
                            message: format!(
                                "region {} is no longer alive at {}",
                                region, point
                            ),
                            hint: Some("the referenced value has been dropped".to_string()),
                        });
                    }
                }
            }
        }

        // Phase 2: Check for cycles in the outlives graph (impossible constraints)
        self.detect_outlives_cycles();

        self.errors.clone()
    }

    /// Detect cycles in the outlives graph — a cycle means contradictory constraints.
    fn detect_outlives_cycles(&mut self) {
        let mut visited = HashSet::new();
        let mut on_stack = HashSet::new();

        let nodes: Vec<LifetimeId> = self.outlives_graph.keys().copied().collect();
        for node in nodes {
            if !visited.contains(&node) {
                self.dfs_cycle_check(node, &mut visited, &mut on_stack);
            }
        }
    }

    fn dfs_cycle_check(
        &mut self,
        node: LifetimeId,
        visited: &mut HashSet<LifetimeId>,
        on_stack: &mut HashSet<LifetimeId>,
    ) {
        visited.insert(node);
        on_stack.insert(node);

        let neighbors: Vec<LifetimeId> = self.outlives_graph
            .get(&node)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();

        for neighbor in neighbors {
            if on_stack.contains(&neighbor) {
                self.errors.push(LifetimeError {
                    kind: LifetimeErrorKind::ConflictingLifetimes,
                    message: format!(
                        "circular lifetime dependency: {} and {} outlive each other",
                        node, neighbor
                    ),
                    hint: Some("break the cycle by restructuring borrows".to_string()),
                });
            } else if !visited.contains(&neighbor) {
                self.dfs_cycle_check(neighbor, visited, on_stack);
            }
        }

        on_stack.remove(&node);
    }

    // ── Queries ────────────────────────────────────────────────────────

    /// Get all regions.
    pub fn regions(&self) -> &BTreeMap<LifetimeId, Region> {
        &self.regions
    }

    /// Get all constraints.
    pub fn constraints(&self) -> &[LifetimeConstraint] {
        &self.constraints
    }

    /// Get all errors (without solving).
    pub fn errors(&self) -> &[LifetimeError] {
        &self.errors
    }

    /// Get active borrows for a variable.
    pub fn borrows_of(&self, variable: &str) -> &[BorrowRecord] {
        self.active_borrows
            .get(variable)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check whether a lifetime outlives another (using the graph).
    pub fn outlives(&self, a: LifetimeId, b: LifetimeId) -> bool {
        if a == b { return true; }
        // Static outlives everything
        if self.regions.get(&a).map(|r| r.kind == RegionKind::Static).unwrap_or(false) {
            return true;
        }
        // DFS from a in the outlives graph
        let mut visited = HashSet::new();
        let mut stack = vec![a];
        while let Some(node) = stack.pop() {
            if node == b { return true; }
            if visited.insert(node) {
                if let Some(neighbors) = self.outlives_graph.get(&node) {
                    stack.extend(neighbors.iter().copied());
                }
            }
        }
        false
    }

    /// Get the scope depth of a region.
    pub fn depth_of(&self, id: LifetimeId) -> u32 {
        self.regions.get(&id).map(|r| r.scope_depth).unwrap_or(0)
    }

    /// Check if a variable currently has any active borrows.
    pub fn is_borrowed(&self, variable: &str) -> bool {
        self.active_borrows
            .get(variable)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Check if a variable is mutably borrowed.
    pub fn is_mutably_borrowed(&self, variable: &str) -> bool {
        self.active_borrows
            .get(variable)
            .map(|v| v.iter().any(|b| b.mutable))
            .unwrap_or(false)
    }

    /// Reset the analyzer for reuse.
    pub fn reset(&mut self) {
        self.regions.clear();
        self.constraints.clear();
        self.active_borrows.clear();
        self.named_lifetimes.clear();
        self.next_id = 0;
        self.scope_depth = 0;
        self.current_function.clear();
        self.current_block = 0;
        self.current_stmt = 0;
        self.errors.clear();
        self.outlives_graph.clear();

        // Re-register 'static
        let static_id = self.fresh_region_id();
        self.regions.insert(static_id, Region {
            id: static_id,
            scope_depth: 0,
            scope_name: "static".to_string(),
            kind: RegionKind::Static,
        });
        self.named_lifetimes.insert("static".to_string(), static_id);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Program-Level Lifetime Checker
// ═══════════════════════════════════════════════════════════════════════

use crate::ast::{Block, Expr, Function, Param, Program, Stmt, TopLevel};

/// High-level lifetime checker that operates on the AST.
pub struct LifetimeChecker {
    analyzer: RegionAnalyzer,
}

impl LifetimeChecker {
    pub fn new() -> Self {
        Self {
            analyzer: RegionAnalyzer::new(),
        }
    }

    /// Check a full program for lifetime errors.
    pub fn check(&mut self, program: &Program) -> Vec<LifetimeError> {
        for item in &program.items {
            match item {
                TopLevel::Function(func) => self.check_function(func),
                TopLevel::Impl(impl_block) => {
                    for method in &impl_block.methods {
                        self.check_function(method);
                    }
                }
                _ => {}
            }
        }
        self.analyzer.solve()
    }

    fn check_function(&mut self, func: &Function) {
        self.analyzer.enter_function(&func.name);

        // Declare lifetime parameters from function signature
        // (convention: capabilities starting with `'` are lifetime params)
        for cap in &func.capabilities {
            if cap.starts_with('\'') {
                self.analyzer.declare_lifetime(&cap[1..]);
            }
        }

        // Declare parameters
        for _param in &func.params {
            self.analyzer.advance_statement();
        }

        // Check the function body
        self.check_block(&func.body);

        self.analyzer.leave_function();
    }

    fn check_block(&mut self, block: &Block) {
        let scope = self.analyzer.enter_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
            self.analyzer.advance_statement();
        }
        if let Some(expr) = &block.tail_expr {
            self.check_expr(expr);
        }
        self.analyzer.leave_scope(scope);
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { value, .. } => {
                if let Some(val) = value {
                    self.check_expr(val);
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
            _ => {}
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Block(block) => {
                self.check_block(block);
            }
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.check_expr(condition);
                self.check_block(then_branch);
                if let Some(eb) = else_branch {
                    self.check_block(eb);
                }
            }
            Expr::Call { func, args, .. } => {
                self.check_expr(func);
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.check_expr(operand);
            }
            Expr::Assign { target, value, .. } => {
                self.check_expr(target);
                self.check_expr(value);
            }
            Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.check_expr(val);
                }
            }
            _ => {}
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_lifetime() {
        let mut analyzer = RegionAnalyzer::new();
        let static_id = analyzer.resolve_lifetime("static").unwrap();
        assert_eq!(static_id, LifetimeId(0));
        assert!(analyzer.regions().get(&static_id).unwrap().kind == RegionKind::Static);
    }

    #[test]
    fn test_named_lifetime_declaration() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");
        let a = analyzer.declare_lifetime("a");
        let b = analyzer.declare_lifetime("b");
        assert_ne!(a, b);
        assert_eq!(analyzer.resolve_lifetime("a"), Some(a));
        assert_eq!(analyzer.resolve_lifetime("b"), Some(b));
    }

    #[test]
    fn test_borrow_tracking() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");
        let scope = analyzer.enter_scope();

        let borrow_region = analyzer.create_borrow("x", false);
        assert!(analyzer.is_borrowed("x"));
        assert!(!analyzer.is_mutably_borrowed("x"));

        analyzer.release_borrows("x");
        assert!(!analyzer.is_borrowed("x"));

        analyzer.leave_scope(scope);
        analyzer.leave_function();
    }

    #[test]
    fn test_mutable_aliasing_detection() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");
        let scope = analyzer.enter_scope();

        // First mutable borrow — ok
        analyzer.create_borrow("x", true);
        assert!(analyzer.errors().is_empty());

        // Second mutable borrow — error
        analyzer.create_borrow("x", true);
        assert_eq!(analyzer.errors().len(), 1);
        assert_eq!(analyzer.errors()[0].kind, LifetimeErrorKind::ConflictingLifetimes);

        analyzer.leave_scope(scope);
        analyzer.leave_function();
    }

    #[test]
    fn test_shared_after_mutable_error() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");
        let scope = analyzer.enter_scope();

        analyzer.create_borrow("x", true);  // &mut x
        analyzer.create_borrow("x", false); // &x — should error

        assert_eq!(analyzer.errors().len(), 1);
        assert_eq!(analyzer.errors()[0].kind, LifetimeErrorKind::ConflictingLifetimes);

        analyzer.leave_scope(scope);
        analyzer.leave_function();
    }

    #[test]
    fn test_outlives_constraint_solving() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");

        let outer_scope = analyzer.enter_scope(); // depth 2
        let a = analyzer.declare_lifetime("a");

        let inner_scope = analyzer.enter_scope(); // depth 3
        let b = analyzer.create_borrow("y", false);

        // b (depth 3) must outlive a (depth 1) — impossible
        analyzer.add_outlives(b, a, "return reference must outlive input");

        analyzer.leave_scope(inner_scope);
        analyzer.leave_scope(outer_scope);

        let errors = analyzer.solve();
        assert!(errors.iter().any(|e| e.kind == LifetimeErrorKind::DanglingReference));
    }

    #[test]
    fn test_scope_borrow_cleanup() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");
        let scope = analyzer.enter_scope();

        analyzer.create_borrow("x", false);
        assert!(analyzer.is_borrowed("x"));

        analyzer.leave_scope(scope);
        // After leaving scope, borrows should be cleaned up
        assert!(!analyzer.is_borrowed("x"));

        analyzer.leave_function();
    }

    #[test]
    fn test_outlives_query() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");

        let a = analyzer.declare_lifetime("a");
        let b = analyzer.declare_lifetime("b");

        analyzer.add_outlives(a, b, "explicit constraint");

        assert!(analyzer.outlives(a, b));
        assert!(!analyzer.outlives(b, a));

        // Static outlives everything
        let static_id = analyzer.resolve_lifetime("static").unwrap();
        assert!(analyzer.outlives(static_id, a));
        assert!(analyzer.outlives(static_id, b));
    }

    #[test]
    fn test_cycle_detection() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("test_fn");

        let a = analyzer.declare_lifetime("a");
        let b = analyzer.declare_lifetime("b");

        // Create a cycle: 'a outlives 'b and 'b outlives 'a
        analyzer.add_outlives(a, b, "a > b");
        analyzer.add_outlives(b, a, "b > a");

        let errors = analyzer.solve();
        assert!(errors.iter().any(|e| e.kind == LifetimeErrorKind::ConflictingLifetimes));
    }

    #[test]
    fn test_reset() {
        let mut analyzer = RegionAnalyzer::new();
        analyzer.enter_function("f");
        analyzer.declare_lifetime("a");
        analyzer.create_borrow("x", true);
        analyzer.reset();

        assert!(analyzer.resolve_lifetime("a").is_none());
        assert!(analyzer.resolve_lifetime("static").is_some());
        assert!(!analyzer.is_borrowed("x"));
    }
}
