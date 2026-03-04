//! Dependent type system for Vitalis.
//!
//! Implements a dependent type theory core:
//! - **Pi types**: Dependent function types (∀x:A. B(x))
//! - **Sigma types**: Dependent pair types (Σx:A. B(x))
//! - **Type-level computation**: Terms at the type level, reduction
//! - **Propositional equality**: Identity type with refl, J eliminator
//! - **Indexed types**: Vectors, finite sets indexed by natural numbers
//! - **Universe hierarchy**: Type : Type₁ : Type₂ : …

use std::collections::HashMap;

// ── Terms / Types (unified in dependent type theory) ────────────────

/// A term in the dependent type theory.
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// Variable by de Bruijn index.
    Var(usize),
    /// Named variable (for pretty-printing).
    Named(String),
    /// Universe level: Type_n.
    Universe(u32),
    /// Pi type: (x : A) → B.
    Pi(String, Box<Term>, Box<Term>),
    /// Sigma type: (x : A) × B.
    Sigma(String, Box<Term>, Box<Term>),
    /// Lambda abstraction: λ(x : A). body.
    Lambda(String, Box<Term>, Box<Term>),
    /// Application: f a.
    App(Box<Term>, Box<Term>),
    /// Pair: (a, b).
    Pair(Box<Term>, Box<Term>),
    /// First projection: fst p.
    Fst(Box<Term>),
    /// Second projection: snd p.
    Snd(Box<Term>),
    /// Natural number type.
    Nat,
    /// Zero.
    Zero,
    /// Successor: S n.
    Succ(Box<Term>),
    /// Natural number eliminator: natrec(P, z, s, n).
    NatRec(Box<Term>, Box<Term>, Box<Term>, Box<Term>),
    /// Identity type: Id A a b.
    Id(Box<Term>, Box<Term>, Box<Term>),
    /// Reflexivity: refl a.
    Refl(Box<Term>),
    /// J eliminator: J(A, C, d, a, b, p).
    JElim {
        ty: Box<Term>,
        motive: Box<Term>,
        refl_case: Box<Term>,
        lhs: Box<Term>,
        rhs: Box<Term>,
        proof: Box<Term>,
    },
    /// Boolean type.
    Bool,
    /// True.
    True,
    /// False.
    False,
    /// Boolean eliminator: if b then t else e.
    BoolRec(Box<Term>, Box<Term>, Box<Term>),
    /// Vector type: Vec A n.
    Vec(Box<Term>, Box<Term>),
    /// Empty vector: nil A.
    VNil(Box<Term>),
    /// Cons: cons A n x xs.
    VCons(Box<Term>, Box<Term>, Box<Term>, Box<Term>),
    /// Finite set: Fin n.
    Fin(Box<Term>),
    /// Finite zero: fzero.
    FZero(Box<Term>),
    /// Finite successor: fsucc.
    FSucc(Box<Term>, Box<Term>),
    /// Annotated term: (t : T).
    Ann(Box<Term>, Box<Term>),
    /// Let binding: let x = e in body.
    Let(String, Box<Term>, Box<Term>),
}

impl Term {
    /// Make a natural number literal.
    pub fn nat_lit(n: u32) -> Self {
        let mut t = Term::Zero;
        for _ in 0..n {
            t = Term::Succ(Box::new(t));
        }
        t
    }

    /// Make a simple (non-dependent) function type: A → B.
    pub fn arrow(a: Term, b: Term) -> Self {
        Term::Pi("_".to_string(), Box::new(a), Box::new(b))
    }

    /// Check if this is a type (Universe).
    pub fn is_type(&self) -> bool {
        matches!(self, Term::Universe(_))
    }

    /// Check if this is a value (WHNF).
    pub fn is_value(&self) -> bool {
        matches!(self,
            Term::Lambda(..) | Term::Pair(..) | Term::Zero | Term::Succ(_) |
            Term::Refl(_) | Term::Universe(_) | Term::Pi(..) | Term::Sigma(..) |
            Term::Nat | Term::Bool | Term::True | Term::False |
            Term::VNil(_) | Term::VCons(..) | Term::FZero(_) | Term::FSucc(..) |
            Term::Id(..) | Term::Vec(..) | Term::Fin(_)
        )
    }
}

