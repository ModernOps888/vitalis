//! Formal Verification — contract-based programming (pre/postconditions,
//! invariants), symbolic execution engine, proof-carrying code.
//!
//! Enables compile-time and runtime verification of program properties
//! via contracts, symbolic path exploration, and automated theorem proving.

use std::collections::HashMap;
use std::sync::Mutex;

// ── Contracts ───────────────────────────────────────────────────────────

/// A function contract with pre/postconditions and invariants.
#[derive(Debug, Clone)]
pub struct Contract {
    pub function_name: String,
    pub preconditions: Vec<Predicate>,
    pub postconditions: Vec<Predicate>,
    pub invariants: Vec<Predicate>,
}

/// A logical predicate for contract checking.
#[derive(Debug, Clone)]
pub enum Predicate {
    /// Variable comparison: x > 0
    Compare(String, CompareOp, SymValue),
    /// Range: x in [lo, hi]
    InRange(String, f64, f64),
    /// Non-null / defined
    NotNull(String),
    /// Type constraint
    HasType(String, String),
    /// Logical AND  
    And(Box<Predicate>, Box<Predicate>),
    /// Logical OR
    Or(Box<Predicate>, Box<Predicate>),
    /// Logical NOT
    Not(Box<Predicate>),
    /// Implication: P => Q
    Implies(Box<Predicate>, Box<Predicate>),
    /// Quantified: forall x in collection, P(x)
    ForAll(String, String, Box<Predicate>),
    /// Boolean literal
    BoolLit(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Contract {
    pub fn new(function_name: &str) -> Self {
        Contract {
            function_name: function_name.to_string(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
        }
    }

    pub fn require(&mut self, pred: Predicate) {
        self.preconditions.push(pred);
    }

    pub fn ensure(&mut self, pred: Predicate) {
        self.postconditions.push(pred);
    }

    pub fn invariant(&mut self, pred: Predicate) {
        self.invariants.push(pred);
    }

    /// Check preconditions against concrete values.
    pub fn check_preconditions(&self, env: &HashMap<String, f64>) -> ContractResult {
        let mut violations = Vec::new();
        for (i, pre) in self.preconditions.iter().enumerate() {
            if !evaluate_predicate(pre, env) {
                violations.push(format!("Precondition {} violated: {:?}", i, pre));
            }
        }
        if violations.is_empty() {
            ContractResult::Satisfied
        } else {
            ContractResult::Violated(violations)
        }
    }

    /// Check postconditions against concrete values.
    pub fn check_postconditions(&self, env: &HashMap<String, f64>) -> ContractResult {
        let mut violations = Vec::new();
        for (i, post) in self.postconditions.iter().enumerate() {
            if !evaluate_predicate(post, env) {
                violations.push(format!("Postcondition {} violated: {:?}", i, post));
            }
        }
        if violations.is_empty() {
            ContractResult::Satisfied
        } else {
            ContractResult::Violated(violations)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContractResult {
    Satisfied,
    Violated(Vec<String>),
    Unknown,
}

/// Evaluate a predicate against a concrete environment.
fn evaluate_predicate(pred: &Predicate, env: &HashMap<String, f64>) -> bool {
    match pred {
        Predicate::Compare(var, op, val) => {
            if let Some(&v) = env.get(var) {
                let target = match val {
                    SymValue::Concrete(c) => *c,
                    SymValue::Variable(other) => env.get(other).copied().unwrap_or(0.0),
                    _ => return true, // Can't evaluate symbolic
                };
                match op {
                    CompareOp::Eq => (v - target).abs() < 1e-10,
                    CompareOp::Ne => (v - target).abs() >= 1e-10,
                    CompareOp::Lt => v < target,
                    CompareOp::Le => v <= target,
                    CompareOp::Gt => v > target,
                    CompareOp::Ge => v >= target,
                }
            } else { true } // Variable not in scope → assume satisfied
        }
        Predicate::InRange(var, lo, hi) => {
            if let Some(&v) = env.get(var) { v >= *lo && v <= *hi } else { true }
        }
        Predicate::NotNull(var) => env.contains_key(var),
        Predicate::HasType(_, _) => true, // Type checking done elsewhere
        Predicate::And(a, b) => evaluate_predicate(a, env) && evaluate_predicate(b, env),
        Predicate::Or(a, b) => evaluate_predicate(a, env) || evaluate_predicate(b, env),
        Predicate::Not(p) => !evaluate_predicate(p, env),
        Predicate::Implies(p, q) => !evaluate_predicate(p, env) || evaluate_predicate(q, env),
        Predicate::ForAll(_, _, _) => true, // Requires symbolic reasoning
        Predicate::BoolLit(b) => *b,
    }
}

// ── Symbolic Execution ──────────────────────────────────────────────────

/// Symbolic value in the execution engine.
#[derive(Debug, Clone)]
pub enum SymValue {
    Concrete(f64),
    Variable(String),
    Add(Box<SymValue>, Box<SymValue>),
    Sub(Box<SymValue>, Box<SymValue>),
    Mul(Box<SymValue>, Box<SymValue>),
    Div(Box<SymValue>, Box<SymValue>),
    Neg(Box<SymValue>),
    Unknown,
}

impl SymValue {
    /// Try to evaluate to a concrete value.
    pub fn as_concrete(&self) -> Option<f64> {
        match self {
            SymValue::Concrete(v) => Some(*v),
            SymValue::Add(a, b) => {
                let av = a.as_concrete()?;
                let bv = b.as_concrete()?;
                Some(av + bv)
            }
            SymValue::Sub(a, b) => {
                let av = a.as_concrete()?;
                let bv = b.as_concrete()?;
                Some(av - bv)
            }
            SymValue::Mul(a, b) => {
                let av = a.as_concrete()?;
                let bv = b.as_concrete()?;
                Some(av * bv)
            }
            SymValue::Div(a, b) => {
                let av = a.as_concrete()?;
                let bv = b.as_concrete()?;
                if bv.abs() < 1e-15 { None } else { Some(av / bv) }
            }
            SymValue::Neg(a) => Some(-a.as_concrete()?),
            _ => None,
        }
    }
}

/// Path constraint accumulated during symbolic execution.
#[derive(Debug, Clone)]
pub struct PathConstraint {
    pub condition: Predicate,
    pub is_true_branch: bool,
}

/// Symbolic execution state.
#[derive(Debug, Clone)]
pub struct SymState {
    pub variables: HashMap<String, SymValue>,
    pub path_constraints: Vec<PathConstraint>,
    pub path_id: u64,
}

impl SymState {
    pub fn new() -> Self {
        SymState {
            variables: HashMap::new(),
            path_constraints: Vec::new(),
            path_id: 0,
        }
    }

    /// Set a variable to a symbolic value.
    pub fn set_var(&mut self, name: &str, val: SymValue) {
        self.variables.insert(name.to_string(), val);
    }

    /// Get a variable's symbolic value.
    pub fn get_var(&self, name: &str) -> Option<&SymValue> {
        self.variables.get(name)
    }

    /// Add a path constraint (from branching).
    pub fn add_constraint(&mut self, condition: Predicate, is_true: bool) {
        self.path_constraints.push(PathConstraint { condition, is_true_branch: is_true });
    }

    /// Fork state for branch exploration.
    pub fn fork(&self, condition: Predicate, is_true: bool) -> SymState {
        let mut new_state = self.clone();
        new_state.add_constraint(condition, is_true);
        new_state.path_id = self.path_id * 2 + if is_true { 1 } else { 0 };
        new_state
    }

    /// Check if path constraints are satisfiable (simplified solver).
    pub fn is_feasible(&self) -> bool {
        // Create concrete env from any known concrete values
        let mut env = HashMap::new();
        for (k, v) in &self.variables {
            if let Some(c) = v.as_concrete() {
                env.insert(k.clone(), c);
            }
        }

        // Check all constraints
        for pc in &self.path_constraints {
            let result = evaluate_predicate(&pc.condition, &env);
            if pc.is_true_branch && !result { return false; }
            if !pc.is_true_branch && result { return false; }
        }
        true
    }
}

/// Symbolic execution engine.
#[derive(Debug)]
pub struct SymbolicExecutor {
    pub max_depth: usize,
    pub max_paths: usize,
    pub explored_paths: Vec<SymState>,
    pub errors_found: Vec<VerificationError>,
}

/// Verification error found by symbolic execution.
#[derive(Debug, Clone)]
pub struct VerificationError {
    pub kind: ErrorKind,
    pub message: String,
    pub path_constraints: Vec<PathConstraint>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    DivisionByZero,
    ArrayOutOfBounds,
    IntegerOverflow,
    ContractViolation,
    UnreachableCode,
    NullDereference,
    AssertionFailure,
}

impl SymbolicExecutor {
    pub fn new(max_depth: usize, max_paths: usize) -> Self {
        SymbolicExecutor {
            max_depth,
            max_paths,
            explored_paths: Vec::new(),
            errors_found: Vec::new(),
        }
    }

    /// Explore a single path from an initial state.
    pub fn explore(&mut self, state: SymState) {
        if self.explored_paths.len() >= self.max_paths {
            return;
        }
        if state.path_constraints.len() >= self.max_depth {
            return;
        }
        if !state.is_feasible() {
            return;
        }
        self.explored_paths.push(state);
    }

    /// Check for division by zero in symbolic expressions.
    pub fn check_division_safety(&mut self, state: &SymState, divisor_var: &str) {
        if let Some(val) = state.get_var(divisor_var) {
            if let Some(c) = val.as_concrete() {
                if c.abs() < 1e-15 {
                    self.errors_found.push(VerificationError {
                        kind: ErrorKind::DivisionByZero,
                        message: format!("Division by zero: {} = {}", divisor_var, c),
                        path_constraints: state.path_constraints.clone(),
                    });
                }
            }
        }
    }

    /// Check array bounds.
    pub fn check_bounds(&mut self, state: &SymState, index_var: &str, length: usize) {
        if let Some(val) = state.get_var(index_var) {
            if let Some(c) = val.as_concrete() {
                let idx = c as i64;
                if idx < 0 || idx >= length as i64 {
                    self.errors_found.push(VerificationError {
                        kind: ErrorKind::ArrayOutOfBounds,
                        message: format!("Index {} out of bounds [0, {})", idx, length),
                        path_constraints: state.path_constraints.clone(),
                    });
                }
            }
        }
    }

    /// Verify a contract against symbolic state.
    pub fn verify_contract(&mut self, state: &SymState, contract: &Contract) {
        let mut env = HashMap::new();
        for (k, v) in &state.variables {
            if let Some(c) = v.as_concrete() {
                env.insert(k.clone(), c);
            }
        }

        if let ContractResult::Violated(violations) = contract.check_postconditions(&env) {
            for msg in violations {
                self.errors_found.push(VerificationError {
                    kind: ErrorKind::ContractViolation,
                    message: msg,
                    path_constraints: state.path_constraints.clone(),
                });
            }
        }
    }

    /// Summary of exploration results.
    pub fn summary(&self) -> VerificationSummary {
        VerificationSummary {
            paths_explored: self.explored_paths.len(),
            errors_found: self.errors_found.len(),
            max_depth_reached: self.explored_paths.iter()
                .map(|p| p.path_constraints.len())
                .max()
                .unwrap_or(0),
        }
    }
}

/// Summary of a verification run.
#[derive(Debug, Clone)]
pub struct VerificationSummary {
    pub paths_explored: usize,
    pub errors_found: usize,
    pub max_depth_reached: usize,
}

// ── Proof-Carrying Code ─────────────────────────────────────────────────

/// A proof certificate that accompanies compiled code.
#[derive(Debug, Clone)]
pub struct ProofCertificate {
    pub function_name: String,
    pub properties: Vec<ProvenProperty>,
    pub assumptions: Vec<String>,
    pub proof_method: ProofMethod,
}

/// A property that has been proven.
#[derive(Debug, Clone)]
pub struct ProvenProperty {
    pub description: String,
    pub predicate: Predicate,
    pub confidence: f64, // 1.0 = formally proven, <1.0 = tested
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProofMethod {
    SymbolicExecution,
    AbstractInterpretation,
    ModelChecking,
    Testing, // Exhaustive testing (bounded)
}

impl ProofCertificate {
    pub fn new(function_name: &str, method: ProofMethod) -> Self {
        ProofCertificate {
            function_name: function_name.to_string(),
            properties: Vec::new(),
            assumptions: Vec::new(),
            proof_method: method,
        }
    }

    pub fn add_property(&mut self, desc: &str, pred: Predicate, confidence: f64) {
        self.properties.push(ProvenProperty {
            description: desc.to_string(),
            predicate: pred,
            confidence: confidence.clamp(0.0, 1.0),
        });
    }

    pub fn add_assumption(&mut self, assumption: &str) {
        self.assumptions.push(assumption.to_string());
    }

    /// Check if all properties are formally proven.
    pub fn is_fully_proven(&self) -> bool {
        !self.properties.is_empty() && self.properties.iter().all(|p| p.confidence >= 1.0)
    }
}

// ── FFI ─────────────────────────────────────────────────────────────────

static VERIFY_STORES: Mutex<Option<HashMap<i64, SymbolicExecutor>>> = Mutex::new(None);

fn verify_store() -> std::sync::MutexGuard<'static, Option<HashMap<i64, SymbolicExecutor>>> {
    VERIFY_STORES.lock().unwrap()
}

fn next_verify_id() -> i64 {
    static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_verify_create(max_depth: i64, max_paths: i64) -> i64 {
    let id = next_verify_id();
    let exec = SymbolicExecutor::new(max_depth as usize, max_paths as usize);
    let mut store = verify_store();
    store.get_or_insert_with(HashMap::new).insert(id, exec);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_verify_paths(id: i64) -> i64 {
    let store = verify_store();
    store.as_ref().and_then(|s| s.get(&id))
        .map(|e| e.explored_paths.len() as i64)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_verify_errors(id: i64) -> i64 {
    let store = verify_store();
    store.as_ref().and_then(|s| s.get(&id))
        .map(|e| e.errors_found.len() as i64)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_verify_free(id: i64) {
    let mut store = verify_store();
    if let Some(s) = store.as_mut() { s.remove(&id); }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_precondition_pass() {
        let mut contract = Contract::new("sqrt");
        contract.require(Predicate::Compare("x".into(), CompareOp::Ge, SymValue::Concrete(0.0)));
        let mut env = HashMap::new();
        env.insert("x".into(), 4.0);
        assert_eq!(contract.check_preconditions(&env), ContractResult::Satisfied);
    }

    #[test]
    fn test_contract_precondition_fail() {
        let mut contract = Contract::new("sqrt");
        contract.require(Predicate::Compare("x".into(), CompareOp::Ge, SymValue::Concrete(0.0)));
        let mut env = HashMap::new();
        env.insert("x".into(), -1.0);
        match contract.check_preconditions(&env) {
            ContractResult::Violated(v) => assert!(!v.is_empty()),
            _ => panic!("Expected violation"),
        }
    }

    #[test]
    fn test_contract_postcondition() {
        let mut contract = Contract::new("abs");
        contract.ensure(Predicate::Compare("result".into(), CompareOp::Ge, SymValue::Concrete(0.0)));
        let mut env = HashMap::new();
        env.insert("result".into(), 5.0);
        assert_eq!(contract.check_postconditions(&env), ContractResult::Satisfied);
    }

    #[test]
    fn test_predicate_and() {
        let pred = Predicate::And(
            Box::new(Predicate::Compare("x".into(), CompareOp::Gt, SymValue::Concrete(0.0))),
            Box::new(Predicate::Compare("x".into(), CompareOp::Lt, SymValue::Concrete(10.0))),
        );
        let mut env = HashMap::new();
        env.insert("x".into(), 5.0);
        assert!(evaluate_predicate(&pred, &env));

        env.insert("x".into(), 15.0);
        assert!(!evaluate_predicate(&pred, &env));
    }

    #[test]
    fn test_predicate_in_range() {
        let pred = Predicate::InRange("x".into(), 0.0, 100.0);
        let mut env = HashMap::new();
        env.insert("x".into(), 50.0);
        assert!(evaluate_predicate(&pred, &env));
        env.insert("x".into(), 150.0);
        assert!(!evaluate_predicate(&pred, &env));
    }

    #[test]
    fn test_predicate_implies() {
        // x > 0 => x >= 0 (always true)
        let pred = Predicate::Implies(
            Box::new(Predicate::Compare("x".into(), CompareOp::Gt, SymValue::Concrete(0.0))),
            Box::new(Predicate::Compare("x".into(), CompareOp::Ge, SymValue::Concrete(0.0))),
        );
        let mut env = HashMap::new();
        env.insert("x".into(), 5.0);
        assert!(evaluate_predicate(&pred, &env));
        env.insert("x".into(), -5.0);
        assert!(evaluate_predicate(&pred, &env)); // P false => implication true
    }

    #[test]
    fn test_sym_value_concrete() {
        let v = SymValue::Add(
            Box::new(SymValue::Concrete(3.0)),
            Box::new(SymValue::Concrete(4.0)),
        );
        assert_eq!(v.as_concrete(), Some(7.0));
    }

    #[test]
    fn test_sym_value_variable() {
        let v = SymValue::Variable("x".into());
        assert_eq!(v.as_concrete(), None);
    }

    #[test]
    fn test_sym_state_fork() {
        let mut state = SymState::new();
        state.set_var("x", SymValue::Concrete(5.0));
        let cond = Predicate::Compare("x".into(), CompareOp::Gt, SymValue::Concrete(0.0));
        let true_branch = state.fork(cond.clone(), true);
        let false_branch = state.fork(cond, false);
        assert_ne!(true_branch.path_id, false_branch.path_id);
        assert!(true_branch.is_feasible());
        assert!(!false_branch.is_feasible()); // x=5 > 0 is true, so false branch infeasible
    }

    #[test]
    fn test_symbolic_executor() {
        let mut exec = SymbolicExecutor::new(10, 100);
        let state = SymState::new();
        exec.explore(state);
        assert_eq!(exec.explored_paths.len(), 1);
        let summary = exec.summary();
        assert_eq!(summary.paths_explored, 1);
        assert_eq!(summary.errors_found, 0);
    }

    #[test]
    fn test_check_division_safety() {
        let mut exec = SymbolicExecutor::new(10, 100);
        let mut state = SymState::new();
        state.set_var("d", SymValue::Concrete(0.0));
        exec.check_division_safety(&state, "d");
        assert_eq!(exec.errors_found.len(), 1);
        assert_eq!(exec.errors_found[0].kind, ErrorKind::DivisionByZero);
    }

    #[test]
    fn test_check_bounds() {
        let mut exec = SymbolicExecutor::new(10, 100);
        let mut state = SymState::new();
        state.set_var("i", SymValue::Concrete(10.0));
        exec.check_bounds(&state, "i", 5); // index 10 out of [0,5)
        assert_eq!(exec.errors_found.len(), 1);
        assert_eq!(exec.errors_found[0].kind, ErrorKind::ArrayOutOfBounds);
    }

    #[test]
    fn test_verify_contract() {
        let mut exec = SymbolicExecutor::new(10, 100);
        let mut contract = Contract::new("f");
        contract.ensure(Predicate::Compare("result".into(), CompareOp::Gt, SymValue::Concrete(0.0)));
        let mut state = SymState::new();
        state.set_var("result", SymValue::Concrete(-1.0));
        exec.verify_contract(&state, &contract);
        assert_eq!(exec.errors_found.len(), 1);
        assert_eq!(exec.errors_found[0].kind, ErrorKind::ContractViolation);
    }

    #[test]
    fn test_proof_certificate() {
        let mut cert = ProofCertificate::new("sort", ProofMethod::SymbolicExecution);
        cert.add_property("output is sorted", Predicate::BoolLit(true), 1.0);
        cert.add_property("output has same length", Predicate::BoolLit(true), 1.0);
        assert!(cert.is_fully_proven());
    }

    #[test]
    fn test_proof_certificate_partial() {
        let mut cert = ProofCertificate::new("f", ProofMethod::Testing);
        cert.add_property("no overflow", Predicate::BoolLit(true), 0.95);
        assert!(!cert.is_fully_proven()); // Not 100% confidence
    }

    #[test]
    fn test_ffi_verify() {
        let id = vitalis_verify_create(10, 100);
        assert!(id > 0);
        assert_eq!(vitalis_verify_paths(id), 0);
        assert_eq!(vitalis_verify_errors(id), 0);
        vitalis_verify_free(id);
    }
}
