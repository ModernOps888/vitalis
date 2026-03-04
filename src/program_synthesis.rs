//! Program Synthesis — Type-guided synthesis, CEGIS, and enumerative search.
//!
//! Provides program synthesis techniques: type-directed search, counter-example
//! guided inductive synthesis (CEGIS), enumerative bottom-up synthesis,
//! and sketch-based completion.

use std::collections::HashMap;

// ── Program Representation ──────────────────────────────────────────────

/// Simple expression language for synthesis.
#[derive(Debug, Clone, PartialEq)]
pub enum SynthExpr {
    Var(String),
    IntConst(i64),
    BoolConst(bool),
    Add(Box<SynthExpr>, Box<SynthExpr>),
    Sub(Box<SynthExpr>, Box<SynthExpr>),
    Mul(Box<SynthExpr>, Box<SynthExpr>),
    Div(Box<SynthExpr>, Box<SynthExpr>),
    Neg(Box<SynthExpr>),
    And(Box<SynthExpr>, Box<SynthExpr>),
    Or(Box<SynthExpr>, Box<SynthExpr>),
    Not(Box<SynthExpr>),
    Eq(Box<SynthExpr>, Box<SynthExpr>),
    Lt(Box<SynthExpr>, Box<SynthExpr>),
    Gt(Box<SynthExpr>, Box<SynthExpr>),
    If(Box<SynthExpr>, Box<SynthExpr>, Box<SynthExpr>),
    /// A hole to be filled during synthesis.
    Hole(SynthType),
}

/// Simple type system for synthesis.
#[derive(Debug, Clone, PartialEq)]
pub enum SynthType {
    Int,
    Bool,
}

/// Input-output example for synthesis.
#[derive(Debug, Clone)]
pub struct IOExample {
    pub inputs: HashMap<String, i64>,
    pub output: i64,
}

// ── Expression Evaluation ───────────────────────────────────────────────

/// Evaluate a synthesis expression with given variable bindings.
pub fn eval_expr(expr: &SynthExpr, env: &HashMap<String, i64>) -> Option<i64> {
    match expr {
        SynthExpr::Var(name) => env.get(name).copied(),
        SynthExpr::IntConst(n) => Some(*n),
        SynthExpr::BoolConst(b) => Some(if *b { 1 } else { 0 }),
        SynthExpr::Add(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(va.wrapping_add(vb))
        }
        SynthExpr::Sub(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(va.wrapping_sub(vb))
        }
        SynthExpr::Mul(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(va.wrapping_mul(vb))
        }
        SynthExpr::Div(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            if vb == 0 { None } else { Some(va / vb) }
        }
        SynthExpr::Neg(a) => {
            let va = eval_expr(a, env)?;
            Some(-va)
        }
        SynthExpr::And(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(if va != 0 && vb != 0 { 1 } else { 0 })
        }
        SynthExpr::Or(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(if va != 0 || vb != 0 { 1 } else { 0 })
        }
        SynthExpr::Not(a) => {
            let va = eval_expr(a, env)?;
            Some(if va == 0 { 1 } else { 0 })
        }
        SynthExpr::Eq(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(if va == vb { 1 } else { 0 })
        }
        SynthExpr::Lt(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(if va < vb { 1 } else { 0 })
        }
        SynthExpr::Gt(a, b) => {
            let va = eval_expr(a, env)?;
            let vb = eval_expr(b, env)?;
            Some(if va > vb { 1 } else { 0 })
        }
        SynthExpr::If(cond, then_e, else_e) => {
            let vc = eval_expr(cond, env)?;
            if vc != 0 { eval_expr(then_e, env) } else { eval_expr(else_e, env) }
        }
        SynthExpr::Hole(_) => None, // Holes cannot be evaluated
    }
}