// ── Substitution ────────────────────────────────────────────────────

/// Substitute term `s` for variable index `idx` in term `t`.
pub fn substitute(t: &Term, idx: usize, s: &Term) -> Term {
    match t {
        Term::Var(i) => {
            if *i == idx { s.clone() }
            else if *i > idx { Term::Var(*i - 1) }
            else { Term::Var(*i) }
        }
        Term::Named(n) => Term::Named(n.clone()),
        Term::Universe(l) => Term::Universe(*l),
        Term::Pi(x, a, b) => Term::Pi(
            x.clone(),
            Box::new(substitute(a, idx, s)),
            Box::new(substitute(b, idx + 1, s)),
        ),
        Term::Sigma(x, a, b) => Term::Sigma(
            x.clone(),
            Box::new(substitute(a, idx, s)),
            Box::new(substitute(b, idx + 1, s)),
        ),
        Term::Lambda(x, ty, body) => Term::Lambda(
            x.clone(),
            Box::new(substitute(ty, idx, s)),
            Box::new(substitute(body, idx + 1, s)),
        ),
        Term::App(f, a) => Term::App(
            Box::new(substitute(f, idx, s)),
            Box::new(substitute(a, idx, s)),
        ),
        Term::Pair(a, b) => Term::Pair(
            Box::new(substitute(a, idx, s)),
            Box::new(substitute(b, idx, s)),
        ),
        Term::Fst(p) => Term::Fst(Box::new(substitute(p, idx, s))),
        Term::Snd(p) => Term::Snd(Box::new(substitute(p, idx, s))),
        Term::Nat => Term::Nat,
        Term::Zero => Term::Zero,
        Term::Succ(n) => Term::Succ(Box::new(substitute(n, idx, s))),
        Term::NatRec(p, z, step, n) => Term::NatRec(
            Box::new(substitute(p, idx, s)),
            Box::new(substitute(z, idx, s)),
            Box::new(substitute(step, idx, s)),
            Box::new(substitute(n, idx, s)),
        ),
        Term::Id(a, x, y) => Term::Id(
            Box::new(substitute(a, idx, s)),
            Box::new(substitute(x, idx, s)),
            Box::new(substitute(y, idx, s)),
        ),
        Term::Refl(a) => Term::Refl(Box::new(substitute(a, idx, s))),
        Term::JElim { ty, motive, refl_case, lhs, rhs, proof } => Term::JElim {
            ty: Box::new(substitute(ty, idx, s)),
            motive: Box::new(substitute(motive, idx, s)),
            refl_case: Box::new(substitute(refl_case, idx, s)),
            lhs: Box::new(substitute(lhs, idx, s)),
            rhs: Box::new(substitute(rhs, idx, s)),
            proof: Box::new(substitute(proof, idx, s)),
        },
        Term::Bool => Term::Bool,
        Term::True => Term::True,
        Term::False => Term::False,
        Term::BoolRec(b, t_case, f_case) => Term::BoolRec(
            Box::new(substitute(b, idx, s)),
            Box::new(substitute(t_case, idx, s)),
            Box::new(substitute(f_case, idx, s)),
        ),
        Term::Vec(a, n) => Term::Vec(
            Box::new(substitute(a, idx, s)),
            Box::new(substitute(n, idx, s)),
        ),
        Term::VNil(a) => Term::VNil(Box::new(substitute(a, idx, s))),
        Term::VCons(a, n, x, xs) => Term::VCons(
            Box::new(substitute(a, idx, s)),
            Box::new(substitute(n, idx, s)),
            Box::new(substitute(x, idx, s)),
            Box::new(substitute(xs, idx, s)),
        ),
        Term::Fin(n) => Term::Fin(Box::new(substitute(n, idx, s))),
        Term::FZero(n) => Term::FZero(Box::new(substitute(n, idx, s))),
        Term::FSucc(n, f) => Term::FSucc(
            Box::new(substitute(n, idx, s)),
            Box::new(substitute(f, idx, s)),
        ),
        Term::Ann(t_inner, ty) => Term::Ann(
            Box::new(substitute(t_inner, idx, s)),
            Box::new(substitute(ty, idx, s)),
        ),
        Term::Let(x, e, body) => Term::Let(
            x.clone(),
            Box::new(substitute(e, idx, s)),
            Box::new(substitute(body, idx + 1, s)),
        ),
    }
}

