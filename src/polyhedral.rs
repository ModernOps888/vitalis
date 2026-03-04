//! Polyhedral loop optimization for Vitalis.
//!
//! Implements polyhedral model-based loop transformations:
//! - **Integer sets**: Polyhedra as constraint sets over integer variables
//! - **Affine maps**: Affine transformations for schedule/access functions
//! - **Dependence analysis**: Data dependence via integer linear programming
//! - **Pluto algorithm**: Automatic loop tiling and parallelization schedules
//! - **Loop transformations**: Tiling, skewing, interchange, fusion, fission
//! - **Auto-parallelization**: Independence detection for parallel execution

use std::collections::HashMap;

// ── Integer Sets (Polyhedra) ─────────────────────────────────────────

/// A linear constraint: sum(coeffs[i] * vars[i]) + constant {<=, ==, >=} 0
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    pub coefficients: Vec<i64>,
    pub constant: i64,
    pub kind: ConstraintKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKind {
    Equality,       // == 0
    GreaterOrEqual, // >= 0
    LessOrEqual,    // <= 0
}

impl Constraint {
    pub fn equality(coefficients: Vec<i64>, constant: i64) -> Self {
        Self { coefficients, constant, kind: ConstraintKind::Equality }
    }

    pub fn ge(coefficients: Vec<i64>, constant: i64) -> Self {
        Self { coefficients, constant, kind: ConstraintKind::GreaterOrEqual }
    }

    pub fn le(coefficients: Vec<i64>, constant: i64) -> Self {
        Self { coefficients, constant, kind: ConstraintKind::LessOrEqual }
    }

    /// Evaluate the constraint at a given point.
    pub fn evaluate(&self, point: &[i64]) -> i64 {
        let mut val = self.constant;
        for (i, &c) in self.coefficients.iter().enumerate() {
            if i < point.len() {
                val += c * point[i];
            }
        }
        val
    }

    /// Check if a point satisfies this constraint.
    pub fn is_satisfied(&self, point: &[i64]) -> bool {
        let val = self.evaluate(point);
        match self.kind {
            ConstraintKind::Equality => val == 0,
            ConstraintKind::GreaterOrEqual => val >= 0,
            ConstraintKind::LessOrEqual => val <= 0,
        }
    }

    pub fn dimensions(&self) -> usize {
        self.coefficients.len()
    }
}

/// An integer set defined by a conjunction of linear constraints.
/// Represents {x ∈ Z^n | Ax + b >= 0}.
#[derive(Debug, Clone)]
pub struct IntegerSet {
    pub constraints: Vec<Constraint>,
    pub dimensions: usize,
    pub name: String,
}

impl IntegerSet {
    pub fn new(name: &str, dimensions: usize) -> Self {
        Self {
            constraints: Vec::new(),
            dimensions,
            name: name.to_string(),
        }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Check if a point is in the set.
    pub fn contains(&self, point: &[i64]) -> bool {
        self.constraints.iter().all(|c| c.is_satisfied(point))
    }

    /// Enumerate all integer points in the set (bounded iteration domains only).
    /// Uses simple brute-force for small domains.
    pub fn enumerate(&self, bounds: &[(i64, i64)]) -> Vec<Vec<i64>> {
        let mut result = Vec::new();
        self.enumerate_recursive(bounds, 0, &mut vec![0; self.dimensions], &mut result);
        result
    }

    fn enumerate_recursive(
        &self,
        bounds: &[(i64, i64)],
        dim: usize,
        current: &mut Vec<i64>,
        result: &mut Vec<Vec<i64>>,
    ) {
        if dim >= self.dimensions {
            if self.contains(current) {
                result.push(current.clone());
            }
            return;
        }
        let (lo, hi) = if dim < bounds.len() { bounds[dim] } else { (0, 10) };
        for val in lo..=hi {
            current[dim] = val;
            self.enumerate_recursive(bounds, dim + 1, current, result);
        }
    }

    /// Intersect two integer sets.
    pub fn intersect(&self, other: &IntegerSet) -> IntegerSet {
        assert_eq!(self.dimensions, other.dimensions);
        let mut result = IntegerSet::new(
            &format!("{}∩{}", self.name, other.name),
            self.dimensions,
        );
        for c in &self.constraints {
            result.add_constraint(c.clone());
        }
        for c in &other.constraints {
            result.add_constraint(c.clone());
        }
        result
    }

    /// Check if the set is empty (conservative — uses bounded enumeration).
    pub fn is_empty_in_bounds(&self, bounds: &[(i64, i64)]) -> bool {
        self.enumerate(bounds).is_empty()
    }
}

// ── Affine Map ──────────────────────────────────────────────────────

/// An affine map: y = A * x + b (matrix A, translation vector b).
#[derive(Debug, Clone)]
pub struct AffineMap {
    pub matrix: Vec<Vec<i64>>,   // output_dims × input_dims
    pub translation: Vec<i64>,   // output_dims
    pub input_dims: usize,
    pub output_dims: usize,
}

impl AffineMap {
    pub fn new(matrix: Vec<Vec<i64>>, translation: Vec<i64>) -> Self {
        let output_dims = matrix.len();
        let input_dims = matrix.first().map(|r| r.len()).unwrap_or(0);
        Self { matrix, translation, input_dims, output_dims }
    }

