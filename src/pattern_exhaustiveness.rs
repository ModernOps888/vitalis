//! Vitalis Pattern Exhaustiveness Checker (v24)
//!
//! Provides exhaustiveness checking for match expressions, detecting:
//! - Non-exhaustive matches (missing patterns)
//! - Redundant/unreachable match arms
//! - Or-pattern support (`A | B`)
//! - Guard clause handling (guarded arms are treated as potentially non-exhaustive)
//! - Nested destructuring for structs, enums, and tuples
//!
//! # Algorithm
//!
//! Uses a matrix-based usefulness algorithm inspired by Rust's pattern matching
//! analysis. The key operations are:
//!
//! - **Usefulness**: A pattern is useful if there exists a value matched by it
//!   that isn't matched by any previous pattern.
//! - **Exhaustiveness**: A matrix of patterns is exhaustive if no value is
//!   left unmatched.
//! - **Specialize**: Given a constructor, produce a sub-matrix for values
//!   starting with that constructor.
//!
//! # References
//!
//! Based on "Warnings for pattern matching" by Luc Maranget (JFP 2007).

use std::collections::{HashMap, HashSet, BTreeSet};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Type Descriptors (for pattern analysis)
// ═══════════════════════════════════════════════════════════════════════

/// Describes the shape of a type for exhaustiveness analysis.
///
/// We don't use the full Vitalis type system here — just enough information
/// to know what constructors exist for each type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeDesc {
    /// Boolean — constructors: true, false
    Bool,
    /// Integer — infinite constructors (never fully exhaustive without wildcard)
    Int,
    /// Float — infinite constructors
    Float,
    /// String — infinite constructors
    Str,
    /// Enum with known variants: name → list of variant (name, arity)
    Enum {
        name: String,
        variants: Vec<(String, usize)>,
    },
    /// Struct with known fields
    Struct {
        name: String,
        fields: Vec<String>,
    },
    /// Option type — constructors: Some(T), None
    Option,
    /// Result type — constructors: Ok(T), Err(E)
    Result,
    /// List — infinite (empty, non-empty)
    List,
    /// Unknown type (treat as infinite constructors)
    Unknown,
}

impl TypeDesc {
    /// Returns the list of all constructors for this type.
    pub fn constructors(&self) -> Vec<Constructor> {
        match self {
            TypeDesc::Bool => vec![
                Constructor::BoolTrue,
                Constructor::BoolFalse,
            ],
            TypeDesc::Int => vec![Constructor::IntRange],
            TypeDesc::Float => vec![Constructor::FloatRange],
            TypeDesc::Str => vec![Constructor::StringWild],
            TypeDesc::Enum { name, variants } => {
                variants.iter().map(|(vname, arity)| Constructor::Variant {
                    enum_name: name.clone(),
                    variant_name: vname.clone(),
                    arity: *arity,
                }).collect()
            }
            TypeDesc::Struct { name, fields } => vec![
                Constructor::Struct {
                    name: name.clone(),
                    arity: fields.len(),
                }
            ],
            TypeDesc::Option => vec![
                Constructor::Variant {
                    enum_name: "Option".to_string(),
                    variant_name: "Some".to_string(),
                    arity: 1,
                },
                Constructor::Variant {
                    enum_name: "Option".to_string(),
                    variant_name: "None".to_string(),
                    arity: 0,
                },
            ],
            TypeDesc::Result => vec![
                Constructor::Variant {
                    enum_name: "Result".to_string(),
                    variant_name: "Ok".to_string(),
                    arity: 1,
                },
                Constructor::Variant {
                    enum_name: "Result".to_string(),
                    variant_name: "Err".to_string(),
                    arity: 1,
                },
            ],
            TypeDesc::List => vec![Constructor::ListWild],
            TypeDesc::Unknown => vec![Constructor::Wild],
        }
    }

    /// Returns whether this type has a finite set of constructors.
    pub fn is_finite(&self) -> bool {
        matches!(self, TypeDesc::Bool | TypeDesc::Enum { .. } | TypeDesc::Struct { .. }
            | TypeDesc::Option | TypeDesc::Result)
    }