// ── Reduction (WHNF) ───────────────────────────────────────────────

/// Reduce a term to weak head normal form.
pub fn whnf(t: &Term) -> Term {
    match t {
        Term::App(f, a) => {
            let f_whnf = whnf(f);
            match f_whnf {
                Term::Lambda(_, _, body) => {
                    let result = substitute(&body, 0, a);
                    whnf(&result)
                }
                _ => Term::App(Box::new(f_whnf), a.clone()),
            }
        }
        Term::Fst(p) => {
            let p_whnf = whnf(p);
            match p_whnf {
                Term::Pair(a, _) => whnf(&a),
                _ => Term::Fst(Box::new(p_whnf)),
            }
        }
        Term::Snd(p) => {
            let p_whnf = whnf(p);
            match p_whnf {
                Term::Pair(_, b) => whnf(&b),
                _ => Term::Snd(Box::new(p_whnf)),
            }
        }
        Term::NatRec(motive, z, step, n) => {
            let n_whnf = whnf(n);
            match n_whnf {
                Term::Zero => whnf(z),
                Term::Succ(pred) => {
                    let rec = Term::NatRec(motive.clone(), z.clone(), step.clone(), pred.clone());
                    let result = Term::App(
                        Box::new(Term::App(step.clone(), pred)),
                        Box::new(rec),
                    );
                    whnf(&result)
                }
                _ => Term::NatRec(motive.clone(), z.clone(), step.clone(), Box::new(n_whnf)),
            }
        }
        Term::BoolRec(b, t_case, f_case) => {
            let b_whnf = whnf(b);
            match b_whnf {
                Term::True => whnf(t_case),
                Term::False => whnf(f_case),
                _ => Term::BoolRec(Box::new(b_whnf), t_case.clone(), f_case.clone()),
            }
        }
        Term::JElim { proof, refl_case, .. } => {
            let p_whnf = whnf(proof);
            match p_whnf {
                Term::Refl(_) => whnf(refl_case),
                _ => t.clone(),
            }
        }
        Term::Let(_, e, body) => {
            let result = substitute(body, 0, e);
            whnf(&result)
        }
        _ => t.clone(),
    }
}

/// Full normalization (reduce under binders).
pub fn normalize(t: &Term) -> Term {
    let w = whnf(t);
    match w {
        Term::Lambda(x, ty, body) => Term::Lambda(
            x,
            Box::new(normalize(&ty)),
            Box::new(normalize(&body)),
        ),
        Term::Pi(x, a, b) => Term::Pi(
            x,
            Box::new(normalize(&a)),
            Box::new(normalize(&b)),
        ),
        Term::Sigma(x, a, b) => Term::Sigma(
            x,
            Box::new(normalize(&a)),
            Box::new(normalize(&b)),
        ),
        Term::App(f, a) => Term::App(
            Box::new(normalize(&f)),
            Box::new(normalize(&a)),
        ),
        Term::Pair(a, b) => Term::Pair(
            Box::new(normalize(&a)),
            Box::new(normalize(&b)),
        ),
        Term::Succ(n) => Term::Succ(Box::new(normalize(&n))),
        other => other,
    }
}

// ── Definitional Equality ───────────────────────────────────────────

/// Check if two terms are definitionally equal.
pub fn definitional_eq(a: &Term, b: &Term) -> bool {
    let a_nf = normalize(a);
    let b_nf = normalize(b);
    alpha_eq(&a_nf, &b_nf)
}