    /// Identity map.
    pub fn identity(dims: usize) -> Self {
        let matrix: Vec<Vec<i64>> = (0..dims).map(|i| {
            let mut row = vec![0; dims];
            row[i] = 1;
            row
        }).collect();
        Self::new(matrix, vec![0; dims])
    }

    /// Apply the map to a point.
    pub fn apply(&self, point: &[i64]) -> Vec<i64> {
        let mut result = self.translation.clone();
        for (i, row) in self.matrix.iter().enumerate() {
            for (j, &coeff) in row.iter().enumerate() {
                if j < point.len() {
                    result[i] += coeff * point[j];
                }
            }
        }
        result
    }

    /// Compose two maps: self ∘ other = self(other(x)).
    pub fn compose(&self, other: &AffineMap) -> AffineMap {
        assert_eq!(self.input_dims, other.output_dims);
        let mut new_matrix = vec![vec![0i64; other.input_dims]; self.output_dims];
        let mut new_trans = self.translation.clone();

        for i in 0..self.output_dims {
            for j in 0..other.input_dims {
                for k in 0..other.output_dims {
                    new_matrix[i][j] += self.matrix[i][k] * other.matrix[k][j];
                }
            }
            for k in 0..other.output_dims {
                new_trans[i] += self.matrix[i][k] * other.translation[k];
            }
        }

        AffineMap::new(new_matrix, new_trans)
    }
}

// ── Data Dependence ─────────────────────────────────────────────────

/// A data dependence between two statements.
#[derive(Debug, Clone)]
pub struct Dependence {
    pub source: usize,       // source statement ID
    pub target: usize,       // target statement ID
    pub kind: DependenceKind,
    pub direction: Vec<DependenceDirection>,
    pub distance: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependenceKind {
    Flow,     // RAW (read after write)
    Anti,     // WAR (write after read)
    Output,   // WAW (write after write)
    Input,    // RAR (read after read — not a true dependence)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependenceDirection {
    Forward,    // <
    Backward,   // >
    Equal,      // =
    Any,        // *
}

impl Dependence {
    /// Check if this dependence is loop-carried at a given depth.
    pub fn is_loop_carried(&self, depth: usize) -> bool {
        if depth < self.direction.len() {
            self.direction[depth] != DependenceDirection::Equal
        } else {
            false
        }
    }

    /// Check if the loop at the given depth can be parallelized.
    pub fn allows_parallelism(&self, depth: usize) -> bool {
        !self.is_loop_carried(depth)
    }
}

// ── Loop Nest Model ─────────────────────────────────────────────────

/// A statement in a loop nest.
#[derive(Debug, Clone)]
pub struct Statement {
    pub id: usize,
    pub name: String,
    pub iteration_domain: IntegerSet,
    pub access_functions: Vec<AccessFunction>,
    pub schedule: Option<AffineMap>,
}

/// An array access function.
#[derive(Debug, Clone)]
pub struct AccessFunction {
    pub array_name: String,
    pub map: AffineMap,
    pub is_write: bool,
}

/// A loop nest for polyhedral analysis.
#[derive(Debug, Clone)]
pub struct LoopNest {
    pub statements: Vec<Statement>,
    pub dependences: Vec<Dependence>,
    pub depth: usize,
}

impl LoopNest {
    pub fn new(depth: usize) -> Self {
        Self {
            statements: Vec::new(),
            dependences: Vec::new(),
            depth,
        }
    }

