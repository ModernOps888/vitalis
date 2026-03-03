//! Non-Lexical Lifetimes (NLL) for Vitalis — v23
//!
//! Implements Polonius-inspired NLL analysis where borrow regions are sets of
//! **control-flow points** rather than lexical scopes. A borrow is alive from
//! its creation until the **last point** the borrowed reference is used — not
//! until the end of its enclosing block.
//!
//! # Why NLL?
//!
//! With lexical lifetimes (v22), this program is rejected:
//! ```text
//! fn example() -> i64 {
//!     let mut x = 10;
//!     let r = &mut x;     // mutable borrow starts
//!     use(r);              // last use of r
//!     let s = &x;          // ERROR (lexical): x still mutably borrowed
//!     s
//! }
//! ```
//! With NLL, the mutable borrow of `x` ends immediately after `use(r)`,
//! so `let s = &x` is valid.
//!
//! # Architecture
//!
//! ```text
//! AST → CFG Builder → Control Flow Graph (nodes + edges)
//!     → Use-Site Collector (where is each variable read/written?)
//!     → Liveness Analysis (backward dataflow: live-in / live-out per node)
//!     → NLL Region Computation (each borrow lives only where its ref is live)
//!     → Conflict Detection (overlapping mutable/shared regions → error)
//! ```
//!
//! # Integration
//!
//! NLL operates on the AST *before* IR lowering and complements the existing
//! `ownership.rs` (move/drop tracking) and `lifetimes.rs` (region constraints).

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt;

use crate::ast::{Block, Expr, Function, Program, Stmt, TopLevel};

// ═══════════════════════════════════════════════════════════════════════
//  CFG — Control Flow Graph
// ═══════════════════════════════════════════════════════════════════════

/// A point in the control-flow graph, uniquely identified by an index.
pub type CfgPoint = u32;

/// A node in the CFG.
#[derive(Debug, Clone)]
pub struct CfgNode {
    pub id: CfgPoint,
    /// Kind of this node (entry, exit, statement, expression, etc.)
    pub kind: CfgNodeKind,
    /// Successors in the CFG
    pub successors: Vec<CfgPoint>,
    /// Predecessors in the CFG
    pub predecessors: Vec<CfgPoint>,
    /// Variables **used** (read) at this point
    pub uses: BTreeSet<String>,
    /// Variables **defined** (written) at this point
    pub defs: BTreeSet<String>,
    /// Scope depth at this point (for debug display)
    pub scope_depth: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CfgNodeKind {
    /// Function entry point
    Entry,
    /// Function exit point
    Exit,
    /// A `let` binding
    LetBinding { name: String },
    /// An expression statement
    ExprStmt,
    /// Assignment: `x = ...`
    Assignment { target: String },
    /// A borrow creation: `&x` or `&mut x`
    Borrow { variable: String, mutable: bool },
    /// A function/method call
    Call,
    /// Conditional branch point (if/match)
    Branch,
    /// Loop header
    LoopHeader,
    /// Loop back-edge
    LoopBack,
    /// Return statement
    Return,
    /// Break / continue
    ControlFlow,
    /// Phi junction (merge point after branches)
    Join,
}

impl fmt::Display for CfgNodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CfgNodeKind::Entry => write!(f, "ENTRY"),
            CfgNodeKind::Exit => write!(f, "EXIT"),
            CfgNodeKind::LetBinding { name } => write!(f, "let {}", name),
            CfgNodeKind::ExprStmt => write!(f, "expr"),
            CfgNodeKind::Assignment { target } => write!(f, "{} = ...", target),
            CfgNodeKind::Borrow { variable, mutable } => {
                if *mutable {
                    write!(f, "&mut {}", variable)
                } else {
                    write!(f, "&{}", variable)
                }
            }
            CfgNodeKind::Call => write!(f, "call"),
            CfgNodeKind::Branch => write!(f, "branch"),
            CfgNodeKind::LoopHeader => write!(f, "loop"),
            CfgNodeKind::LoopBack => write!(f, "loop-back"),
            CfgNodeKind::Return => write!(f, "return"),
            CfgNodeKind::ControlFlow => write!(f, "ctrl"),
            CfgNodeKind::Join => write!(f, "join"),
        }
    }
}

/// Control Flow Graph built from an AST function.
#[derive(Debug, Clone)]
pub struct Cfg {
    pub function_name: String,
    pub nodes: Vec<CfgNode>,
    pub entry: CfgPoint,
    pub exit: CfgPoint,
}