/// Alpha-equivalence (structural equality up to bound variable names).
pub fn alpha_eq(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (Term::Var(i), Term::Var(j)) => i == j,
        (Term::Named(x), Term::Named(y)) => x == y,
        (Term::Universe(i), Term::Universe(j)) => i == j,
        (Term::Pi(_, a1, b1), Term::Pi(_, a2, b2)) => alpha_eq(a1, a2) && alpha_eq(b1, b2),
        (Term::Sigma(_, a1, b1), Term::Sigma(_, a2, b2)) => alpha_eq(a1, a2) && alpha_eq(b1, b2),
        (Term::Lambda(_, t1, b1), Term::Lambda(_, t2, b2)) => alpha_eq(t1, t2) && alpha_eq(b1, b2),
        (Term::App(f1, a1), Term::App(f2, a2)) => alpha_eq(f1, f2) && alpha_eq(a1, a2),
        (Term::Pair(a1, b1), Term::Pair(a2, b2)) => alpha_eq(a1, a2) && alpha_eq(b1, b2),
        (Term::Fst(p1), Term::Fst(p2)) => alpha_eq(p1, p2),
        (Term::Snd(p1), Term::Snd(p2)) => alpha_eq(p1, p2),
        (Term::Nat, Term::Nat) => true,
        (Term::Zero, Term::Zero) => true,
        (Term::Succ(n1), Term::Succ(n2)) => alpha_eq(n1, n2),
        (Term::Bool, Term::Bool) => true,
        (Term::True, Term::True) => true,
        (Term::False, Term::False) => true,
        (Term::Id(a1, x1, y1), Term::Id(a2, x2, y2)) =>
            alpha_eq(a1, a2) && alpha_eq(x1, x2) && alpha_eq(y1, y2),
        (Term::Refl(a1), Term::Refl(a2)) => alpha_eq(a1, a2),
        _ => false,
    }
}

// ── Type Checking Context ───────────────────────────────────────────

/// Typing context: maps variable indices to their types.
#[derive(Debug, Clone)]
pub struct Context {
    pub entries: Vec<(String, Term)>,
    pub definitions: HashMap<String, Term>,
}

impl Context {
    pub fn empty() -> Self {
        Self { entries: Vec::new(), definitions: HashMap::new() }
    }

    pub fn extend(&self, name: &str, ty: Term) -> Context {
        let mut new = self.clone();
        new.entries.push((name.to_string(), ty));
        new
    }

    pub fn lookup(&self, idx: usize) -> Option<&Term> {
        if idx < self.entries.len() {
            Some(&self.entries[self.entries.len() - 1 - idx].1)
        } else {
            None
        }
    }

    pub fn define(&mut self, name: &str, value: Term) {
        self.definitions.insert(name.to_string(), value);
    }
}

// ── Type Checker ────────────────────────────────────────────────────

/// Type checking / inference errors.
#[derive(Debug, Clone)]
pub enum TypeError {
    Mismatch { expected: Term, got: Term },
    NotAFunction(Term),
    NotAPair(Term),
    UnboundVariable(usize),
    NotAType(Term),
    UniverseInconsistency(u32, u32),
    Other(String),
}