/// Pretty-print a synthesis expression.
pub fn expr_to_string(expr: &SynthExpr) -> String {
    match expr {
        SynthExpr::Var(name) => name.clone(),
        SynthExpr::IntConst(n) => n.to_string(),
        SynthExpr::BoolConst(b) => b.to_string(),
        SynthExpr::Add(a, b) => format!("({} + {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Sub(a, b) => format!("({} - {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Mul(a, b) => format!("({} * {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Div(a, b) => format!("({} / {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Neg(a) => format!("(-{})", expr_to_string(a)),
        SynthExpr::And(a, b) => format!("({} && {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Or(a, b) => format!("({} || {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Not(a) => format!("(!{})", expr_to_string(a)),
        SynthExpr::Eq(a, b) => format!("({} == {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Lt(a, b) => format!("({} < {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::Gt(a, b) => format!("({} > {})", expr_to_string(a), expr_to_string(b)),
        SynthExpr::If(c, t, e) => format!("if {} {{ {} }} else {{ {} }}",
            expr_to_string(c), expr_to_string(t), expr_to_string(e)),
        SynthExpr::Hole(ty) => format!("??{:?}", ty),
    }
}

/// Expression complexity (AST node count).
pub fn expr_complexity(expr: &SynthExpr) -> usize {
    match expr {
        SynthExpr::Var(_) | SynthExpr::IntConst(_) | SynthExpr::BoolConst(_) | SynthExpr::Hole(_) => 1,
        SynthExpr::Neg(a) | SynthExpr::Not(a) => 1 + expr_complexity(a),
        SynthExpr::Add(a, b) | SynthExpr::Sub(a, b) | SynthExpr::Mul(a, b) |
        SynthExpr::Div(a, b) | SynthExpr::And(a, b) | SynthExpr::Or(a, b) |
        SynthExpr::Eq(a, b) | SynthExpr::Lt(a, b) | SynthExpr::Gt(a, b) =>
            1 + expr_complexity(a) + expr_complexity(b),
        SynthExpr::If(c, t, e) => 1 + expr_complexity(c) + expr_complexity(t) + expr_complexity(e),
    }
}

// ── Enumerative Synthesis (Bottom-Up) ───────────────────────────────────

/// Bottom-up enumerative synthesis. Enumerates expressions of increasing size.
pub fn enumerate_synthesis(
    examples: &[IOExample],
    variables: &[String],
    max_depth: usize,
    constants: &[i64],
) -> Option<SynthExpr> {
    // Level 0: variables and constants
    let mut candidates: Vec<SynthExpr> = Vec::new();
    for var in variables {
        candidates.push(SynthExpr::Var(var.clone()));
    }
    for &c in constants {
        candidates.push(SynthExpr::IntConst(c));
    }

    // Check level 0
    for expr in &candidates {
        if check_examples(expr, examples) {
            return Some(expr.clone());
        }
    }

    // Enumerate increasing depth
    for _depth in 1..=max_depth {
        let prev = candidates.clone();
        let mut new_candidates = Vec::new();

        // Unary ops
        for a in &prev {
            if expr_complexity(a) <= max_depth {
                new_candidates.push(SynthExpr::Neg(Box::new(a.clone())));
            }
        }

        // Binary ops
        for a in &prev {
            for b in &prev {
                if expr_complexity(a) + expr_complexity(b) + 1 <= max_depth + 1 {
                    new_candidates.push(SynthExpr::Add(Box::new(a.clone()), Box::new(b.clone())));
                    new_candidates.push(SynthExpr::Sub(Box::new(a.clone()), Box::new(b.clone())));
                    new_candidates.push(SynthExpr::Mul(Box::new(a.clone()), Box::new(b.clone())));
                }
            }
        }

        // Check new candidates
        for expr in &new_candidates {
            if check_examples(expr, examples) {
                return Some(expr.clone());
            }
        }

        candidates.extend(new_candidates);

        // Prune to keep search space manageable
        if candidates.len() > 10000 {
            candidates.truncate(5000);
        }
    }

    None
}

/// Check if an expression satisfies all IO examples.
pub fn check_examples(expr: &SynthExpr, examples: &[IOExample]) -> bool {
    for example in examples {
        match eval_expr(expr, &example.inputs) {
            Some(result) if result == example.output => {},
            _ => return false,
        }
    }
    true
}

// ── CEGIS (Counter-Example Guided Inductive Synthesis) ──────────────────

/// CEGIS loop: synthesize → verify → add counter-examples → repeat.
pub fn cegis_synthesis(
    specification: &dyn Fn(&HashMap<String, i64>, i64) -> bool,
    variables: &[String],
    verify_inputs: &[HashMap<String, i64>],
    max_iterations: usize,
    max_depth: usize,
    constants: &[i64],
) -> Option<SynthExpr> {
    let mut examples: Vec<IOExample> = Vec::new();

    for _iter in 0..max_iterations {
        // Phase 1: Synthesize a candidate from current examples
        let candidate = if examples.is_empty() {
            // Bootstrap: try simple expressions
            let mut found = None;
            for &c in constants {
                let expr = SynthExpr::IntConst(c);
                if verify_candidate(&expr, specification, verify_inputs).is_none() {
                    return Some(expr);
                }
                if found.is_none() {
                    found = Some(expr);
                }
            }
            found?
        } else {
            enumerate_synthesis(&examples, variables, max_depth, constants)?
        };

        // Phase 2: Verify against specification
        match verify_candidate(&candidate, specification, verify_inputs) {
            None => return Some(candidate), // All inputs pass!
            Some(counter_example) => {
                // Phase 3: Add counter-example
                // We need the correct output for this counter-example
                // Try to find it by brute force search
                for output in -100..=100 {
                    if specification(&counter_example, output) {
                        examples.push(IOExample {
                            inputs: counter_example,
                            output,
                        });
                        break;
                    }
                }
            }
        }
    }
    None
}

/// Verify a candidate expression against a specification.
/// Returns a counter-example (failing input) or None if the candidate is correct.
fn verify_candidate(
    expr: &SynthExpr,
    specification: &dyn Fn(&HashMap<String, i64>, i64) -> bool,
    verify_inputs: &[HashMap<String, i64>],
) -> Option<HashMap<String, i64>> {
    for inputs in verify_inputs {
        if let Some(result) = eval_expr(expr, inputs) {
            if !specification(inputs, result) {
                return Some(inputs.clone());
            }
        } else {
            return Some(inputs.clone()); // Expression failed to evaluate
        }
    }
    None
}

// ── Sketch-Based Synthesis ──────────────────────────────────────────────

/// Fill holes in a sketch expression.
pub fn fill_sketch(
    sketch: &SynthExpr,
    examples: &[IOExample],
    constants: &[i64],
    variables: &[String],
) -> Option<SynthExpr> {
    // Find holes and try to fill them
    if !has_holes(sketch) {
        if check_examples(sketch, examples) {
            return Some(sketch.clone());
        }
        return None;
    }

    // Generate fillers for the first hole
    let fillers = generate_fillers(constants, variables);
    for filler in fillers {
        let filled = substitute_first_hole(sketch, &filler);
        if let Some(result) = fill_sketch(&filled, examples, constants, variables) {
            return Some(result);
        }
    }
    None
}

/// Check if expression has any holes.
fn has_holes(expr: &SynthExpr) -> bool {
    match expr {
        SynthExpr::Hole(_) => true,
        SynthExpr::Var(_) | SynthExpr::IntConst(_) | SynthExpr::BoolConst(_) => false,
        SynthExpr::Neg(a) | SynthExpr::Not(a) => has_holes(a),
        SynthExpr::Add(a, b) | SynthExpr::Sub(a, b) | SynthExpr::Mul(a, b) |
        SynthExpr::Div(a, b) | SynthExpr::And(a, b) | SynthExpr::Or(a, b) |
        SynthExpr::Eq(a, b) | SynthExpr::Lt(a, b) | SynthExpr::Gt(a, b) =>
            has_holes(a) || has_holes(b),
        SynthExpr::If(c, t, e) => has_holes(c) || has_holes(t) || has_holes(e),
    }
}

/// Substitute the first hole found (DFS) with a filler.
fn substitute_first_hole(expr: &SynthExpr, filler: &SynthExpr) -> SynthExpr {
    match expr {
        SynthExpr::Hole(_) => filler.clone(),
        SynthExpr::Var(_) | SynthExpr::IntConst(_) | SynthExpr::BoolConst(_) => expr.clone(),
        SynthExpr::Neg(a) => {
            if has_holes(a) {
                SynthExpr::Neg(Box::new(substitute_first_hole(a, filler)))
            } else { expr.clone() }
        }
        SynthExpr::Not(a) => {
            if has_holes(a) {
                SynthExpr::Not(Box::new(substitute_first_hole(a, filler)))
            } else { expr.clone() }
        }
        SynthExpr::Add(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Add(x, y)),
        SynthExpr::Sub(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Sub(x, y)),
        SynthExpr::Mul(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Mul(x, y)),
        SynthExpr::Div(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Div(x, y)),
        SynthExpr::And(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::And(x, y)),
        SynthExpr::Or(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Or(x, y)),
        SynthExpr::Eq(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Eq(x, y)),
        SynthExpr::Lt(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Lt(x, y)),
        SynthExpr::Gt(a, b) => binary_substitute(a, b, filler, |x, y| SynthExpr::Gt(x, y)),
        SynthExpr::If(c, t, e) => {
            if has_holes(c) {
                SynthExpr::If(Box::new(substitute_first_hole(c, filler)), t.clone(), e.clone())
            } else if has_holes(t) {
                SynthExpr::If(c.clone(), Box::new(substitute_first_hole(t, filler)), e.clone())
            } else {
                SynthExpr::If(c.clone(), t.clone(), Box::new(substitute_first_hole(e, filler)))
            }
        }
    }
}

fn binary_substitute(
    a: &SynthExpr,
    b: &SynthExpr,
    filler: &SynthExpr,
    constructor: fn(Box<SynthExpr>, Box<SynthExpr>) -> SynthExpr,
) -> SynthExpr {
    if has_holes(a) {
        constructor(Box::new(substitute_first_hole(a, filler)), Box::new(b.clone()))
    } else {
        constructor(Box::new(a.clone()), Box::new(substitute_first_hole(b, filler)))
    }
}

/// Generate candidate fillers for holes.
fn generate_fillers(constants: &[i64], variables: &[String]) -> Vec<SynthExpr> {
    let mut fillers = Vec::new();
    for var in variables {
        fillers.push(SynthExpr::Var(var.clone()));
    }
    for &c in constants {
        fillers.push(SynthExpr::IntConst(c));
    }
    fillers
}

// ── FFI Interface ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_synth_eval(expr_type: i64, val_a: i64, val_b: i64) -> i64 {
    match expr_type {
        0 => val_a + val_b,   // Add
        1 => val_a - val_b,   // Sub
        2 => val_a * val_b,   // Mul
        3 => if val_b != 0 { val_a / val_b } else { 0 },  // Div
        4 => -val_a,          // Neg
        5 => if val_a == val_b { 1 } else { 0 },  // Eq
        6 => if val_a < val_b { 1 } else { 0 },   // Lt
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_synth_complexity(depth: i64, binary_ops: i64, unary_ops: i64) -> i64 {
    1 + binary_ops * 2 + unary_ops + depth
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_env(pairs: &[(&str, i64)]) -> HashMap<String, i64> {
        pairs.iter().map(|&(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn test_eval_const() {
        let expr = SynthExpr::IntConst(42);
        assert_eq!(eval_expr(&expr, &HashMap::new()), Some(42));
    }

    #[test]
    fn test_eval_var() {
        let expr = SynthExpr::Var("x".to_string());
        let env = make_env(&[("x", 10)]);
        assert_eq!(eval_expr(&expr, &env), Some(10));
    }

    #[test]
    fn test_eval_add() {
        let expr = SynthExpr::Add(
            Box::new(SynthExpr::Var("x".to_string())),
            Box::new(SynthExpr::IntConst(5)),
        );
        let env = make_env(&[("x", 3)]);
        assert_eq!(eval_expr(&expr, &env), Some(8));
    }

    #[test]
    fn test_eval_if() {
        let expr = SynthExpr::If(
            Box::new(SynthExpr::Gt(
                Box::new(SynthExpr::Var("x".to_string())),
                Box::new(SynthExpr::IntConst(0)),
            )),
            Box::new(SynthExpr::Var("x".to_string())),
            Box::new(SynthExpr::Neg(Box::new(SynthExpr::Var("x".to_string())))),
        );
        assert_eq!(eval_expr(&expr, &make_env(&[("x", 5)])), Some(5));
        assert_eq!(eval_expr(&expr, &make_env(&[("x", -3)])), Some(3));
    }

    #[test]
    fn test_expr_to_string() {
        let expr = SynthExpr::Add(
            Box::new(SynthExpr::Var("x".to_string())),
            Box::new(SynthExpr::IntConst(1)),
        );
        assert_eq!(expr_to_string(&expr), "(x + 1)");
    }

    #[test]
    fn test_expr_complexity() {
        let expr = SynthExpr::Add(
            Box::new(SynthExpr::Var("x".to_string())),
            Box::new(SynthExpr::IntConst(1)),
        );
        assert_eq!(expr_complexity(&expr), 3); // Add + Var + IntConst
    }

    #[test]
    fn test_enumerate_identity() {
        // Synthesize f(x) = x
        let examples = vec![
            IOExample { inputs: make_env(&[("x", 1)]), output: 1 },
            IOExample { inputs: make_env(&[("x", 5)]), output: 5 },
        ];
        let result = enumerate_synthesis(&examples, &["x".to_string()], 3, &[0, 1, 2]);
        assert!(result.is_some());
        let expr = result.unwrap();
        assert_eq!(expr_to_string(&expr), "x");
    }

    #[test]
    fn test_enumerate_add_const() {
        // Synthesize f(x) = x + 1
        let examples = vec![
            IOExample { inputs: make_env(&[("x", 0)]), output: 1 },
            IOExample { inputs: make_env(&[("x", 5)]), output: 6 },
            IOExample { inputs: make_env(&[("x", -3)]), output: -2 },
        ];
        let result = enumerate_synthesis(&examples, &["x".to_string()], 3, &[0, 1, 2]);
        assert!(result.is_some());
    }

    #[test]
    fn test_enumerate_double() {
        // Synthesize f(x) = x + x = 2x
        let examples = vec![
            IOExample { inputs: make_env(&[("x", 1)]), output: 2 },
            IOExample { inputs: make_env(&[("x", 3)]), output: 6 },
        ];
        let result = enumerate_synthesis(&examples, &["x".to_string()], 3, &[0, 1, 2]);
        assert!(result.is_some());
    }

    #[test]
    fn test_check_examples() {
        let expr = SynthExpr::Var("x".to_string());
        let examples = vec![
            IOExample { inputs: make_env(&[("x", 1)]), output: 1 },
            IOExample { inputs: make_env(&[("x", 2)]), output: 2 },
        ];
        assert!(check_examples(&expr, &examples));
    }

    #[test]
    fn test_fill_sketch() {
        // Sketch: ?? + 1, should fill with x
        let sketch = SynthExpr::Add(
            Box::new(SynthExpr::Hole(SynthType::Int)),
            Box::new(SynthExpr::IntConst(1)),
        );
        let examples = vec![
            IOExample { inputs: make_env(&[("x", 0)]), output: 1 },
            IOExample { inputs: make_env(&[("x", 5)]), output: 6 },
        ];
        let result = fill_sketch(&sketch, &examples, &[0, 1], &["x".to_string()]);
        assert!(result.is_some());
    }

    #[test]
    fn test_has_holes() {
        assert!(has_holes(&SynthExpr::Hole(SynthType::Int)));
        assert!(!has_holes(&SynthExpr::IntConst(5)));
        assert!(has_holes(&SynthExpr::Add(
            Box::new(SynthExpr::IntConst(1)),
            Box::new(SynthExpr::Hole(SynthType::Int)),
        )));
    }

    #[test]
    fn test_cegis_simple() {
        // Spec: f(x) == x * 2
        let spec = |inputs: &HashMap<String, i64>, output: i64| -> bool {
            let x = inputs.get("x").copied().unwrap_or(0);
            output == x * 2
        };
        let verify_inputs: Vec<HashMap<String, i64>> = (-5..=5)
            .map(|i| make_env(&[("x", i)]))
            .collect();
        let result = cegis_synthesis(
            &spec,
            &["x".to_string()],
            &verify_inputs,
            10,
            3,
            &[0, 1, 2],
        );
        // Should find x + x or x * 2
        if let Some(expr) = &result {
            for inputs in &verify_inputs {
                let val = eval_expr(expr, inputs).unwrap();
                assert!(spec(inputs, val));
            }
        }
    }

    #[test]
    fn test_eval_div_by_zero() {
        let expr = SynthExpr::Div(
            Box::new(SynthExpr::IntConst(10)),
            Box::new(SynthExpr::IntConst(0)),
        );
        assert_eq!(eval_expr(&expr, &HashMap::new()), None);
    }

    #[test]
    fn test_eval_boolean_ops() {
        let expr = SynthExpr::And(
            Box::new(SynthExpr::BoolConst(true)),
            Box::new(SynthExpr::BoolConst(false)),
        );
        assert_eq!(eval_expr(&expr, &HashMap::new()), Some(0));

        let expr = SynthExpr::Or(
            Box::new(SynthExpr::BoolConst(true)),
            Box::new(SynthExpr::BoolConst(false)),
        );
        assert_eq!(eval_expr(&expr, &HashMap::new()), Some(1));
    }

    #[test]
    fn test_ffi_synth_eval() {
        assert_eq!(vitalis_synth_eval(0, 3, 4), 7);  // Add
        assert_eq!(vitalis_synth_eval(1, 10, 3), 7);  // Sub
        assert_eq!(vitalis_synth_eval(2, 3, 4), 12);  // Mul
        assert_eq!(vitalis_synth_eval(3, 10, 3), 3);  // Div
        assert_eq!(vitalis_synth_eval(3, 10, 0), 0);  // Div by zero
    }
}