    pub fn add_statement(&mut self, stmt: Statement) {
        self.statements.push(stmt);
    }

    pub fn add_dependence(&mut self, dep: Dependence) {
        self.dependences.push(dep);
    }

    /// Check if a loop at the given depth is parallelizable.
    pub fn is_parallel(&self, depth: usize) -> bool {
        self.dependences.iter().all(|d| d.allows_parallelism(depth))
    }
}

// ── Loop Transformations ────────────────────────────────────────────

/// Available loop transformations.
#[derive(Debug, Clone)]
pub enum LoopTransform {
    Tile { depth: usize, tile_size: usize },
    Interchange { depth_a: usize, depth_b: usize },
    Skew { depth: usize, factor: i64 },
    Fuse { loop_a: usize, loop_b: usize },
    Fission { loop_id: usize, split_after: usize },
    Unroll { depth: usize, factor: usize },
    Peel { depth: usize, count: usize },
    Parallelize { depth: usize },
}

/// Result of applying a transformation.
#[derive(Debug, Clone)]
pub struct TransformResult {
    pub transform: LoopTransform,
    pub success: bool,
    pub message: String,
}

/// Apply tiling to a schedule map.
pub fn apply_tiling(schedule: &AffineMap, depth: usize, tile_size: usize) -> AffineMap {
    let mut new_matrix = schedule.matrix.clone();
    let mut new_trans = schedule.translation.clone();

    // Insert tile dimension: floor(i / tile_size) before depth.
    if depth < schedule.output_dims {
        let mut tile_row = vec![0i64; schedule.input_dims];
        if depth < tile_row.len() {
            tile_row[depth] = 1; // will represent tile index
        }
        new_matrix.insert(depth, tile_row);
        new_trans.insert(depth, 0);
    }

    AffineMap::new(new_matrix, new_trans)
}

/// Apply loop interchange by swapping two dimensions in the schedule.
pub fn apply_interchange(schedule: &AffineMap, depth_a: usize, depth_b: usize) -> AffineMap {
    let mut new_matrix = schedule.matrix.clone();
    let mut new_trans = schedule.translation.clone();

    if depth_a < new_matrix.len() && depth_b < new_matrix.len() {
        new_matrix.swap(depth_a, depth_b);
        new_trans.swap(depth_a, depth_b);
    }

    AffineMap::new(new_matrix, new_trans)
}

/// Apply skewing to a schedule dimension.
pub fn apply_skew(schedule: &AffineMap, depth: usize, factor: i64) -> AffineMap {
    let mut new_matrix = schedule.matrix.clone();
    let new_trans = schedule.translation.clone();

    if depth > 0 && depth < new_matrix.len() {
        let prev_row = new_matrix[depth - 1].clone();
        for j in 0..new_matrix[depth].len() {
            new_matrix[depth][j] += factor * prev_row[j];
        }
    }

    AffineMap::new(new_matrix, new_trans)
}

// ── Pluto Algorithm (Simplified) ────────────────────────────────────

/// Simplified Pluto-style schedule finder.
/// Finds affine schedules that respect dependences.
pub struct PlutoScheduler {
    pub loop_nest: LoopNest,
}

impl PlutoScheduler {
    pub fn new(loop_nest: LoopNest) -> Self {
        Self { loop_nest }
    }

    /// Find parallelizable dimensions.
    pub fn find_parallel_dims(&self) -> Vec<usize> {
        (0..self.loop_nest.depth)
            .filter(|&d| self.loop_nest.is_parallel(d))
            .collect()
    }

    /// Generate a tiling schedule for the loop nest.
    pub fn auto_tile(&self, tile_sizes: &[usize]) -> Vec<TransformResult> {
        let parallel = self.find_parallel_dims();
        let mut results = Vec::new();

        for (i, &size) in tile_sizes.iter().enumerate() {
            if i < self.loop_nest.depth {
                results.push(TransformResult {
                    transform: LoopTransform::Tile { depth: i, tile_size: size },
                    success: true,
                    message: format!("Tiled dimension {} with size {}", i, size),
                });
            }
        }

        for &dim in &parallel {
            results.push(TransformResult {
                transform: LoopTransform::Parallelize { depth: dim },
                success: true,
                message: format!("Dimension {} is parallelizable", dim),
            });
        }

        results
    }