/// Infer the type of a term in a context.
pub fn infer(ctx: &Context, term: &Term) -> Result<Term, TypeError> {
    match term {
        Term::Var(idx) => {
            ctx.lookup(*idx).cloned().ok_or(TypeError::UnboundVariable(*idx))
        }
        Term::Universe(n) => Ok(Term::Universe(n + 1)),
        Term::Pi(x, a, b) => {
            let _ = infer(ctx, a)?;
            let ctx2 = ctx.extend(x, *a.clone());
            let _ = infer(&ctx2, b)?;
            Ok(Term::Universe(0)) // simplified
        }
        Term::Sigma(x, a, b) => {
            let _ = infer(ctx, a)?;
            let ctx2 = ctx.extend(x, *a.clone());
            let _ = infer(&ctx2, b)?;
            Ok(Term::Universe(0))
        }
        Term::Lambda(x, ann_ty, body) => {
            let _ = infer(ctx, ann_ty)?;
            let ctx2 = ctx.extend(x, *ann_ty.clone());
            let body_ty = infer(&ctx2, body)?;
            Ok(Term::Pi(x.clone(), ann_ty.clone(), Box::new(body_ty)))
        }
        Term::App(f, a) => {
            let f_ty = infer(ctx, f)?;
            let f_ty = whnf(&f_ty);
            match f_ty {
                Term::Pi(_, param_ty, ret_ty) => {
                    check(ctx, a, &param_ty)?;
                    Ok(substitute(&ret_ty, 0, a))
                }
                other => Err(TypeError::NotAFunction(other)),
            }
        }
        Term::Pair(a, b) => {
            let a_ty = infer(ctx, a)?;
            let b_ty = infer(ctx, b)?;
            Ok(Term::Sigma("_".into(), Box::new(a_ty), Box::new(b_ty)))
        }
        Term::Fst(p) => {
            let p_ty = infer(ctx, p)?;
            let p_ty = whnf(&p_ty);
            match p_ty {
                Term::Sigma(_, a, _) => Ok(*a),
                other => Err(TypeError::NotAPair(other)),
            }
        }
        Term::Snd(p) => {
            let p_ty = infer(ctx, p)?;
            let p_ty = whnf(&p_ty);
            match p_ty {
                Term::Sigma(_, _, b) => {
                    let fst = Term::Fst(p.clone());
                    Ok(substitute(&b, 0, &fst))
                }
                other => Err(TypeError::NotAPair(other)),
            }
        }
        Term::Nat => Ok(Term::Universe(0)),
        Term::Zero => Ok(Term::Nat),
        Term::Succ(n) => {
            check(ctx, n, &Term::Nat)?;
            Ok(Term::Nat)
        }
        Term::Bool => Ok(Term::Universe(0)),
        Term::True | Term::False => Ok(Term::Bool),
        Term::BoolRec(b, t, f) => {
            check(ctx, b, &Term::Bool)?;
            let t_ty = infer(ctx, t)?;
            check(ctx, f, &t_ty)?;
            Ok(t_ty)
        }
        Term::Id(a, x, y) => {
            let _ = infer(ctx, a)?;
            check(ctx, x, a)?;
            check(ctx, y, a)?;
            Ok(Term::Universe(0))
        }
        Term::Refl(a) => {
            let ty = infer(ctx, a)?;
            Ok(Term::Id(Box::new(ty), a.clone(), a.clone()))
        }
        Term::Ann(t, ty) => {
            check(ctx, t, ty)?;
            Ok(*ty.clone())
        }
        Term::Let(x, e, body) => {
            let e_ty = infer(ctx, e)?;
            let ctx2 = ctx.extend(x, e_ty);
            let body_ty = infer(&ctx2, body)?;
            Ok(substitute(&body_ty, 0, e))
        }
        _ => Err(TypeError::Other(format!("cannot infer type for {:?}", term))),
    }
}