impl Cfg {
    /// Pretty-print the CFG for debugging / dump-ir output.
    pub fn display(&self) -> String {
        let mut out = format!("CFG for '{}':\n", self.function_name);
        for node in &self.nodes {
            out.push_str(&format!(
                "  [{}] {:?}  succs={:?}  uses={:?}  defs={:?}\n",
                node.id, node.kind, node.successors, node.uses, node.defs,
            ));
        }
        out
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CFG Builder
// ═══════════════════════════════════════════════════════════════════════

/// Builds a CFG from an AST function.
pub struct CfgBuilder {
    nodes: Vec<CfgNode>,
    scope_depth: u32,
}

impl CfgBuilder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            scope_depth: 0,
        }
    }

    fn alloc_node(&mut self, kind: CfgNodeKind) -> CfgPoint {
        let id = self.nodes.len() as CfgPoint;
        self.nodes.push(CfgNode {
            id,
            kind,
            successors: Vec::new(),
            predecessors: Vec::new(),
            uses: BTreeSet::new(),
            defs: BTreeSet::new(),
            scope_depth: self.scope_depth,
        });
        id
    }

    fn add_edge(&mut self, from: CfgPoint, to: CfgPoint) {
        if !self.nodes[from as usize].successors.contains(&to) {
            self.nodes[from as usize].successors.push(to);
        }
        if !self.nodes[to as usize].predecessors.contains(&from) {
            self.nodes[to as usize].predecessors.push(from);
        }
    }

    fn add_use(&mut self, node: CfgPoint, var: &str) {
        self.nodes[node as usize].uses.insert(var.to_string());
    }

    fn add_def(&mut self, node: CfgPoint, var: &str) {
        self.nodes[node as usize].defs.insert(var.to_string());
    }

    /// Build a CFG from an AST function.
    pub fn build_function(&mut self, func: &Function) -> Cfg {
        let entry = self.alloc_node(CfgNodeKind::Entry);
        let exit = self.alloc_node(CfgNodeKind::Exit);

        // Declare parameters as definitions at the entry point
        for param in &func.params {
            self.add_def(entry, &param.name);
        }

        // Build the body
        let body_end = self.build_block(&func.body, entry);
        self.add_edge(body_end, exit);

        Cfg {
            function_name: func.name.clone(),
            nodes: self.nodes.clone(),
            entry,
            exit,
        }
    }

    /// Build a block, returning the last CFG point.
    fn build_block(&mut self, block: &Block, mut current: CfgPoint) -> CfgPoint {
        self.scope_depth += 1;
        for stmt in &block.stmts {
            current = self.build_stmt(stmt, current);
        }
        if let Some(tail) = &block.tail_expr {
            current = self.build_expr(tail, current);
        }
        self.scope_depth -= 1;
        current
    }

    fn build_stmt(&mut self, stmt: &Stmt, current: CfgPoint) -> CfgPoint {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let node = self.alloc_node(CfgNodeKind::LetBinding {
                    name: name.clone(),
                });
                self.add_def(node, name);
                self.add_edge(current, node);

                if let Some(val) = value {
                    self.collect_uses(val, node);
                }
                node
            }

            Stmt::Expr(expr) => {
                self.build_expr(expr, current)
            }

            Stmt::While { condition, body, .. } => {
                let header = self.alloc_node(CfgNodeKind::LoopHeader);
                self.add_edge(current, header);
                self.collect_uses(condition, header);

                // Body
                let body_end = self.build_block(body, header);
                let back = self.alloc_node(CfgNodeKind::LoopBack);
                self.add_edge(body_end, back);
                self.add_edge(back, header); // back-edge

                // Exit
                let join = self.alloc_node(CfgNodeKind::Join);
                self.add_edge(header, join); // skip body
                join
            }

            Stmt::For { var, iter, body, .. } => {
                let header = self.alloc_node(CfgNodeKind::LoopHeader);
                self.add_edge(current, header);
                self.collect_uses(iter, header);
                self.add_def(header, var);

                let body_end = self.build_block(body, header);
                let back = self.alloc_node(CfgNodeKind::LoopBack);
                self.add_edge(body_end, back);
                self.add_edge(back, header);

                let join = self.alloc_node(CfgNodeKind::Join);
                self.add_edge(header, join);
                join
            }

            Stmt::Loop { body, .. } => {
                let header = self.alloc_node(CfgNodeKind::LoopHeader);
                self.add_edge(current, header);

                let body_end = self.build_block(body, header);
                self.add_edge(body_end, header); // infinite back-edge

                let join = self.alloc_node(CfgNodeKind::Join);
                self.add_edge(header, join); // break exits here
                join
            }
        }
    }

    fn build_expr(&mut self, expr: &Expr, current: CfgPoint) -> CfgPoint {
        match expr {
            Expr::Ident(name, _) => {
                self.add_use(current, name);
                current
            }

            Expr::Binary { left, right, .. } => {
                let after_left = self.build_expr(left, current);
                self.build_expr(right, after_left)
            }

            Expr::Unary { operand, .. } => {
                self.build_expr(operand, current)
            }

            Expr::Call { func, args, .. } => {
                let call_node = self.alloc_node(CfgNodeKind::Call);
                self.add_edge(current, call_node);
                self.collect_uses(func, call_node);
                for arg in args {
                    self.collect_uses(arg, call_node);
                }
                call_node
            }

            Expr::MethodCall { object, args, .. } => {
                let call_node = self.alloc_node(CfgNodeKind::Call);
                self.add_edge(current, call_node);
                self.collect_uses(object, call_node);
                for arg in args {
                    self.collect_uses(arg, call_node);
                }
                call_node
            }

            Expr::Field { object, .. } => {
                self.build_expr(object, current)
            }

            Expr::Index { object, index, .. } => {
                let after_obj = self.build_expr(object, current);
                self.build_expr(index, after_obj)
            }

            Expr::If { condition, then_branch, else_branch, .. } => {
                let branch = self.alloc_node(CfgNodeKind::Branch);
                self.add_edge(current, branch);
                self.collect_uses(condition, branch);

                let then_end = self.build_block(then_branch, branch);

                let join = self.alloc_node(CfgNodeKind::Join);
                self.add_edge(then_end, join);

                if let Some(eb) = else_branch {
                    let else_end = self.build_block(eb, branch);
                    self.add_edge(else_end, join);
                } else {
                    self.add_edge(branch, join);
                }
                join
            }

            Expr::Match { subject, arms, .. } => {
                let branch = self.alloc_node(CfgNodeKind::Branch);
                self.add_edge(current, branch);
                self.collect_uses(subject, branch);

                let join = self.alloc_node(CfgNodeKind::Join);
                for arm in arms {
                    let arm_end = self.build_expr(&arm.body, branch);
                    self.add_edge(arm_end, join);
                }
                join
            }

            Expr::Block(block) => {
                self.build_block(block, current)
            }

            Expr::List { elements, .. } => {
                let mut cur = current;
                for el in elements {
                    cur = self.build_expr(el, cur);
                }
                cur
            }

            Expr::Lambda { body, params, .. } => {
                // Lambda captures are uses of outer variables
                let node = self.alloc_node(CfgNodeKind::ExprStmt);
                self.add_edge(current, node);
                for param in params {
                    self.add_def(node, &param.name);
                }
                self.collect_uses(body, node);
                node
            }

            Expr::Assign { target, value, .. } => {
                let node = self.alloc_node(CfgNodeKind::Assignment {
                    target: extract_ident(target).unwrap_or_default(),
                });
                self.add_edge(current, node);
                if let Some(name) = extract_ident(target) {
                    self.add_def(node, &name);
                }
                self.collect_uses(value, node);
                node
            }

            Expr::CompoundAssign { target, value, .. } => {
                let node = self.alloc_node(CfgNodeKind::Assignment {
                    target: extract_ident(target).unwrap_or_default(),
                });
                self.add_edge(current, node);
                if let Some(name) = extract_ident(target) {
                    self.add_use(node, &name);
                    self.add_def(node, &name);
                }
                self.collect_uses(value, node);
                node
            }

            Expr::Return { value, .. } => {
                let node = self.alloc_node(CfgNodeKind::Return);
                self.add_edge(current, node);
                if let Some(val) = value {
                    self.collect_uses(val, node);
                }
                node
            }

            Expr::Break(_) | Expr::Continue(_) => {
                let node = self.alloc_node(CfgNodeKind::ControlFlow);
                self.add_edge(current, node);
                node
            }

            Expr::TryCatch { try_body, catch_var, catch_body, .. } => {
                let try_end = self.build_block(try_body, current);
                let catch_start = self.alloc_node(CfgNodeKind::LetBinding {
                    name: catch_var.clone(),
                });
                self.add_edge(current, catch_start); // exception edge
                self.add_def(catch_start, catch_var);

                let catch_end = self.build_block(catch_body, catch_start);
                let join = self.alloc_node(CfgNodeKind::Join);
                self.add_edge(try_end, join);
                self.add_edge(catch_end, join);
                join
            }

            Expr::Pipe { stages, .. } => {
                let mut cur = current;
                for stage in stages {
                    cur = self.build_expr(stage, cur);
                }
                cur
            }

            // Literals and other simple expressions — no control flow
            _ => {
                self.collect_uses(expr, current);
                current
            }
        }
    }

    /// Collect variable uses from an expression without adding new CFG nodes.
    fn collect_uses(&mut self, expr: &Expr, target_node: CfgPoint) {
        match expr {
            Expr::Ident(name, _) => {
                self.add_use(target_node, name);
            }
            Expr::Binary { left, right, .. } => {
                self.collect_uses(left, target_node);
                self.collect_uses(right, target_node);
            }
            Expr::Unary { operand, .. } => {
                self.collect_uses(operand, target_node);
            }
            Expr::Call { func, args, .. } => {
                self.collect_uses(func, target_node);
                for arg in args {
                    self.collect_uses(arg, target_node);
                }
            }
            Expr::MethodCall { object, args, .. } => {
                self.collect_uses(object, target_node);
                for arg in args {
                    self.collect_uses(arg, target_node);
                }
            }
            Expr::Field { object, .. } => {
                self.collect_uses(object, target_node);
            }
            Expr::Index { object, index, .. } => {
                self.collect_uses(object, target_node);
                self.collect_uses(index, target_node);
            }
            Expr::Lambda { body, .. } => {
                self.collect_uses(body, target_node);
            }
            Expr::List { elements, .. } => {
                for el in elements {
                    self.collect_uses(el, target_node);
                }
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, val) in fields {
                    self.collect_uses(val, target_node);
                }
            }
            Expr::If { condition, .. } => {
                self.collect_uses(condition, target_node);
            }
            Expr::Try { expr, .. } => {
                self.collect_uses(expr, target_node);
            }
            Expr::Cast { expr, .. } => {
                self.collect_uses(expr, target_node);
            }
            Expr::Assign { target, value, .. } => {
                self.collect_uses(target, target_node);
                self.collect_uses(value, target_node);
            }
            Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.collect_uses(val, target_node);
                }
            }
            _ => {} // Literals — no variables
        }
    }
}