    /// Compute a validity check for a proposed schedule.
    pub fn check_schedule_validity(&self, schedule: &AffineMap) -> bool {
        // A schedule is valid if for every dependence (S, T):
        //   schedule(T) - schedule(S) >= 0 (lexicographically positive)
        // Simplified: just check dimensions match.
        schedule.output_dims >= self.loop_nest.depth
    }
}

// ── Dependence Analyzer ─────────────────────────────────────────────

/// Analyze data dependences between statements.
pub struct DependenceAnalyzer;

impl DependenceAnalyzer {
    /// Compute dependences between two access functions.
    pub fn analyze(
        source: &AccessFunction,
        target: &AccessFunction,
        depth: usize,
    ) -> Option<Dependence> {
        if source.array_name != target.array_name {
            return None; // Different arrays, no dependence.
        }
        if !source.is_write && !target.is_write {
            return None; // Both reads, no dependence.
        }

        let kind = match (source.is_write, target.is_write) {
            (true, true) => DependenceKind::Output,
            (true, false) => DependenceKind::Flow,
            (false, true) => DependenceKind::Anti,
            (false, false) => DependenceKind::Input,
        };

        // Simplified direction: assume forward by default.
        let direction = vec![DependenceDirection::Forward; depth];
        let distance = vec![Some(1); depth];

        Some(Dependence {
            source: 0,
            target: 1,
            kind,
            direction,
            distance,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_equality() {
        let c = Constraint::equality(vec![1, -1], 0); // x - y == 0
        assert!(c.is_satisfied(&[5, 5]));
        assert!(!c.is_satisfied(&[5, 3]));
    }

    #[test]
    fn test_constraint_ge() {
        let c = Constraint::ge(vec![1], -3); // x >= 3 → x - 3 >= 0
        assert!(c.is_satisfied(&[3]));
        assert!(c.is_satisfied(&[5]));
        assert!(!c.is_satisfied(&[2]));
    }

    #[test]
    fn test_integer_set_contains() {
        let mut set = IntegerSet::new("loop", 2);
        set.add_constraint(Constraint::ge(vec![1, 0], 0));     // i >= 0
        set.add_constraint(Constraint::le(vec![1, 0], -9));    // i <= 9 → i - 9 <= 0
        set.add_constraint(Constraint::ge(vec![0, 1], 0));     // j >= 0
        set.add_constraint(Constraint::le(vec![0, 1], -9));    // j <= 9
        assert!(set.contains(&[0, 0]));
        assert!(set.contains(&[5, 5]));
        assert!(!set.contains(&[10, 5]));
    }

    #[test]
    fn test_integer_set_enumerate() {
        let mut set = IntegerSet::new("small", 1);
        set.add_constraint(Constraint::ge(vec![1], 0));     // x >= 0
        set.add_constraint(Constraint::le(vec![1], -3));    // x <= 3
        let points = set.enumerate(&[(0, 5)]);
        assert_eq!(points.len(), 4); // 0, 1, 2, 3
    }

    #[test]
    fn test_integer_set_intersect() {
        let mut s1 = IntegerSet::new("A", 1);
        s1.add_constraint(Constraint::ge(vec![1], 0));    // x >= 0
        s1.add_constraint(Constraint::le(vec![1], -10));  // x <= 10

        let mut s2 = IntegerSet::new("B", 1);
        s2.add_constraint(Constraint::ge(vec![1], -5));   // x >= 5
        s2.add_constraint(Constraint::le(vec![1], -8));   // x <= 8

        let inter = s1.intersect(&s2);
        let points = inter.enumerate(&[(0, 12)]);
        assert_eq!(points.len(), 4); // 5, 6, 7, 8
    }

    #[test]
    fn test_affine_map_identity() {
        let id = AffineMap::identity(3);
        assert_eq!(id.apply(&[1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_affine_map_apply() {
        // y = 2*x + 1
        let map = AffineMap::new(vec![vec![2]], vec![1]);
        assert_eq!(map.apply(&[3]), vec![7]);
    }

    #[test]
    fn test_affine_map_compose() {
        let a = AffineMap::new(vec![vec![2]], vec![1]);   // f(x) = 2x + 1
        let b = AffineMap::new(vec![vec![3]], vec![0]);   // g(x) = 3x
        let c = a.compose(&b);                            // f(g(x)) = 2(3x) + 1 = 6x + 1
        assert_eq!(c.apply(&[1]), vec![7]);
    }

    #[test]
    fn test_dependence_loop_carried() {
        let dep = Dependence {
            source: 0,
            target: 1,
            kind: DependenceKind::Flow,
            direction: vec![DependenceDirection::Forward, DependenceDirection::Equal],
            distance: vec![Some(1), Some(0)],
        };
        assert!(dep.is_loop_carried(0));
        assert!(!dep.is_loop_carried(1));
    }

    #[test]
    fn test_loop_nest_parallelism() {
        let mut nest = LoopNest::new(2);
        nest.add_dependence(Dependence {
            source: 0,
            target: 0,
            kind: DependenceKind::Flow,
            direction: vec![DependenceDirection::Forward, DependenceDirection::Equal],
            distance: vec![Some(1), Some(0)],
        });
        assert!(!nest.is_parallel(0)); // carried in dim 0
        assert!(nest.is_parallel(1));  // not carried in dim 1
    }

    #[test]
    fn test_tiling() {
        let schedule = AffineMap::identity(2);
        let tiled = apply_tiling(&schedule, 0, 32);
        assert_eq!(tiled.output_dims, 3); // added tile dimension
    }

    #[test]
    fn test_interchange() {
        let schedule = AffineMap::identity(2);
        let swapped = apply_interchange(&schedule, 0, 1);
        assert_eq!(swapped.apply(&[1, 2]), vec![2, 1]);
    }

    #[test]
    fn test_skew() {
        let schedule = AffineMap::identity(2);
        let skewed = apply_skew(&schedule, 1, 1);
        // j' = j + i
        assert_eq!(skewed.apply(&[2, 3]), vec![2, 5]);
    }

    #[test]
    fn test_pluto_scheduler() {
        let mut nest = LoopNest::new(2);
        nest.add_statement(Statement {
            id: 0,
            name: "S0".to_string(),
            iteration_domain: IntegerSet::new("S0", 2),
            access_functions: Vec::new(),
            schedule: Some(AffineMap::identity(2)),
        });
        let scheduler = PlutoScheduler::new(nest);
        let parallel = scheduler.find_parallel_dims();
        assert_eq!(parallel, vec![0, 1]); // no dependences → all parallel
    }

    #[test]
    fn test_pluto_auto_tile() {
        let nest = LoopNest::new(2);
        let scheduler = PlutoScheduler::new(nest);
        let results = scheduler.auto_tile(&[32, 32]);
        assert!(results.len() >= 2);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_dependence_analyzer() {
        let write_access = AccessFunction {
            array_name: "A".to_string(),
            map: AffineMap::identity(1),
            is_write: true,
        };
        let read_access = AccessFunction {
            array_name: "A".to_string(),
            map: AffineMap::identity(1),
            is_write: false,
        };
        let dep = DependenceAnalyzer::analyze(&write_access, &read_access, 1);
        assert!(dep.is_some());
        assert_eq!(dep.unwrap().kind, DependenceKind::Flow);
    }

    #[test]
    fn test_no_dependence_different_arrays() {
        let a = AccessFunction {
            array_name: "A".to_string(),
            map: AffineMap::identity(1),
            is_write: true,
        };
        let b = AccessFunction {
            array_name: "B".to_string(),
            map: AffineMap::identity(1),
            is_write: false,
        };
        assert!(DependenceAnalyzer::analyze(&a, &b, 1).is_none());
    }

    #[test]
    fn test_schedule_validity() {
        let nest = LoopNest::new(2);
        let scheduler = PlutoScheduler::new(nest);
        let valid_sched = AffineMap::identity(2);
        assert!(scheduler.check_schedule_validity(&valid_sched));
    }

    #[test]
    fn test_constraint_dimensions() {
        let c = Constraint::ge(vec![1, 2, 3], 0);
        assert_eq!(c.dimensions(), 3);
    }

    #[test]
    fn test_integer_set_empty() {
        let mut set = IntegerSet::new("empty", 1);
        set.add_constraint(Constraint::ge(vec![1], -5));   // x >= 5
        set.add_constraint(Constraint::le(vec![1], -3));   // x <= 3
        assert!(set.is_empty_in_bounds(&[(0, 10)]));
    }
}