    /// Returns the number of constructors.
    pub fn constructor_count(&self) -> usize {
        match self {
            TypeDesc::Bool => 2,
            TypeDesc::Enum { variants, .. } => variants.len(),
            TypeDesc::Struct { .. } => 1,
            TypeDesc::Option => 2,
            TypeDesc::Result => 2,
            _ => usize::MAX, // infinite
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Constructors
// ═══════════════════════════════════════════════════════════════════════

/// A pattern constructor — the "head" of a pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor {
    /// Boolean true
    BoolTrue,
    /// Boolean false
    BoolFalse,
    /// Specific integer literal
    IntLit(i64),
    /// Any integer (for range patterns / wildcard matching)
    IntRange,
    /// Specific float literal (as bits for comparison)
    FloatLit(u64),
    /// Any float
    FloatRange,
    /// Specific string literal
    StringLit(String),
    /// Any string
    StringWild,
    /// Enum variant with name and sub-pattern arity
    Variant {
        enum_name: String,
        variant_name: String,
        arity: usize,
    },
    /// Struct with field count
    Struct {
        name: String,
        arity: usize,
    },
    /// List (not fully enumerable)
    ListWild,
    /// Wildcard (matches everything)
    Wild,
}

impl Constructor {
    /// Arity: how many sub-patterns this constructor has
    pub fn arity(&self) -> usize {
        match self {
            Constructor::BoolTrue | Constructor::BoolFalse => 0,
            Constructor::IntLit(_) | Constructor::IntRange => 0,
            Constructor::FloatLit(_) | Constructor::FloatRange => 0,
            Constructor::StringLit(_) | Constructor::StringWild => 0,
            Constructor::Variant { arity, .. } => *arity,
            Constructor::Struct { arity, .. } => *arity,
            Constructor::ListWild => 0,
            Constructor::Wild => 0,
        }
    }

    /// Check if this constructor is a wildcard.
    pub fn is_wild(&self) -> bool {
        matches!(self, Constructor::Wild)
    }
}

impl fmt::Display for Constructor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constructor::BoolTrue => write!(f, "true"),
            Constructor::BoolFalse => write!(f, "false"),
            Constructor::IntLit(n) => write!(f, "{}", n),
            Constructor::IntRange => write!(f, "<int>"),
            Constructor::FloatLit(bits) => write!(f, "<float:{}>", bits),
            Constructor::FloatRange => write!(f, "<float>"),
            Constructor::StringLit(s) => write!(f, "\"{}\"", s),
            Constructor::StringWild => write!(f, "<string>"),
            Constructor::Variant { variant_name, arity, .. } => {
                write!(f, "{}", variant_name)?;
                if *arity > 0 {
                    write!(f, "(")?;
                    for i in 0..*arity {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "_")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            Constructor::Struct { name, .. } => write!(f, "{} {{ .. }}", name),
            Constructor::ListWild => write!(f, "[..]"),
            Constructor::Wild => write!(f, "_"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Patterns (analysis representation)
// ═══════════════════════════════════════════════════════════════════════

/// A pattern in the analysis representation.
///
/// This is a simplified form of the AST `Pattern` that's easier to work
/// with in the usefulness algorithm.
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisPattern {
    /// Matches a specific constructor and its sub-patterns
    Constructor {
        ctor: Constructor,
        sub_pats: Vec<AnalysisPattern>,
    },
    /// Wildcard — matches anything (includes variable bindings)
    Wildcard,
    /// Or-pattern: matches if any alternative matches
    Or(Vec<AnalysisPattern>),
}

impl AnalysisPattern {
    pub fn wildcard() -> Self {
        AnalysisPattern::Wildcard
    }

    pub fn ctor(ctor: Constructor, sub_pats: Vec<AnalysisPattern>) -> Self {
        AnalysisPattern::Constructor { ctor, sub_pats }
    }

    pub fn bool_true() -> Self {
        Self::ctor(Constructor::BoolTrue, vec![])
    }

    pub fn bool_false() -> Self {
        Self::ctor(Constructor::BoolFalse, vec![])
    }

    pub fn int_lit(n: i64) -> Self {
        Self::ctor(Constructor::IntLit(n), vec![])
    }

    pub fn string_lit(s: &str) -> Self {
        Self::ctor(Constructor::StringLit(s.to_string()), vec![])
    }

    pub fn variant(enum_name: &str, variant_name: &str, sub_pats: Vec<AnalysisPattern>) -> Self {
        let arity = sub_pats.len();
        Self::ctor(
            Constructor::Variant {
                enum_name: enum_name.to_string(),
                variant_name: variant_name.to_string(),
                arity,
            },
            sub_pats,
        )
    }

    pub fn struct_pat(name: &str, field_pats: Vec<AnalysisPattern>) -> Self {
        let arity = field_pats.len();
        Self::ctor(Constructor::Struct { name: name.to_string(), arity }, field_pats)
    }

    pub fn or(alts: Vec<AnalysisPattern>) -> Self {
        AnalysisPattern::Or(alts)
    }

    /// Check if this pattern is a wildcard or variable binding.
    pub fn is_wildcard(&self) -> bool {
        matches!(self, AnalysisPattern::Wildcard)
    }

    /// Get the head constructor (if not a wildcard or or-pattern).
    pub fn head_ctor(&self) -> Option<&Constructor> {
        match self {
            AnalysisPattern::Constructor { ctor, .. } => Some(ctor),
            _ => None,
        }
    }
}

impl fmt::Display for AnalysisPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisPattern::Wildcard => write!(f, "_"),
            AnalysisPattern::Constructor { ctor, sub_pats } => {
                write!(f, "{}", ctor)?;
                if !sub_pats.is_empty() {
                    write!(f, "(")?;
                    for (i, p) in sub_pats.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", p)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            AnalysisPattern::Or(alts) => {
                for (i, alt) in alts.iter().enumerate() {
                    if i > 0 { write!(f, " | ")?; }
                    write!(f, "{}", alt)?;
                }
                Ok(())
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Pattern Matrix
// ═══════════════════════════════════════════════════════════════════════

/// A matrix of patterns for exhaustiveness analysis.
///
/// Each row corresponds to a match arm. The matrix is specialized by
/// constructor to reduce the problem size.
#[derive(Debug, Clone)]
pub struct PatternMatrix {
    /// Rows of patterns. Each row has the same number of columns.
    pub rows: Vec<PatternRow>,
    /// Number of columns
    pub width: usize,
}

/// A single row in the pattern matrix.
#[derive(Debug, Clone)]
pub struct PatternRow {
    /// The patterns in this row
    pub patterns: Vec<AnalysisPattern>,
    /// Whether this arm has a guard (guarded arms don't count for exhaustiveness)
    pub has_guard: bool,
    /// Original arm index (for reporting)
    pub arm_index: usize,
}

impl PatternMatrix {
    pub fn new(width: usize) -> Self {
        Self {
            rows: Vec::new(),
            width,
        }
    }

    pub fn add_row(&mut self, patterns: Vec<AnalysisPattern>, has_guard: bool, arm_index: usize) {
        assert_eq!(patterns.len(), self.width, "row width mismatch");
        self.rows.push(PatternRow {
            patterns,
            has_guard,
            arm_index,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// Specialize the matrix for a given constructor.
    ///
    /// For each row:
    /// - If the first pattern matches the constructor → expand sub-patterns + rest
    /// - If the first pattern is a wildcard → expand with wildcards + rest
    /// - If the first pattern is a different constructor → remove the row
    /// - If the first pattern is an or-pattern → expand each alternative
    pub fn specialize(&self, ctor: &Constructor, type_desc: &TypeDesc) -> PatternMatrix {
        let ctor_arity = ctor.arity();
        let new_width = ctor_arity + self.width - 1;
        let mut result = PatternMatrix::new(new_width);

        for row in &self.rows {
            if row.patterns.is_empty() { continue; }
            let first = &row.patterns[0];
            let rest: Vec<AnalysisPattern> = row.patterns[1..].to_vec();

            match first {
                AnalysisPattern::Constructor { ctor: row_ctor, sub_pats } => {
                    if constructors_compatible(row_ctor, ctor) {
                        let mut new_pats = sub_pats.clone();
                        new_pats.extend(rest);
                        result.add_row(new_pats, row.has_guard, row.arm_index);
                    }
                    // else: different constructor, skip this row
                }
                AnalysisPattern::Wildcard => {
                    // Wildcard matches any constructor
                    let mut new_pats: Vec<AnalysisPattern> = (0..ctor_arity)
                        .map(|_| AnalysisPattern::Wildcard)
                        .collect();
                    new_pats.extend(rest);
                    result.add_row(new_pats, row.has_guard, row.arm_index);
                }
                AnalysisPattern::Or(alts) => {
                    // Expand or-pattern: each alternative becomes its own row
                    for alt in alts {
                        let mut expanded = vec![alt.clone()];
                        expanded.extend(rest.clone());
                        let sub_matrix = PatternMatrix {
                            rows: vec![PatternRow {
                                patterns: expanded,
                                has_guard: row.has_guard,
                                arm_index: row.arm_index,
                            }],
                            width: self.width,
                        };
                        let specialized = sub_matrix.specialize(ctor, type_desc);
                        for sub_row in specialized.rows {
                            result.rows.push(sub_row);
                        }
                    }
                }
            }
        }

        result
    }

    /// Default matrix: keeps only wildcard rows with the first column removed.
    /// Used when the type has infinite constructors.
    pub fn default_matrix(&self) -> PatternMatrix {
        let new_width = if self.width > 0 { self.width - 1 } else { 0 };
        let mut result = PatternMatrix::new(new_width);

        for row in &self.rows {
            if row.patterns.is_empty() { continue; }
            let first = &row.patterns[0];
            let rest: Vec<AnalysisPattern> = row.patterns[1..].to_vec();

            match first {
                AnalysisPattern::Wildcard => {
                    result.add_row(rest, row.has_guard, row.arm_index);
                }
                AnalysisPattern::Or(alts) => {
                    // If any alternative is a wildcard, include this row
                    if alts.iter().any(|a| a.is_wildcard()) {
                        result.add_row(rest, row.has_guard, row.arm_index);
                    }
                }
                _ => {} // Skip constructor patterns
            }
        }

        result
    }

    /// Collect all head constructors that appear in the first column.
    pub fn head_constructors(&self) -> Vec<Constructor> {
        let mut ctors = Vec::new();
        let mut seen = HashSet::new();

        for row in &self.rows {
            if row.patterns.is_empty() { continue; }
            match &row.patterns[0] {
                AnalysisPattern::Constructor { ctor, .. } => {
                    let key = format!("{:?}", ctor);
                    if seen.insert(key) {
                        ctors.push(ctor.clone());
                    }
                }
                AnalysisPattern::Or(alts) => {
                    for alt in alts {
                        if let AnalysisPattern::Constructor { ctor, .. } = alt {
                            let key = format!("{:?}", ctor);
                            if seen.insert(key) {
                                ctors.push(ctor.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        ctors
    }
}

/// Check if two constructors are compatible (represent the same variant/value).
fn constructors_compatible(a: &Constructor, b: &Constructor) -> bool {
    match (a, b) {
        (Constructor::BoolTrue, Constructor::BoolTrue) => true,
        (Constructor::BoolFalse, Constructor::BoolFalse) => true,
        (Constructor::IntLit(x), Constructor::IntLit(y)) => x == y,
        (Constructor::IntLit(_), Constructor::IntRange) => true,
        (Constructor::IntRange, Constructor::IntLit(_)) => true,
        (Constructor::IntRange, Constructor::IntRange) => true,
        (Constructor::FloatLit(x), Constructor::FloatLit(y)) => x == y,
        (Constructor::FloatLit(_), Constructor::FloatRange) => true,
        (Constructor::FloatRange, Constructor::FloatLit(_)) => true,
        (Constructor::FloatRange, Constructor::FloatRange) => true,
        (Constructor::StringLit(x), Constructor::StringLit(y)) => x == y,
        (Constructor::StringLit(_), Constructor::StringWild) => true,
        (Constructor::StringWild, Constructor::StringLit(_)) => true,
        (Constructor::StringWild, Constructor::StringWild) => true,
        (Constructor::Variant { variant_name: a, enum_name: ae, .. },
         Constructor::Variant { variant_name: b, enum_name: be, .. }) => a == b && ae == be,
        (Constructor::Struct { name: a, .. }, Constructor::Struct { name: b, .. }) => a == b,
        (Constructor::Wild, _) | (_, Constructor::Wild) => true,
        _ => false,
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Usefulness Algorithm
// ═══════════════════════════════════════════════════════════════════════

/// Check if a pattern vector is useful with respect to a pattern matrix.
///
/// A pattern vector `q` is useful w.r.t. matrix `P` if there is a value that
/// `q` matches but no row in `P` matches.
pub fn is_useful(matrix: &PatternMatrix, query: &[AnalysisPattern], type_desc: &TypeDesc) -> bool {
    // Base case: empty matrix
    if matrix.width == 0 {
        // If the matrix has no columns, useful iff no rows (no prior patterns)
        return matrix.rows.iter().all(|r| r.has_guard);
    }

    if query.is_empty() {
        return true;
    }

    let first = &query[0];
    let rest = &query[1..];

    match first {
        AnalysisPattern::Constructor { ctor, sub_pats } => {
            let specialized = matrix.specialize(ctor, type_desc);
            let mut extended_query = sub_pats.clone();
            extended_query.extend_from_slice(rest);
            is_useful(&specialized, &extended_query, type_desc)
        }
        AnalysisPattern::Wildcard => {
            let head_ctors = matrix.head_constructors();

            if type_desc.is_finite() && head_ctors_cover_type(&head_ctors, type_desc) {
                // Complete: check usefulness against each constructor
                let all_ctors = type_desc.constructors();
                for ctor in &all_ctors {
                    let specialized = matrix.specialize(ctor, type_desc);
                    let mut sub_query: Vec<AnalysisPattern> = (0..ctor.arity())
                        .map(|_| AnalysisPattern::Wildcard)
                        .collect();
                    sub_query.extend_from_slice(rest);
                    if is_useful(&specialized, &sub_query, type_desc) {
                        return true;
                    }
                }
                false
            } else {
                // Incomplete: use default matrix
                let def = matrix.default_matrix();
                is_useful(&def, rest, type_desc)
            }
        }
        AnalysisPattern::Or(alts) => {
            // An or-pattern is useful if any alternative is useful
            for alt in alts {
                let mut expanded = vec![alt.clone()];
                expanded.extend_from_slice(rest);
                if is_useful(matrix, &expanded, type_desc) {
                    return true;
                }
            }
            false
        }
    }
}

/// Check if the head constructors in the matrix cover all constructors of the type.
fn head_ctors_cover_type(head_ctors: &[Constructor], type_desc: &TypeDesc) -> bool {
    if !type_desc.is_finite() {
        return false;
    }
    let all_ctors = type_desc.constructors();
    all_ctors.iter().all(|target_ctor| {
        head_ctors.iter().any(|hc| constructors_compatible(hc, target_ctor))
    })
}

// ═══════════════════════════════════════════════════════════════════════
//  Exhaustiveness & Redundancy Checking
// ═══════════════════════════════════════════════════════════════════════

/// Result of exhaustiveness analysis on a match expression.
#[derive(Debug, Clone)]
pub struct ExhaustivenessResult {
    /// Whether the match is exhaustive (covers all possible values)
    pub is_exhaustive: bool,
    /// Missing patterns (witness values that aren't matched)
    pub missing_patterns: Vec<String>,
    /// Indices of redundant (unreachable) arms
    pub redundant_arms: Vec<usize>,
    /// Indices of arms with guards (treated as non-exhaustive for safety)
    pub guarded_arms: Vec<usize>,
    /// Warnings generated during analysis
    pub warnings: Vec<ExhaustivenessWarning>,
}

/// A warning about pattern matching quality.
#[derive(Debug, Clone)]
pub struct ExhaustivenessWarning {
    pub kind: WarningKind,
    pub message: String,
    pub arm_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarningKind {
    /// Non-exhaustive match — missing patterns
    NonExhaustive,
    /// Unreachable arm — previous patterns already cover it
    UnreachableArm,
    /// Guarded arm doesn't guarantee coverage
    GuardedArm,
    /// Catch-all wildcard makes subsequent arms unreachable
    CatchAllShadows,
}

/// Check exhaustiveness and redundancy of a match expression.
///
/// * `arms` — (pattern, has_guard) pairs for each match arm
/// * `type_desc` — describes the type being matched on
pub fn check_match(
    arms: &[(AnalysisPattern, bool)],
    type_desc: &TypeDesc,
) -> ExhaustivenessResult {
    let mut result = ExhaustivenessResult {
        is_exhaustive: false,
        missing_patterns: Vec::new(),
        redundant_arms: Vec::new(),
        guarded_arms: Vec::new(),
        warnings: Vec::new(),
    };

    // Build the pattern matrix incrementally, checking usefulness of each arm
    let mut matrix = PatternMatrix::new(1);

    for (i, (pat, has_guard)) in arms.iter().enumerate() {
        if *has_guard {
            result.guarded_arms.push(i);
        }

        // Check if this arm is useful (not redundant)
        let query = vec![pat.clone()];
        let useful = is_useful(&matrix, &query, type_desc);

        if !useful && !has_guard {
            result.redundant_arms.push(i);
            result.warnings.push(ExhaustivenessWarning {
                kind: WarningKind::UnreachableArm,
                message: format!("match arm {} is unreachable — previous patterns already cover it", i),
                arm_index: Some(i),
            });
        }

        // Add arm to the matrix
        matrix.add_row(vec![pat.clone()], *has_guard, i);
    }

    // Check exhaustiveness: is the wildcard pattern useful?
    let wildcard_useful = is_useful(&matrix, &[AnalysisPattern::Wildcard], type_desc);

    if wildcard_useful {
        result.is_exhaustive = false;
        // Compute missing patterns (witnesses)
        result.missing_patterns = compute_missing_patterns(&matrix, type_desc);
        result.warnings.push(ExhaustivenessWarning {
            kind: WarningKind::NonExhaustive,
            message: format!(
                "non-exhaustive match — missing patterns: {}",
                result.missing_patterns.join(", ")
            ),
            arm_index: None,
        });
    } else {
        result.is_exhaustive = true;
    }

    // Detect catch-all that shadows subsequent arms
    for i in 0..arms.len() {
        if arms[i].0.is_wildcard() && !arms[i].1 && i < arms.len() - 1 {
            for j in (i + 1)..arms.len() {
                result.warnings.push(ExhaustivenessWarning {
                    kind: WarningKind::CatchAllShadows,
                    message: format!("arm {} is unreachable because arm {} is a catch-all wildcard", j, i),
                    arm_index: Some(j),
                });
            }
            break;
        }
    }

    result
}

/// Compute the missing patterns (witnesses to non-exhaustiveness).
fn compute_missing_patterns(matrix: &PatternMatrix, type_desc: &TypeDesc) -> Vec<String> {
    let mut missing = Vec::new();

    if type_desc.is_finite() {
        let all_ctors = type_desc.constructors();
        let head_ctors = matrix.head_constructors();

        for ctor in &all_ctors {
            if !head_ctors.iter().any(|hc| constructors_compatible(hc, ctor)) {
                missing.push(format!("{}", ctor));
            }
        }
    }

    if missing.is_empty() {
        missing.push("_".to_string());
    }

    missing
}

// ═══════════════════════════════════════════════════════════════════════
//  AST Pattern Conversion
// ═══════════════════════════════════════════════════════════════════════

/// Convert an AST Pattern to an AnalysisPattern.
///
/// This bridges the AST representation (from parser) to the analysis
/// representation used by the exhaustiveness algorithm.
pub fn ast_pattern_to_analysis(pat: &crate::ast::Pattern) -> AnalysisPattern {
    match pat {
        crate::ast::Pattern::Wildcard(_) => AnalysisPattern::Wildcard,
        crate::ast::Pattern::Ident(_, _) => {
            // Variable bindings act as wildcards for exhaustiveness
            AnalysisPattern::Wildcard
        }
        crate::ast::Pattern::Literal(expr) => {
            match expr {
                crate::ast::Expr::IntLiteral(n, _) => AnalysisPattern::int_lit(*n),
                crate::ast::Expr::BoolLiteral(b, _) => {
                    if *b { AnalysisPattern::bool_true() } else { AnalysisPattern::bool_false() }
                }
                crate::ast::Expr::StringLiteral(s, _) => AnalysisPattern::string_lit(s),
                _ => AnalysisPattern::Wildcard,
            }
        }
        crate::ast::Pattern::Variant { name, fields, .. } => {
            let sub_pats: Vec<AnalysisPattern> = fields.iter()
                .map(|f| ast_pattern_to_analysis(f))
                .collect();
            let arity = sub_pats.len();
            AnalysisPattern::Constructor {
                ctor: Constructor::Variant {
                    enum_name: String::new(), // resolved during type checking
                    variant_name: name.clone(),
                    arity,
                },
                sub_pats,
            }
        }
        crate::ast::Pattern::Struct { name, fields, .. } => {
            let sub_pats: Vec<AnalysisPattern> = fields.iter()
                .map(|(_, p)| ast_pattern_to_analysis(p))
                .collect();
            let arity = sub_pats.len();
            AnalysisPattern::Constructor {
                ctor: Constructor::Struct {
                    name: name.clone(),
                    arity,
                },
                sub_pats,
            }
        }
        crate::ast::Pattern::Or { patterns, .. } => {
            let alts: Vec<AnalysisPattern> = patterns.iter()
                .map(|p| ast_pattern_to_analysis(p))
                .collect();
            AnalysisPattern::Or(alts)
        }
        crate::ast::Pattern::Tuple { elements, .. } => {
            let sub_pats: Vec<AnalysisPattern> = elements.iter()
                .map(|p| ast_pattern_to_analysis(p))
                .collect();
            let arity = sub_pats.len();
            AnalysisPattern::Constructor {
                ctor: Constructor::Struct {
                    name: "tuple".to_string(),
                    arity,
                },
                sub_pats,
            }
        }
    }
}

/// Convenience: analyze a match expression given AST MatchArms and a type descriptor.
pub fn analyze_match_arms(
    arms: &[crate::ast::MatchArm],
    type_desc: &TypeDesc,
) -> ExhaustivenessResult {
    let analysis_arms: Vec<(AnalysisPattern, bool)> = arms.iter()
        .map(|arm| {
            let pat = ast_pattern_to_analysis(&arm.pattern);
            let has_guard = arm.guard.is_some();
            (pat, has_guard)
        })
        .collect();

    check_match(&analysis_arms, type_desc)
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Type Descriptors ──

    #[test]
    fn test_bool_type_constructors() {
        let ty = TypeDesc::Bool;
        assert_eq!(ty.constructors().len(), 2);
        assert!(ty.is_finite());
        assert_eq!(ty.constructor_count(), 2);
    }

    #[test]
    fn test_enum_type_constructors() {
        let ty = TypeDesc::Enum {
            name: "Color".to_string(),
            variants: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 0),
                ("Blue".to_string(), 0),
            ],
        };
        assert_eq!(ty.constructors().len(), 3);
        assert!(ty.is_finite());
    }

    #[test]
    fn test_int_type_infinite() {
        let ty = TypeDesc::Int;
        assert!(!ty.is_finite());
    }

    #[test]
    fn test_option_type() {
        let ty = TypeDesc::Option;
        assert!(ty.is_finite());
        assert_eq!(ty.constructor_count(), 2);
    }

    #[test]
    fn test_result_type() {
        let ty = TypeDesc::Result;
        assert!(ty.is_finite());
        assert_eq!(ty.constructor_count(), 2);
    }

    // ── Constructors ──

    #[test]
    fn test_constructor_arity() {
        assert_eq!(Constructor::BoolTrue.arity(), 0);
        assert_eq!(Constructor::IntLit(42).arity(), 0);
        assert_eq!(Constructor::Variant {
            enum_name: "E".into(), variant_name: "V".into(), arity: 2,
        }.arity(), 2);
        assert_eq!(Constructor::Struct { name: "S".into(), arity: 3 }.arity(), 3);
    }

    #[test]
    fn test_constructor_display() {
        assert_eq!(format!("{}", Constructor::BoolTrue), "true");
        assert_eq!(format!("{}", Constructor::IntLit(42)), "42");
        assert_eq!(format!("{}", Constructor::Variant {
            enum_name: "E".into(), variant_name: "Some".into(), arity: 1,
        }), "Some(_)");
    }

    #[test]
    fn test_constructors_compatible() {
        assert!(constructors_compatible(&Constructor::BoolTrue, &Constructor::BoolTrue));
        assert!(!constructors_compatible(&Constructor::BoolTrue, &Constructor::BoolFalse));
        assert!(constructors_compatible(&Constructor::IntLit(5), &Constructor::IntLit(5)));
        assert!(!constructors_compatible(&Constructor::IntLit(5), &Constructor::IntLit(6)));
        assert!(constructors_compatible(&Constructor::Wild, &Constructor::BoolTrue));
    }

    // ── Analysis Patterns ──

    #[test]
    fn test_analysis_pattern_creation() {
        let w = AnalysisPattern::wildcard();
        assert!(w.is_wildcard());

        let b = AnalysisPattern::bool_true();
        assert!(!b.is_wildcard());
        assert_eq!(b.head_ctor(), Some(&Constructor::BoolTrue));
    }

    #[test]
    fn test_or_pattern() {
        let or = AnalysisPattern::or(vec![
            AnalysisPattern::int_lit(1),
            AnalysisPattern::int_lit(2),
        ]);
        assert!(!or.is_wildcard());
        let display = format!("{}", or);
        assert!(display.contains("1"));
        assert!(display.contains("2"));
    }

    #[test]
    fn test_variant_pattern() {
        let pat = AnalysisPattern::variant("Option", "Some", vec![AnalysisPattern::wildcard()]);
        if let AnalysisPattern::Constructor { ctor, sub_pats } = &pat {
            assert_eq!(ctor.arity(), 1);
            assert_eq!(sub_pats.len(), 1);
        } else {
            panic!("expected constructor");
        }
    }

    #[test]
    fn test_struct_pattern() {
        let pat = AnalysisPattern::struct_pat("Point", vec![
            AnalysisPattern::wildcard(),
            AnalysisPattern::wildcard(),
        ]);
        if let AnalysisPattern::Constructor { ctor, .. } = &pat {
            assert_eq!(ctor.arity(), 2);
        } else {
            panic!("expected constructor");
        }
    }

    // ── Pattern Matrix ──

    #[test]
    fn test_matrix_basic() {
        let mut m = PatternMatrix::new(1);
        m.add_row(vec![AnalysisPattern::bool_true()], false, 0);
        assert_eq!(m.height(), 1);
        assert_eq!(m.width, 1);
    }

    #[test]
    fn test_matrix_head_constructors() {
        let mut m = PatternMatrix::new(1);
        m.add_row(vec![AnalysisPattern::bool_true()], false, 0);
        m.add_row(vec![AnalysisPattern::bool_false()], false, 1);
        let ctors = m.head_constructors();
        assert_eq!(ctors.len(), 2);
    }

    #[test]
    fn test_matrix_specialize() {
        let mut m = PatternMatrix::new(1);
        m.add_row(vec![AnalysisPattern::bool_true()], false, 0);
        m.add_row(vec![AnalysisPattern::wildcard()], false, 1);

        let specialized = m.specialize(&Constructor::BoolTrue, &TypeDesc::Bool);
        assert_eq!(specialized.height(), 2); // both rows match BoolTrue

        let specialized_false = m.specialize(&Constructor::BoolFalse, &TypeDesc::Bool);
        assert_eq!(specialized_false.height(), 1); // only wildcard row
    }

    #[test]
    fn test_matrix_default() {
        let mut m = PatternMatrix::new(1);
        m.add_row(vec![AnalysisPattern::int_lit(1)], false, 0);
        m.add_row(vec![AnalysisPattern::wildcard()], false, 1);

        let def = m.default_matrix();
        assert_eq!(def.height(), 1); // only the wildcard row
    }

    // ── Exhaustiveness: Bool ──

    #[test]
    fn test_bool_exhaustive_true_false() {
        let arms = vec![
            (AnalysisPattern::bool_true(), false),
            (AnalysisPattern::bool_false(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.is_exhaustive);
        assert!(result.redundant_arms.is_empty());
    }

    #[test]
    fn test_bool_non_exhaustive_true_only() {
        let arms = vec![
            (AnalysisPattern::bool_true(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(!result.is_exhaustive);
        assert!(result.missing_patterns.iter().any(|p| p.contains("false")));
    }

    #[test]
    fn test_bool_exhaustive_with_wildcard() {
        let arms = vec![
            (AnalysisPattern::bool_true(), false),
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_bool_wildcard_only() {
        let arms = vec![
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.is_exhaustive);
    }

    // ── Exhaustiveness: Enum ──

    #[test]
    fn test_enum_exhaustive() {
        let ty = TypeDesc::Enum {
            name: "Color".to_string(),
            variants: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 0),
                ("Blue".to_string(), 0),
            ],
        };
        let arms = vec![
            (AnalysisPattern::variant("Color", "Red", vec![]), false),
            (AnalysisPattern::variant("Color", "Green", vec![]), false),
            (AnalysisPattern::variant("Color", "Blue", vec![]), false),
        ];
        let result = check_match(&arms, &ty);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_enum_non_exhaustive() {
        let ty = TypeDesc::Enum {
            name: "Color".to_string(),
            variants: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 0),
                ("Blue".to_string(), 0),
            ],
        };
        let arms = vec![
            (AnalysisPattern::variant("Color", "Red", vec![]), false),
            (AnalysisPattern::variant("Color", "Green", vec![]), false),
        ];
        let result = check_match(&arms, &ty);
        assert!(!result.is_exhaustive);
        assert!(result.missing_patterns.iter().any(|p| p.contains("Blue")));
    }

    #[test]
    fn test_enum_with_wildcard() {
        let ty = TypeDesc::Enum {
            name: "Color".to_string(),
            variants: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 0),
                ("Blue".to_string(), 0),
            ],
        };
        let arms = vec![
            (AnalysisPattern::variant("Color", "Red", vec![]), false),
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &ty);
        assert!(result.is_exhaustive);
    }

    // ── Exhaustiveness: Option ──

    #[test]
    fn test_option_exhaustive() {
        let arms = vec![
            (AnalysisPattern::variant("Option", "Some", vec![AnalysisPattern::wildcard()]), false),
            (AnalysisPattern::variant("Option", "None", vec![]), false),
        ];
        let result = check_match(&arms, &TypeDesc::Option);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_option_missing_none() {
        let arms = vec![
            (AnalysisPattern::variant("Option", "Some", vec![AnalysisPattern::wildcard()]), false),
        ];
        let result = check_match(&arms, &TypeDesc::Option);
        assert!(!result.is_exhaustive);
        assert!(result.missing_patterns.iter().any(|p| p.contains("None")));
    }

    // ── Exhaustiveness: Int ──

    #[test]
    fn test_int_needs_wildcard() {
        let arms = vec![
            (AnalysisPattern::int_lit(0), false),
            (AnalysisPattern::int_lit(1), false),
        ];
        let result = check_match(&arms, &TypeDesc::Int);
        assert!(!result.is_exhaustive);
    }

    #[test]
    fn test_int_with_wildcard() {
        let arms = vec![
            (AnalysisPattern::int_lit(0), false),
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Int);
        assert!(result.is_exhaustive);
    }

    // ── Redundancy ──

    #[test]
    fn test_redundant_arm() {
        let arms = vec![
            (AnalysisPattern::wildcard(), false),
            (AnalysisPattern::bool_true(), false), // redundant — wildcard already covers
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.redundant_arms.contains(&1));
    }

    #[test]
    fn test_no_redundancy() {
        let arms = vec![
            (AnalysisPattern::bool_true(), false),
            (AnalysisPattern::bool_false(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.redundant_arms.is_empty());
    }

    #[test]
    fn test_duplicate_arm_redundant() {
        let arms = vec![
            (AnalysisPattern::int_lit(1), false),
            (AnalysisPattern::int_lit(1), false), // duplicate
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Int);
        assert!(result.redundant_arms.contains(&1));
    }

    // ── Guards ──

    #[test]
    fn test_guarded_arm_not_exhaustive() {
        // A guarded wildcard doesn't guarantee exhaustiveness
        let arms = vec![
            (AnalysisPattern::bool_true(), false),
            (AnalysisPattern::wildcard(), true), // guarded — may not match
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        // Guarded arms are tracked
        assert!(result.guarded_arms.contains(&1));
    }

    #[test]
    fn test_guard_then_complete() {
        let arms = vec![
            (AnalysisPattern::bool_true(), true),  // guarded
            (AnalysisPattern::bool_true(), false),  // unguarded covers true
            (AnalysisPattern::bool_false(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.is_exhaustive);
    }

    // ── Or-Patterns ──

    #[test]
    fn test_or_pattern_exhaustive() {
        let ty = TypeDesc::Enum {
            name: "Color".to_string(),
            variants: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 0),
                ("Blue".to_string(), 0),
            ],
        };
        let arms = vec![
            (AnalysisPattern::or(vec![
                AnalysisPattern::variant("Color", "Red", vec![]),
                AnalysisPattern::variant("Color", "Green", vec![]),
            ]), false),
            (AnalysisPattern::variant("Color", "Blue", vec![]), false),
        ];
        let result = check_match(&arms, &ty);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_or_pattern_partial() {
        let ty = TypeDesc::Enum {
            name: "Color".to_string(),
            variants: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 0),
                ("Blue".to_string(), 0),
            ],
        };
        let arms = vec![
            (AnalysisPattern::or(vec![
                AnalysisPattern::variant("Color", "Red", vec![]),
                AnalysisPattern::variant("Color", "Green", vec![]),
            ]), false),
        ];
        let result = check_match(&arms, &ty);
        assert!(!result.is_exhaustive);
        assert!(result.missing_patterns.iter().any(|p| p.contains("Blue")));
    }

    // ── Catch-All Shadows ──

    #[test]
    fn test_catch_all_shadows_warning() {
        let arms = vec![
            (AnalysisPattern::int_lit(1), false),
            (AnalysisPattern::wildcard(), false),
            (AnalysisPattern::int_lit(2), false), // shadowed by wildcard
        ];
        let result = check_match(&arms, &TypeDesc::Int);
        assert!(result.warnings.iter().any(|w| w.kind == WarningKind::CatchAllShadows));
    }

    // ── Struct Patterns ──

    #[test]
    fn test_struct_exhaustive_with_wildcard() {
        let ty = TypeDesc::Struct {
            name: "Point".to_string(),
            fields: vec!["x".to_string(), "y".to_string()],
        };
        let arms = vec![
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &ty);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_struct_pattern_with_fields() {
        let ty = TypeDesc::Struct {
            name: "Point".to_string(),
            fields: vec!["x".to_string(), "y".to_string()],
        };
        let arms = vec![
            (AnalysisPattern::struct_pat("Point", vec![
                AnalysisPattern::wildcard(),
                AnalysisPattern::wildcard(),
            ]), false),
        ];
        let result = check_match(&arms, &ty);
        assert!(result.is_exhaustive);
    }

    // ── Warnings ──

    #[test]
    fn test_warning_kinds() {
        let arms = vec![
            (AnalysisPattern::wildcard(), false),
            (AnalysisPattern::bool_true(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.warnings.iter().any(|w| w.kind == WarningKind::UnreachableArm));
        assert!(result.warnings.iter().any(|w| w.kind == WarningKind::CatchAllShadows));
    }

    #[test]
    fn test_no_warnings_clean_match() {
        let arms = vec![
            (AnalysisPattern::bool_true(), false),
            (AnalysisPattern::bool_false(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(result.warnings.is_empty());
    }

    // ── AST Conversion ──

    #[test]
    fn test_ast_wildcard_conversion() {
        let ast_pat = crate::ast::Pattern::Wildcard(crate::ast::Span::new(0, 1));
        let analysis = ast_pattern_to_analysis(&ast_pat);
        assert!(analysis.is_wildcard());
    }

    #[test]
    fn test_ast_ident_is_wildcard() {
        let ast_pat = crate::ast::Pattern::Ident("x".to_string(), crate::ast::Span::new(0, 1));
        let analysis = ast_pattern_to_analysis(&ast_pat);
        assert!(analysis.is_wildcard()); // variable bindings are wildcards
    }

    #[test]
    fn test_ast_literal_conversion() {
        let ast_pat = crate::ast::Pattern::Literal(
            crate::ast::Expr::IntLiteral(42, crate::ast::Span::new(0, 2))
        );
        let analysis = ast_pattern_to_analysis(&ast_pat);
        assert_eq!(analysis.head_ctor(), Some(&Constructor::IntLit(42)));
    }

    #[test]
    fn test_ast_bool_literal_conversion() {
        let ast_pat = crate::ast::Pattern::Literal(
            crate::ast::Expr::BoolLiteral(true, crate::ast::Span::new(0, 4))
        );
        let analysis = ast_pattern_to_analysis(&ast_pat);
        assert_eq!(analysis.head_ctor(), Some(&Constructor::BoolTrue));
    }

    #[test]
    fn test_ast_variant_conversion() {
        let ast_pat = crate::ast::Pattern::Variant {
            name: "Some".to_string(),
            fields: vec![crate::ast::Pattern::Ident("x".to_string(), crate::ast::Span::new(5, 6))],
            span: crate::ast::Span::new(0, 7),
        };
        let analysis = ast_pattern_to_analysis(&ast_pat);
        if let AnalysisPattern::Constructor { ctor, sub_pats } = &analysis {
            match ctor {
                Constructor::Variant { variant_name, arity, .. } => {
                    assert_eq!(variant_name, "Some");
                    assert_eq!(*arity, 1);
                }
                _ => panic!("expected variant constructor"),
            }
            assert_eq!(sub_pats.len(), 1);
            assert!(sub_pats[0].is_wildcard()); // x becomes wildcard
        } else {
            panic!("expected constructor pattern");
        }
    }

    #[test]
    fn test_ast_struct_pattern_conversion() {
        let ast_pat = crate::ast::Pattern::Struct {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), crate::ast::Pattern::Wildcard(crate::ast::Span::new(0, 1))),
                ("y".to_string(), crate::ast::Pattern::Literal(
                    crate::ast::Expr::IntLiteral(0, crate::ast::Span::new(0, 1))
                )),
            ],
            span: crate::ast::Span::new(0, 20),
        };
        let analysis = ast_pattern_to_analysis(&ast_pat);
        if let AnalysisPattern::Constructor { ctor, sub_pats } = &analysis {
            match ctor {
                Constructor::Struct { name, arity } => {
                    assert_eq!(name, "Point");
                    assert_eq!(*arity, 2);
                }
                _ => panic!("expected struct constructor"),
            }
            assert_eq!(sub_pats.len(), 2);
            assert!(sub_pats[0].is_wildcard());
            assert_eq!(sub_pats[1].head_ctor(), Some(&Constructor::IntLit(0)));
        } else {
            panic!("expected constructor pattern");
        }
    }

    // ── Analyze Match Arms (integration) ──

    #[test]
    fn test_analyze_match_arms_integration() {
        let arms = vec![
            crate::ast::MatchArm {
                pattern: crate::ast::Pattern::Literal(
                    crate::ast::Expr::BoolLiteral(true, crate::ast::Span::new(0, 4))
                ),
                guard: None,
                body: crate::ast::Expr::IntLiteral(1, crate::ast::Span::new(8, 9)),
                span: crate::ast::Span::new(0, 9),
            },
            crate::ast::MatchArm {
                pattern: crate::ast::Pattern::Literal(
                    crate::ast::Expr::BoolLiteral(false, crate::ast::Span::new(10, 15))
                ),
                guard: None,
                body: crate::ast::Expr::IntLiteral(0, crate::ast::Span::new(19, 20)),
                span: crate::ast::Span::new(10, 20),
            },
        ];
        let result = analyze_match_arms(&arms, &TypeDesc::Bool);
        assert!(result.is_exhaustive);
        assert!(result.redundant_arms.is_empty());
    }

    #[test]
    fn test_analyze_match_arms_with_guard() {
        let arms = vec![
            crate::ast::MatchArm {
                pattern: crate::ast::Pattern::Wildcard(crate::ast::Span::new(0, 1)),
                guard: Some(crate::ast::Expr::BoolLiteral(true, crate::ast::Span::new(5, 9))),
                body: crate::ast::Expr::IntLiteral(1, crate::ast::Span::new(13, 14)),
                span: crate::ast::Span::new(0, 14),
            },
        ];
        let result = analyze_match_arms(&arms, &TypeDesc::Bool);
        assert_eq!(result.guarded_arms, vec![0]);
    }

    // ── Edge Cases ──

    #[test]
    fn test_empty_match() {
        let arms: Vec<(AnalysisPattern, bool)> = vec![];
        let result = check_match(&arms, &TypeDesc::Bool);
        assert!(!result.is_exhaustive);
    }

    #[test]
    fn test_single_wildcard_exhaustive() {
        let arms = vec![
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Int);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_result_exhaustive() {
        let arms = vec![
            (AnalysisPattern::variant("Result", "Ok", vec![AnalysisPattern::wildcard()]), false),
            (AnalysisPattern::variant("Result", "Err", vec![AnalysisPattern::wildcard()]), false),
        ];
        let result = check_match(&arms, &TypeDesc::Result);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_multiple_or_patterns() {
        let arms = vec![
            (AnalysisPattern::or(vec![
                AnalysisPattern::int_lit(1),
                AnalysisPattern::int_lit(2),
                AnalysisPattern::int_lit(3),
            ]), false),
            (AnalysisPattern::wildcard(), false),
        ];
        let result = check_match(&arms, &TypeDesc::Int);
        assert!(result.is_exhaustive);
        assert!(result.redundant_arms.is_empty());
    }
}