/// Check that a term has a given type.
pub fn check(ctx: &Context, term: &Term, expected: &Term) -> Result<(), TypeError> {
    let inferred = infer(ctx, term)?;
    let inferred_nf = normalize(&inferred);
    let expected_nf = normalize(expected);
    if alpha_eq(&inferred_nf, &expected_nf) {
        Ok(())
    } else {
        Err(TypeError::Mismatch { expected: expected_nf, got: inferred_nf })
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nat_literal() {
        let three = Term::nat_lit(3);
        assert_eq!(
            three,
            Term::Succ(Box::new(Term::Succ(Box::new(Term::Succ(Box::new(Term::Zero))))))
        );
    }

    #[test]
    fn test_arrow_type() {
        let arr = Term::arrow(Term::Nat, Term::Nat);
        match arr {
            Term::Pi(_, a, b) => {
                assert_eq!(*a, Term::Nat);
                assert_eq!(*b, Term::Nat);
            }
            _ => panic!("expected Pi"),
        }
    }

    #[test]
    fn test_is_value() {
        assert!(Term::Zero.is_value());
        assert!(Term::Nat.is_value());
        assert!(Term::Universe(0).is_value());
        assert!(!Term::Var(0).is_value());
    }

    #[test]
    fn test_substitute_var() {
        let t = Term::Var(0);
        let result = substitute(&t, 0, &Term::Zero);
        assert_eq!(result, Term::Zero);
    }

    #[test]
    fn test_whnf_beta_reduction() {
        // (λx:Nat. x) 0 → 0
        let id = Term::Lambda("x".into(), Box::new(Term::Nat), Box::new(Term::Var(0)));
        let app = Term::App(Box::new(id), Box::new(Term::Zero));
        let result = whnf(&app);
        assert_eq!(result, Term::Zero);
    }

    #[test]
    fn test_whnf_fst() {
        let pair = Term::Pair(Box::new(Term::Zero), Box::new(Term::True));
        let fst = Term::Fst(Box::new(pair));
        assert_eq!(whnf(&fst), Term::Zero);
    }

    #[test]
    fn test_whnf_snd() {
        let pair = Term::Pair(Box::new(Term::Zero), Box::new(Term::True));
        let snd = Term::Snd(Box::new(pair));
        assert_eq!(whnf(&snd), Term::True);
    }

    #[test]
    fn test_whnf_bool_rec_true() {
        let t = Term::BoolRec(Box::new(Term::True), Box::new(Term::Zero), Box::new(Term::nat_lit(1)));
        assert_eq!(whnf(&t), Term::Zero);
    }

    #[test]
    fn test_whnf_bool_rec_false() {
        let t = Term::BoolRec(Box::new(Term::False), Box::new(Term::Zero), Box::new(Term::nat_lit(1)));
        assert_eq!(whnf(&t), Term::nat_lit(1));
    }

    #[test]
    fn test_definitional_eq() {
        let a = Term::nat_lit(3);
        let b = Term::Succ(Box::new(Term::Succ(Box::new(Term::Succ(Box::new(Term::Zero))))));
        assert!(definitional_eq(&a, &b));
    }

    #[test]
    fn test_alpha_eq() {
        let a = Term::Lambda("x".into(), Box::new(Term::Nat), Box::new(Term::Var(0)));
        let b = Term::Lambda("y".into(), Box::new(Term::Nat), Box::new(Term::Var(0)));
        assert!(alpha_eq(&a, &b));
    }

    #[test]
    fn test_infer_zero() {
        let ctx = Context::empty();
        let ty = infer(&ctx, &Term::Zero).unwrap();
        assert_eq!(ty, Term::Nat);
    }

    #[test]
    fn test_infer_succ() {
        let ctx = Context::empty();
        let ty = infer(&ctx, &Term::Succ(Box::new(Term::Zero))).unwrap();
        assert_eq!(ty, Term::Nat);
    }

    #[test]
    fn test_infer_true() {
        let ctx = Context::empty();
        let ty = infer(&ctx, &Term::True).unwrap();
        assert_eq!(ty, Term::Bool);
    }

    #[test]
    fn test_infer_universe() {
        let ctx = Context::empty();
        let ty = infer(&ctx, &Term::Universe(0)).unwrap();
        assert_eq!(ty, Term::Universe(1));
    }

    #[test]
    fn test_infer_nat_type() {
        let ctx = Context::empty();
        let ty = infer(&ctx, &Term::Nat).unwrap();
        assert_eq!(ty, Term::Universe(0));
    }

    #[test]
    fn test_infer_identity() {
        let ctx = Context::empty();
        // λ(x : Nat). x
        let id = Term::Lambda("x".into(), Box::new(Term::Nat), Box::new(Term::Var(0)));
        let ty = infer(&ctx, &id).unwrap();
        match ty {
            Term::Pi(_, a, b) => {
                assert_eq!(*a, Term::Nat);
                assert_eq!(*b, Term::Nat);
            }
            _ => panic!("expected Pi type"),
        }
    }

    #[test]
    fn test_infer_application() {
        let ctx = Context::empty();
        let id = Term::Lambda("x".into(), Box::new(Term::Nat), Box::new(Term::Var(0)));
        let app = Term::App(Box::new(id), Box::new(Term::Zero));
        let ty = infer(&ctx, &app).unwrap();
        assert_eq!(ty, Term::Nat);
    }

    #[test]
    fn test_infer_pair() {
        let ctx = Context::empty();
        let pair = Term::Pair(Box::new(Term::Zero), Box::new(Term::True));
        let ty = infer(&ctx, &pair).unwrap();
        match ty {
            Term::Sigma(_, a, b) => {
                assert_eq!(*a, Term::Nat);
                assert_eq!(*b, Term::Bool);
            }
            _ => panic!("expected Sigma"),
        }
    }

    #[test]
    fn test_infer_refl() {
        let ctx = Context::empty();
        let refl = Term::Refl(Box::new(Term::Zero));
        let ty = infer(&ctx, &refl).unwrap();
        match ty {
            Term::Id(a, x, y) => {
                assert_eq!(*a, Term::Nat);
                assert_eq!(*x, Term::Zero);
                assert_eq!(*y, Term::Zero);
            }
            _ => panic!("expected Id type"),
        }
    }

    #[test]
    fn test_check_nat() {
        let ctx = Context::empty();
        assert!(check(&ctx, &Term::Zero, &Term::Nat).is_ok());
        assert!(check(&ctx, &Term::True, &Term::Nat).is_err());
    }

    #[test]
    fn test_context_extend_lookup() {
        let ctx = Context::empty();
        let ctx2 = ctx.extend("x", Term::Nat);
        assert_eq!(ctx2.lookup(0), Some(&Term::Nat));
    }

    #[test]
    fn test_j_eliminator_refl() {
        let j = Term::JElim {
            ty: Box::new(Term::Nat),
            motive: Box::new(Term::Nat), // simplified
            refl_case: Box::new(Term::Zero),
            lhs: Box::new(Term::Zero),
            rhs: Box::new(Term::Zero),
            proof: Box::new(Term::Refl(Box::new(Term::Zero))),
        };
        let result = whnf(&j);
        assert_eq!(result, Term::Zero);
    }

    #[test]
    fn test_let_binding() {
        // let x = 0 in succ x → succ 0
        let t = Term::Let(
            "x".into(),
            Box::new(Term::Zero),
            Box::new(Term::Succ(Box::new(Term::Var(0)))),
        );
        let result = whnf(&t);
        assert_eq!(result, Term::Succ(Box::new(Term::Zero)));
    }

    #[test]
    fn test_normalize_nested() {
        // (λx.x) ((λy.y) 0) → 0
        let inner_id = Term::Lambda("y".into(), Box::new(Term::Nat), Box::new(Term::Var(0)));
        let inner_app = Term::App(Box::new(inner_id), Box::new(Term::Zero));
        let outer_id = Term::Lambda("x".into(), Box::new(Term::Nat), Box::new(Term::Var(0)));
        let full = Term::App(Box::new(outer_id), Box::new(inner_app));
        let result = normalize(&full);
        assert_eq!(result, Term::Zero);
    }

    #[test]
    fn test_pi_type_inference() {
        let ctx = Context::empty();
        let pi = Term::Pi("x".into(), Box::new(Term::Nat), Box::new(Term::Nat));
        let ty = infer(&ctx, &pi).unwrap();
        assert_eq!(ty, Term::Universe(0));
    }

    #[test]
    fn test_sigma_type_inference() {
        let ctx = Context::empty();
        let sigma = Term::Sigma("x".into(), Box::new(Term::Nat), Box::new(Term::Bool));
        let ty = infer(&ctx, &sigma).unwrap();
        assert_eq!(ty, Term::Universe(0));
    }

    #[test]
    fn test_ann_inference() {
        let ctx = Context::empty();
        let ann = Term::Ann(Box::new(Term::Zero), Box::new(Term::Nat));
        let ty = infer(&ctx, &ann).unwrap();
        assert_eq!(ty, Term::Nat);
    }

    #[test]
    fn test_unbound_variable_error() {
        let ctx = Context::empty();
        let result = infer(&ctx, &Term::Var(0));
        assert!(result.is_err());
    }
}