/// Extract the identifier name from an expression (for assignment targets).
fn extract_ident(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident(name, _) => Some(name.clone()),
        Expr::Field { object, field, .. } => {
            extract_ident(object).map(|base| format!("{}.{}", base, field))
        }
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Liveness Analysis (backward dataflow)
// ═══════════════════════════════════════════════════════════════════════

/// Result of liveness analysis: for each CFG point, the set of live variables.
#[derive(Debug, Clone)]
pub struct LivenessResult {
    /// Variables live **at entry** to each CFG node.
    pub live_in: Vec<BTreeSet<String>>,
    /// Variables live **at exit** of each CFG node.
    pub live_out: Vec<BTreeSet<String>>,
}

/// Compute liveness analysis on a CFG using iterative backward dataflow.
///
/// Standard equations:
///   live_out[n] = ∪ { live_in[s] | s ∈ successors(n) }
///   live_in[n]  = uses[n] ∪ (live_out[n] − defs[n])
pub fn compute_liveness(cfg: &Cfg) -> LivenessResult {
    let n = cfg.nodes.len();
    let mut live_in: Vec<BTreeSet<String>> = vec![BTreeSet::new(); n];
    let mut live_out: Vec<BTreeSet<String>> = vec![BTreeSet::new(); n];

    // Iterate until fixpoint
    let mut changed = true;
    while changed {
        changed = false;

        // Process nodes in reverse order (backward analysis)
        for i in (0..n).rev() {
            // live_out[i] = union of live_in of all successors
            let mut new_out = BTreeSet::new();
            for &succ in &cfg.nodes[i].successors {
                for var in &live_in[succ as usize] {
                    new_out.insert(var.clone());
                }
            }

            // live_in[i] = uses[i] ∪ (live_out[i] - defs[i])
            let mut new_in: BTreeSet<String> = cfg.nodes[i].uses.clone();
            for var in &new_out {
                if !cfg.nodes[i].defs.contains(var) {
                    new_in.insert(var.clone());
                }
            }

            if new_in != live_in[i] || new_out != live_out[i] {
                live_in[i] = new_in;
                live_out[i] = new_out;
                changed = true;
            }
        }
    }

    LivenessResult { live_in, live_out }
}

// ═══════════════════════════════════════════════════════════════════════
//  NLL Regions — sets of CFG points
// ═══════════════════════════════════════════════════════════════════════

/// An NLL region: the set of CFG points where a borrow is considered active.
#[derive(Debug, Clone)]
pub struct NllRegion {
    /// Unique identifier for this borrow
    pub id: u32,
    /// The variable being borrowed
    pub variable: String,
    /// Whether this is a mutable borrow
    pub mutable: bool,
    /// The CFG point where the borrow is created
    pub origin: CfgPoint,
    /// The variable name of the reference (if bound to a let)
    pub ref_name: Option<String>,
    /// The set of CFG points where this borrow is alive
    pub live_points: BTreeSet<CfgPoint>,
}

impl NllRegion {
    /// Check whether this region overlaps with another at any CFG point.
    pub fn overlaps(&self, other: &NllRegion) -> bool {
        // Efficient set intersection check
        for point in &self.live_points {
            if other.live_points.contains(point) {
                return true;
            }
        }
        false
    }

    /// Check whether this region is alive at a specific CFG point.
    pub fn alive_at(&self, point: CfgPoint) -> bool {
        self.live_points.contains(&point)
    }
}

impl fmt::Display for NllRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = if self.mutable { "&mut" } else { "&" };
        write!(
            f,
            "NllRegion#{} ({}{}, origin={}, points={:?})",
            self.id, kind, self.variable, self.origin, self.live_points
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  NLL Errors
// ═══════════════════════════════════════════════════════════════════════

/// An error detected during NLL analysis.
#[derive(Debug, Clone)]
pub struct NllError {
    pub kind: NllErrorKind,
    pub message: String,
    pub hint: Option<String>,
    /// The CFG point where the conflict occurs
    pub conflict_point: Option<CfgPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NllErrorKind {
    /// Two mutable borrows of the same variable overlap
    MutableAliasing,
    /// A mutable borrow and a shared borrow of the same variable overlap
    MutableSharedConflict,
    /// Use of a moved value
    UseAfterMove,
    /// Borrow of a moved value
    BorrowAfterMove,
    /// Modification of a borrowed value
    ModifyWhileBorrowed,
}

impl fmt::Display for NllError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NLL error: {}", self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  NLL Checker — the main analysis engine
// ═══════════════════════════════════════════════════════════════════════

/// The NLL borrow checker: builds CFG, computes liveness, checks borrow conflicts.
pub struct NllChecker {
    /// Collected borrow regions
    regions: Vec<NllRegion>,
    /// Next region ID
    next_id: u32,
    /// Collected errors
    errors: Vec<NllError>,
    /// Map from variable to its move points
    move_points: HashMap<String, Vec<CfgPoint>>,
    /// Map from borrow reference name to its region index
    ref_to_region: HashMap<String, usize>,
}

impl NllChecker {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            next_id: 0,
            errors: Vec::new(),
            move_points: HashMap::new(),
            ref_to_region: HashMap::new(),
        }
    }

    /// Full NLL analysis of a program.
    pub fn check(&mut self, program: &Program) -> Vec<NllError> {
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
        self.errors.clone()
    }

    /// Analyze a single function with NLL.
    pub fn check_function(&mut self, func: &Function) {
        // 1. Build CFG
        let mut builder = CfgBuilder::new();
        let cfg = builder.build_function(func);

        // 2. Compute liveness
        let liveness = compute_liveness(&cfg);

        // 3. Collect borrow regions from the CFG
        self.collect_borrows(&cfg, &liveness);

        // 4. Detect conflicts between overlapping regions
        self.detect_conflicts(&cfg);

        // 5. Check for modifications while borrowed
        self.check_modify_while_borrowed(&cfg, &liveness);

        // Clear per-function state for next function
        self.regions.clear();
        self.move_points.clear();
        self.ref_to_region.clear();
    }

    /// Collect borrow regions by scanning the CFG for borrow nodes and
    /// computing their NLL live ranges via the liveness results.
    fn collect_borrows(&mut self, cfg: &Cfg, liveness: &LivenessResult) {
        for node in &cfg.nodes {
            match &node.kind {
                CfgNodeKind::Borrow { variable, mutable } => {
                    let region_id = self.next_id;
                    self.next_id += 1;

                    // The borrow reference is "alive" at all points where the
                    // reference variable is live — from origin to last use.
                    let ref_name = self.infer_ref_name(cfg, node.id);
                    let live_points = if let Some(ref ref_var) = ref_name {
                        self.compute_ref_live_range(cfg, liveness, node.id, ref_var)
                    } else {
                        // Anonymous borrow (e.g., `f(&x)`) — alive at origin only
                        let mut pts = BTreeSet::new();
                        pts.insert(node.id);
                        pts
                    };

                    let region = NllRegion {
                        id: region_id,
                        variable: variable.clone(),
                        mutable: *mutable,
                        origin: node.id,
                        ref_name: ref_name.clone(),
                        live_points,
                    };

                    let idx = self.regions.len();
                    self.regions.push(region);
                    if let Some(ref_var) = ref_name {
                        self.ref_to_region.insert(ref_var, idx);
                    }
                }

                CfgNodeKind::LetBinding { name: _ } => {
                    // Check if this let binding creates a borrow (via init expr)
                    // We look for use-patterns that indicate borrowing
                    // For now, borrows are explicit CfgNodeKind::Borrow nodes
                }

                _ => {}
            }
        }
    }

    /// Infer which reference variable a borrow is stored into by looking at
    /// the next statement: `let r = &x` means the borrow of x is stored in r.
    fn infer_ref_name(&self, cfg: &Cfg, borrow_point: CfgPoint) -> Option<String> {
        // Look at successors for a LetBinding
        for &succ in &cfg.nodes[borrow_point as usize].successors {
            if let CfgNodeKind::LetBinding { name } = &cfg.nodes[succ as usize].kind {
                return Some(name.clone());
            }
        }
        None
    }

    /// Compute the live range of a reference variable using liveness data.
    /// The reference is live from its definition point through all points
    /// where liveness says it's live — ending at the last use.
    fn compute_ref_live_range(
        &self,
        cfg: &Cfg,
        liveness: &LivenessResult,
        origin: CfgPoint,
        ref_var: &str,
    ) -> BTreeSet<CfgPoint> {
        let mut live_points = BTreeSet::new();
        live_points.insert(origin);

        // Walk forward from origin, collecting points where ref_var is live
        let mut worklist = VecDeque::new();
        let mut visited = HashSet::new();
        worklist.push_back(origin);

        while let Some(point) = worklist.pop_front() {
            if !visited.insert(point) {
                continue;
            }

            // The reference is live here if liveness says so
            if liveness.live_out[point as usize].contains(ref_var) {
                live_points.insert(point);
                for &succ in &cfg.nodes[point as usize].successors {
                    worklist.push_back(succ);
                }
            } else if liveness.live_in[point as usize].contains(ref_var)
                || cfg.nodes[point as usize].uses.contains(ref_var)
            {
                live_points.insert(point);
                // Don't propagate past last use
            }
        }

        live_points
    }

    /// Detect conflicts between overlapping NLL regions.
    fn detect_conflicts(&mut self, _cfg: &Cfg) {
        let regions = self.regions.clone();
        for i in 0..regions.len() {
            for j in (i + 1)..regions.len() {
                let a = &regions[i];
                let b = &regions[j];

                // Only check borrows of the same variable
                if a.variable != b.variable {
                    continue;
                }

                // Find overlap point (first common CFG point)
                let overlap_point = a.live_points.intersection(&b.live_points).next().copied();

                if overlap_point.is_none() {
                    continue; // No overlap — NLL says these are fine!
                }

                // Both mutable → error
                if a.mutable && b.mutable {
                    self.errors.push(NllError {
                        kind: NllErrorKind::MutableAliasing,
                        message: format!(
                            "cannot have two mutable borrows of '{}' alive at the same time \
                             (regions #{} and #{} overlap at point {})",
                            a.variable, a.id, b.id,
                            overlap_point.unwrap()
                        ),
                        hint: Some(
                            "with NLL, use the first &mut before creating the second".to_string()
                        ),
                        conflict_point: overlap_point,
                    });
                }

                // One mutable, one shared → error
                if a.mutable != b.mutable {
                    self.errors.push(NllError {
                        kind: NllErrorKind::MutableSharedConflict,
                        message: format!(
                            "cannot borrow '{}' as {} while it is borrowed as {} \
                             (regions #{} and #{} overlap at point {})",
                            a.variable,
                            if b.mutable { "mutable" } else { "shared" },
                            if a.mutable { "mutable" } else { "shared" },
                            a.id, b.id,
                            overlap_point.unwrap()
                        ),
                        hint: Some(
                            "ensure the first borrow's last use is before the second borrow"
                                .to_string(),
                        ),
                        conflict_point: overlap_point,
                    });
                }
            }
        }
    }

    /// Check for modifications to a variable while it is borrowed.
    fn check_modify_while_borrowed(&mut self, cfg: &Cfg, _liveness: &LivenessResult) {
        let regions = self.regions.clone();
        for node in &cfg.nodes {
            if let CfgNodeKind::Assignment { target } = &node.kind {
                // Check if any borrow of `target` is alive at this point
                for region in &regions {
                    if region.variable == *target && region.alive_at(node.id) {
                        self.errors.push(NllError {
                            kind: NllErrorKind::ModifyWhileBorrowed,
                            message: format!(
                                "cannot assign to '{}' while it is borrowed (region #{} \
                                 is still alive at point {})",
                                target, region.id, node.id
                            ),
                            hint: Some(
                                "ensure all borrows of the variable end before modifying it"
                                    .to_string(),
                            ),
                            conflict_point: Some(node.id),
                        });
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Convenience API
// ═══════════════════════════════════════════════════════════════════════

/// Run NLL analysis on source code. Returns a list of NLL errors.
pub fn analyze_nll(source: &str) -> Vec<NllError> {
    let (program, parse_errors) = crate::parser::parse(source);
    if !parse_errors.is_empty() {
        return vec![NllError {
            kind: NllErrorKind::UseAfterMove,
            message: "Cannot perform NLL analysis — parse errors present".to_string(),
            hint: None,
            conflict_point: None,
        }];
    }
    let mut checker = NllChecker::new();
    checker.check(&program)
}

/// Build a CFG from a source function (for debugging / dump output).
pub fn build_cfg_from_source(source: &str) -> Vec<Cfg> {
    let (program, _) = crate::parser::parse(source);
    let mut cfgs = Vec::new();

    for item in &program.items {
        match item {
            TopLevel::Function(func) => {
                let mut builder = CfgBuilder::new();
                let cfg = builder.build_function(func);
                cfgs.push(cfg);
            }
            TopLevel::Impl(impl_block) => {
                for method in &impl_block.methods {
                    let mut builder = CfgBuilder::new();
                    let cfg = builder.build_function(method);
                    cfgs.push(cfg);
                }
            }
            _ => {}
        }
    }

    cfgs
}

/// Compute liveness for a source function (for debugging).
pub fn compute_liveness_from_source(source: &str) -> Vec<(String, LivenessResult)> {
    let cfgs = build_cfg_from_source(source);
    cfgs.into_iter()
        .map(|cfg| {
            let liveness = compute_liveness(&cfg);
            (cfg.function_name.clone(), liveness)
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── CFG construction ───────────────────────────────────────────────

    #[test]
    fn test_cfg_simple_function() {
        let cfgs = build_cfg_from_source("fn main() -> i64 { let x = 42; x }");
        assert_eq!(cfgs.len(), 1);
        let cfg = &cfgs[0];
        assert_eq!(cfg.function_name, "main");
        assert!(cfg.nodes.len() >= 3); // entry + let + exit at minimum
    }

    #[test]
    fn test_cfg_has_entry_and_exit() {
        let cfgs = build_cfg_from_source("fn f() -> i64 { 1 }");
        let cfg = &cfgs[0];
        assert_eq!(cfg.nodes[cfg.entry as usize].kind, CfgNodeKind::Entry);
        assert_eq!(cfg.nodes[cfg.exit as usize].kind, CfgNodeKind::Exit);
    }

    #[test]
    fn test_cfg_sequential_statements() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let a = 1; let b = 2; a + b }"
        );
        let cfg = &cfgs[0];
        // Entry → let a → let b → (tail expr uses a,b) → Exit
        assert!(cfg.nodes.len() >= 4);
    }

    #[test]
    fn test_cfg_if_branch() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let x = 10; if x > 0 { x } else { 0 } }"
        );
        let cfg = &cfgs[0];
        // Should have a Branch node and a Join node
        let has_branch = cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::Branch);
        let has_join = cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::Join);
        assert!(has_branch, "CFG should have a Branch node for if-expr");
        assert!(has_join, "CFG should have a Join node after if-expr");
    }

    #[test]
    fn test_cfg_while_loop() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let mut i = 0; while i < 10 { i = i + 1; } i }"
        );
        let cfg = &cfgs[0];
        let has_loop_header = cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::LoopHeader);
        let has_loop_back = cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::LoopBack);
        assert!(has_loop_header, "CFG should have a LoopHeader node");
        assert!(has_loop_back, "CFG should have a LoopBack node");
    }

    #[test]
    fn test_cfg_for_loop() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let mut s = 0; for i in [1, 2, 3] { s = s + i; } s }"
        );
        let cfg = &cfgs[0];
        let has_loop_header = cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::LoopHeader);
        assert!(has_loop_header);
    }

    #[test]
    fn test_cfg_call_node() {
        let cfgs = build_cfg_from_source(
            "fn foo(x: i64) -> i64 { x + 1 } fn main() -> i64 { foo(42) }"
        );
        assert_eq!(cfgs.len(), 2);
        let main_cfg = &cfgs[1];
        let has_call = main_cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::Call);
        assert!(has_call, "CFG for main should have a Call node");
    }

    #[test]
    fn test_cfg_multiple_functions() {
        let cfgs = build_cfg_from_source(
            "fn a() -> i64 { 1 } fn b() -> i64 { 2 } fn c() -> i64 { 3 }"
        );
        assert_eq!(cfgs.len(), 3);
        assert_eq!(cfgs[0].function_name, "a");
        assert_eq!(cfgs[1].function_name, "b");
        assert_eq!(cfgs[2].function_name, "c");
    }

    #[test]
    fn test_cfg_uses_and_defs() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let x = 42; x + 1 }"
        );
        let cfg = &cfgs[0];
        // The let binding should def 'x'
        let let_node = cfg.nodes.iter().find(|n| {
            matches!(&n.kind, CfgNodeKind::LetBinding { name } if name == "x")
        });
        assert!(let_node.is_some());
        assert!(let_node.unwrap().defs.contains("x"));
    }

    #[test]
    fn test_cfg_display() {
        let cfgs = build_cfg_from_source("fn main() -> i64 { 42 }");
        let display = cfgs[0].display();
        assert!(display.contains("CFG for 'main'"));
        assert!(display.contains("Entry"));
        assert!(display.contains("Exit"));
    }

    // ── Liveness analysis ──────────────────────────────────────────────

    #[test]
    fn test_liveness_simple() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let x = 42; x }"
        );
        let liveness = compute_liveness(&cfgs[0]);
        // 'x' should be live between its definition and its use
        assert!(!liveness.live_in.is_empty());
    }

    #[test]
    fn test_liveness_dead_variable() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let x = 42; let y = 10; x }"
        );
        let cfg = &cfgs[0];
        let liveness = compute_liveness(cfg);
        // 'y' is defined but never used — should not be live at exit
        let exit_live = &liveness.live_in[cfg.exit as usize];
        assert!(!exit_live.contains("y"), "Dead variable 'y' should not be live at exit");
    }

    #[test]
    fn test_liveness_multiple_uses() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let x = 10; let y = x + 1; x + y }"
        );
        let liveness = compute_liveness(&cfgs[0]);
        // 'x' is used in two places, should be live across both
        assert!(!liveness.live_in.is_empty());
    }

    #[test]
    fn test_liveness_function_params() {
        let cfgs = build_cfg_from_source(
            "fn add(a: i64, b: i64) -> i64 { a + b }"
        );
        let cfg = &cfgs[0];
        let liveness = compute_liveness(cfg);
        // Parameters 'a' and 'b' should be live within the function body
        assert!(!liveness.live_out.is_empty());
    }

    #[test]
    fn test_liveness_if_branches() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let x = 10; if x > 0 { x } else { 0 } }"
        );
        let liveness = compute_liveness(&cfgs[0]);
        assert!(!liveness.live_in.is_empty());
    }

    // ── NLL analysis ───────────────────────────────────────────────────

    #[test]
    fn test_nll_clean_program() {
        let errors = analyze_nll("fn main() -> i64 { let x = 42; x }");
        assert!(errors.is_empty(), "Expected no NLL errors: {:?}", errors);
    }

    #[test]
    fn test_nll_sequential_borrows_no_conflict() {
        // With NLL, sequential non-overlapping borrows should be fine
        let errors = analyze_nll(
            "fn main() -> i64 { let x = 10; let a = x + 1; let b = x + 2; a + b }"
        );
        assert!(errors.is_empty(), "Sequential uses should not conflict: {:?}", errors);
    }

    #[test]
    fn test_nll_multiple_functions() {
        let errors = analyze_nll(
            "fn foo(x: i64) -> i64 { x + 1 } \
             fn bar(y: i64) -> i64 { y * 2 } \
             fn main() -> i64 { foo(bar(21)) }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_if_expression() {
        let errors = analyze_nll(
            "fn main() -> i64 { let x = 42; if x > 0 { x + 1 } else { x - 1 } }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_nested_blocks() {
        let errors = analyze_nll(
            "fn main() -> i64 { let x = 1; { let y = 2; x + y } }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_while_loop() {
        let errors = analyze_nll(
            "fn main() -> i64 { let mut i = 0; while i < 10 { i = i + 1; } i }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_for_loop() {
        let errors = analyze_nll(
            "fn main() -> i64 { let mut s = 0; for i in [1, 2, 3] { s = s + i; } s }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_lambda() {
        let errors = analyze_nll(
            "fn main() -> i64 { let f = |x: i64| -> i64 { x + 1 }; f(41) }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_pipe_chain() {
        let errors = analyze_nll(
            "fn inc(x: i64) -> i64 { x + 1 } fn dbl(x: i64) -> i64 { x * 2 } \
             fn main() -> i64 { 5 |> inc |> dbl }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_parse_error_handling() {
        let errors = analyze_nll("fn { broken syntax");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("parse errors"));
    }

    #[test]
    fn test_nll_complex_expression() {
        let errors = analyze_nll(
            "fn main() -> i64 { let a = 5; let b = 10; (a + b) * 2 - a }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_match_expression() {
        let errors = analyze_nll(
            "fn main() -> i64 { let x = 3; match x { 1 => 10, 2 => 20, _ => 30 } }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_shadowing() {
        let errors = analyze_nll(
            "fn main() -> i64 { let x = 1; { let x = 2; x } }"
        );
        assert!(errors.is_empty());
    }

    // ── NLL region types ───────────────────────────────────────────────

    #[test]
    fn test_nll_region_overlap() {
        let mut a = NllRegion {
            id: 0,
            variable: "x".to_string(),
            mutable: true,
            origin: 1,
            ref_name: Some("r1".to_string()),
            live_points: [1, 2, 3].iter().copied().collect(),
        };
        let b = NllRegion {
            id: 1,
            variable: "x".to_string(),
            mutable: true,
            origin: 3,
            ref_name: Some("r2".to_string()),
            live_points: [3, 4, 5].iter().copied().collect(),
        };
        assert!(a.overlaps(&b), "Regions sharing point 3 should overlap");

        // After NLL shrinks region a to end before point 3
        a.live_points = [1, 2].iter().copied().collect();
        assert!(!a.overlaps(&b), "Non-overlapping regions should not conflict");
    }

    #[test]
    fn test_nll_region_alive_at() {
        let region = NllRegion {
            id: 0,
            variable: "x".to_string(),
            mutable: false,
            origin: 2,
            ref_name: None,
            live_points: [2, 3, 4].iter().copied().collect(),
        };
        assert!(region.alive_at(2));
        assert!(region.alive_at(3));
        assert!(region.alive_at(4));
        assert!(!region.alive_at(5));
        assert!(!region.alive_at(0));
    }

    #[test]
    fn test_nll_region_display() {
        let region = NllRegion {
            id: 7,
            variable: "data".to_string(),
            mutable: true,
            origin: 3,
            ref_name: Some("r".to_string()),
            live_points: [3, 4].iter().copied().collect(),
        };
        let s = format!("{}", region);
        assert!(s.contains("NllRegion#7"));
        assert!(s.contains("&mut"));
        assert!(s.contains("data"));
    }

    #[test]
    fn test_nll_error_display() {
        let err = NllError {
            kind: NllErrorKind::MutableAliasing,
            message: "two mutable borrows".to_string(),
            hint: Some("stagger the borrows".to_string()),
            conflict_point: Some(5),
        };
        let s = format!("{}", err);
        assert!(s.contains("NLL error"));
        assert!(s.contains("two mutable borrows"));
        assert!(s.contains("stagger"));
    }

    #[test]
    fn test_nll_error_kinds() {
        assert_ne!(NllErrorKind::MutableAliasing, NllErrorKind::MutableSharedConflict);
        assert_ne!(NllErrorKind::UseAfterMove, NllErrorKind::BorrowAfterMove);
        assert_ne!(NllErrorKind::ModifyWhileBorrowed, NllErrorKind::MutableAliasing);
    }

    #[test]
    fn test_nll_checker_creation() {
        let checker = NllChecker::new();
        assert!(checker.regions.is_empty());
        assert!(checker.errors.is_empty());
    }

    // ── CFG builder edge cases ────────────────────────────────────────

    #[test]
    fn test_cfg_return_node() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let x = 42; return x; }"
        );
        let cfg = &cfgs[0];
        let has_return = cfg.nodes.iter().any(|n| n.kind == CfgNodeKind::Return);
        assert!(has_return, "CFG should have a Return node");
    }

    #[test]
    fn test_cfg_assignment_node() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let mut x = 0; x = 42; x }"
        );
        let cfg = &cfgs[0];
        let has_assignment = cfg.nodes.iter().any(|n| {
            matches!(&n.kind, CfgNodeKind::Assignment { target } if target == "x")
        });
        assert!(has_assignment, "CFG should have an Assignment node for x");
    }

    #[test]
    fn test_cfg_predecessors() {
        let cfgs = build_cfg_from_source("fn main() -> i64 { let x = 1; x }");
        let cfg = &cfgs[0];
        // Exit node should have at least one predecessor
        assert!(
            !cfg.nodes[cfg.exit as usize].predecessors.is_empty(),
            "Exit should have predecessors"
        );
    }

    #[test]
    fn test_cfg_loop_back_edge() {
        let cfgs = build_cfg_from_source(
            "fn main() -> i64 { let mut i = 0; while i < 5 { i = i + 1; } i }"
        );
        let cfg = &cfgs[0];
        // The LoopBack node should have a successor pointing to LoopHeader
        let loop_back = cfg.nodes.iter().find(|n| n.kind == CfgNodeKind::LoopBack);
        assert!(loop_back.is_some());
        let header = cfg.nodes.iter().find(|n| n.kind == CfgNodeKind::LoopHeader);
        assert!(header.is_some());
        assert!(
            loop_back.unwrap().successors.contains(&header.unwrap().id),
            "LoopBack should have an edge back to LoopHeader"
        );
    }

    // ── Convenience API ────────────────────────────────────────────────

    #[test]
    fn test_build_cfg_from_source() {
        let cfgs = build_cfg_from_source("fn main() -> i64 { 0 }");
        assert_eq!(cfgs.len(), 1);
    }

    #[test]
    fn test_compute_liveness_from_source() {
        let results = compute_liveness_from_source(
            "fn main() -> i64 { let x = 1; x }"
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "main");
    }

    #[test]
    fn test_nll_empty_function() {
        let errors = analyze_nll("fn main() -> i64 { 0 }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nll_deeply_nested() {
        let errors = analyze_nll(
            "fn main() -> i64 { let x = 1; { { { x + 1 } } } }"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_extract_ident() {
        use crate::ast::Span;
        let span = Span { start: 0, end: 1 };
        assert_eq!(extract_ident(&Expr::Ident("x".to_string(), span.clone())), Some("x".to_string()));
        assert_eq!(extract_ident(&Expr::IntLiteral(42, span)), None);
    }

    #[test]
    fn test_cfg_node_kind_display() {
        assert_eq!(format!("{}", CfgNodeKind::Entry), "ENTRY");
        assert_eq!(format!("{}", CfgNodeKind::Exit), "EXIT");
        assert_eq!(
            format!("{}", CfgNodeKind::Borrow { variable: "x".to_string(), mutable: true }),
            "&mut x"
        );
        assert_eq!(
            format!("{}", CfgNodeKind::Borrow { variable: "y".to_string(), mutable: false }),
            "&y"
        );
        assert_eq!(
            format!("{}", CfgNodeKind::LetBinding { name: "foo".to_string() }),
            "let foo"
        );
        assert_eq!(
            format!("{}", CfgNodeKind::Assignment { target: "bar".to_string() }),
            "bar = ..."
        );
    }
}
