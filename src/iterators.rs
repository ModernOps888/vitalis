//! Vitalis Iterator & Generator Protocol (v26)
//!
//! A lazy iterator protocol with generator (`yield`) support, defining the
//! Iterator trait, adapter combinators (map, filter, take, skip, zip, chain,
//! enumerate, flat_map, fold), and compiler transforms for generator-to-state-
//! machine lowering.
//!
//! # Architecture
//!
//! - **IteratorDef**: Defines a custom iterator with an `Item` type and `next` method
//! - **IteratorAdapter**: Lazy combinators that wrap an inner iterator source
//! - **GeneratorDef**: A function that uses `yield` to produce values lazily
//! - **GeneratorStateMachine**: The lowered state machine form of a generator
//! - **IteratorPipeline**: Chains multiple adapters for fusion optimization
//!
//! # Examples
//!
//! ```text
//! // Vitalis source (future syntax):
//! fn fibonacci() yields i64 {
//!     let a = 0;
//!     let b = 1;
//!     loop {
//!         yield a;
//!         let tmp = a + b;
//!         a = b;
//!         b = tmp;
//!     }
//! }
//!
//! let first_10 = fibonacci() |> take(10) |> collect();
//! let evens = range(0, 100) |> filter(|x| x % 2 == 0) |> map(|x| x * x);
//! ```

use std::collections::HashMap;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Iterator Protocol Core
// ═══════════════════════════════════════════════════════════════════════

/// The value type yielded by iterators and generators.
#[derive(Debug, Clone, PartialEq)]
pub enum IterValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<IterValue>),
    Pair(Box<IterValue>, Box<IterValue>),
    Void,
}

impl fmt::Display for IterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IterValue::Int(n) => write!(f, "{}", n),
            IterValue::Float(v) => write!(f, "{}", v),
            IterValue::Bool(b) => write!(f, "{}", b),
            IterValue::Str(s) => write!(f, "\"{}\"", s),
            IterValue::List(elems) => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
            IterValue::Pair(a, b) => write!(f, "({}, {})", a, b),
            IterValue::Void => write!(f, "void"),
        }
    }
}

impl IterValue {
    /// Type name for diagnostics.
    pub fn type_name(&self) -> &'static str {
        match self {
            IterValue::Int(_) => "i64",
            IterValue::Float(_) => "f64",
            IterValue::Bool(_) => "bool",
            IterValue::Str(_) => "str",
            IterValue::List(_) => "list",
            IterValue::Pair(_, _) => "pair",
            IterValue::Void => "void",
        }
    }
}

/// Item type descriptor for the iterator protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemType {
    I64,
    F64,
    Bool,
    Str,
    Pair(Box<ItemType>, Box<ItemType>),
    Any,
}

impl fmt::Display for ItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ItemType::I64 => write!(f, "i64"),
            ItemType::F64 => write!(f, "f64"),
            ItemType::Bool => write!(f, "bool"),
            ItemType::Str => write!(f, "str"),
            ItemType::Pair(a, b) => write!(f, "({}, {})", a, b),
            ItemType::Any => write!(f, "any"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Iterator Definition & Source
// ═══════════════════════════════════════════════════════════════════════

/// An iterator definition — declares the item type and a `next` strategy.
#[derive(Debug, Clone)]
pub struct IteratorDef {
    pub name: String,
    pub item_type: ItemType,
}

/// Where an iterator gets its elements.
#[derive(Debug, Clone)]
pub enum IteratorSource {
    /// A numeric range: start..end (exclusive) with optional step.
    Range {
        start: i64,
        end: i64,
        step: i64,
    },
    /// An in-memory collection of values.
    Collection(Vec<IterValue>),
    /// A named generator function.
    Generator(String),
    /// An empty iterator.
    Empty,
    /// Repeat a single value N times.
    Repeat {
        value: IterValue,
        count: Option<usize>,
    },
    /// Generate values on the fly with a counter.
    Counter {
        start: i64,
        step: i64,
    },
}

// ═══════════════════════════════════════════════════════════════════════
//  Iterator Adapters (Lazy Combinators)
// ═══════════════════════════════════════════════════════════════════════

/// A lazy adapter that transforms an iterator pipeline.
#[derive(Debug, Clone)]
pub enum IteratorAdapter {
    /// Map each element through a named transform.
    Map { func_name: String },
    /// Keep elements that satisfy a named predicate.
    Filter { predicate_name: String },
    /// Take at most N elements.
    Take { count: usize },
    /// Skip the first N elements.
    Skip { count: usize },
    /// Zip with another source.
    Zip { other: IteratorSource },
    /// Chain another source after this one.
    Chain { other: IteratorSource },
    /// Emit (index, value) pairs.
    Enumerate,
    /// Flat-map: map then flatten one level.
    FlatMap { func_name: String },
    /// Take elements while predicate holds.
    TakeWhile { predicate_name: String },
    /// Skip elements while predicate holds.
    SkipWhile { predicate_name: String },
    /// Inspect each element (for debugging) without consuming.
    Inspect { func_name: String },
    /// Deduplicate consecutive equal elements.
    Dedup,
    /// Reverse the iterator (requires finite).
    Reverse,
}

impl fmt::Display for IteratorAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IteratorAdapter::Map { func_name } => write!(f, ".map({})", func_name),
            IteratorAdapter::Filter { predicate_name } => {
                write!(f, ".filter({})", predicate_name)
            }
            IteratorAdapter::Take { count } => write!(f, ".take({})", count),
            IteratorAdapter::Skip { count } => write!(f, ".skip({})", count),
            IteratorAdapter::Zip { .. } => write!(f, ".zip(...)"),
            IteratorAdapter::Chain { .. } => write!(f, ".chain(...)"),
            IteratorAdapter::Enumerate => write!(f, ".enumerate()"),
            IteratorAdapter::FlatMap { func_name } => write!(f, ".flat_map({})", func_name),
            IteratorAdapter::TakeWhile { predicate_name } => {
                write!(f, ".take_while({})", predicate_name)
            }
            IteratorAdapter::SkipWhile { predicate_name } => {
                write!(f, ".skip_while({})", predicate_name)
            }
            IteratorAdapter::Inspect { func_name } => write!(f, ".inspect({})", func_name),
            IteratorAdapter::Dedup => write!(f, ".dedup()"),
            IteratorAdapter::Reverse => write!(f, ".reverse()"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Iterator Pipeline
// ═══════════════════════════════════════════════════════════════════════

/// A complete iterator expression: source + chain of adapters + terminal.
#[derive(Debug, Clone)]
pub struct IteratorPipeline {
    pub source: IteratorSource,
    pub adapters: Vec<IteratorAdapter>,
}

impl IteratorPipeline {
    pub fn new(source: IteratorSource) -> Self {
        Self {
            source,
            adapters: Vec::new(),
        }
    }

    pub fn map(mut self, func_name: &str) -> Self {
        self.adapters.push(IteratorAdapter::Map {
            func_name: func_name.to_string(),
        });
        self
    }

    pub fn filter(mut self, pred_name: &str) -> Self {
        self.adapters.push(IteratorAdapter::Filter {
            predicate_name: pred_name.to_string(),
        });
        self
    }

    pub fn take(mut self, n: usize) -> Self {
        self.adapters.push(IteratorAdapter::Take { count: n });
        self
    }

    pub fn skip(mut self, n: usize) -> Self {
        self.adapters.push(IteratorAdapter::Skip { count: n });
        self
    }

    pub fn enumerate(mut self) -> Self {
        self.adapters.push(IteratorAdapter::Enumerate);
        self
    }

    pub fn zip(mut self, other: IteratorSource) -> Self {
        self.adapters.push(IteratorAdapter::Zip { other });
        self
    }

    pub fn chain(mut self, other: IteratorSource) -> Self {
        self.adapters.push(IteratorAdapter::Chain { other });
        self
    }

    pub fn dedup(mut self) -> Self {
        self.adapters.push(IteratorAdapter::Dedup);
        self
    }

    pub fn reverse(mut self) -> Self {
        self.adapters.push(IteratorAdapter::Reverse);
        self
    }

    /// Number of adapters in this pipeline.
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Generator Definition
// ═══════════════════════════════════════════════════════════════════════

/// The state of a running generator.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorState {
    /// The generator yielded a value and can be resumed.
    Yielded(IterValue),
    /// The generator completed, optionally returning a final value.
    Completed(Option<IterValue>),
}

impl fmt::Display for GeneratorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneratorState::Yielded(v) => write!(f, "Yielded({})", v),
            GeneratorState::Completed(Some(v)) => write!(f, "Completed({})", v),
            GeneratorState::Completed(None) => write!(f, "Completed"),
        }
    }
}

/// A yield point in a generator — position where execution can suspend.
#[derive(Debug, Clone)]
pub struct YieldPoint {
    pub state_id: u32,
    pub has_value: bool,
}

/// A generator definition before lowering to a state machine.
#[derive(Debug, Clone)]
pub struct GeneratorDef {
    pub name: String,
    pub yield_type: ItemType,
    pub return_type: Option<ItemType>,
    pub yield_points: Vec<YieldPoint>,
    pub is_infinite: bool,
}

impl GeneratorDef {
    /// Number of yield points in this generator.
    pub fn yield_count(&self) -> usize {
        self.yield_points.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  State Machine (Lowered Generator)
// ═══════════════════════════════════════════════════════════════════════

/// Transition from one state machine state to another.
#[derive(Debug, Clone, PartialEq)]
pub enum SMTransition {
    /// Yield a value and go to next state.
    Yield { next_state: u32 },
    /// Transition unconditionally to another state.
    Goto(u32),
    /// Complete the generator.
    Return,
    /// Conditional branch.
    Branch {
        true_state: u32,
        false_state: u32,
    },
}

impl fmt::Display for SMTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SMTransition::Yield { next_state } => write!(f, "yield → S{}", next_state),
            SMTransition::Goto(s) => write!(f, "goto S{}", s),
            SMTransition::Return => write!(f, "return"),
            SMTransition::Branch {
                true_state,
                false_state,
            } => write!(f, "branch ? S{} : S{}", true_state, false_state),
        }
    }
}

/// A single state in the lowered generator state machine.
#[derive(Debug, Clone)]
pub struct SMState {
    pub id: u32,
    pub label: String,
    pub transition: SMTransition,
}

/// The complete lowered state machine for a generator.
#[derive(Debug, Clone)]
pub struct GeneratorStateMachine {
    pub name: String,
    pub states: Vec<SMState>,
    pub initial_state: u32,
    pub yield_type: ItemType,
    pub locals: Vec<(String, ItemType)>,
}

impl GeneratorStateMachine {
    /// Number of states.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Check if a state ID exists.
    pub fn has_state(&self, id: u32) -> bool {
        self.states.iter().any(|s| s.id == id)
    }

    /// Get all yield transitions.
    pub fn yield_states(&self) -> Vec<u32> {
        self.states
            .iter()
            .filter_map(|s| {
                if let SMTransition::Yield { .. } = &s.transition {
                    Some(s.id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the terminal (return) states.
    pub fn return_states(&self) -> Vec<u32> {
        self.states
            .iter()
            .filter_map(|s| {
                if matches!(&s.transition, SMTransition::Return) {
                    Some(s.id)
                } else {
                    None
                }
            })
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Generator Transformer (AST → State Machine)
// ═══════════════════════════════════════════════════════════════════════

/// Transforms a generator definition into a state machine.
pub struct GeneratorTransformer {
    next_state_id: u32,
}

impl GeneratorTransformer {
    pub fn new() -> Self {
        Self { next_state_id: 0 }
    }

    fn alloc_state(&mut self) -> u32 {
        let id = self.next_state_id;
        self.next_state_id += 1;
        id
    }

    /// Lower a generator definition into a state machine.
    pub fn lower(&mut self, gen_def: &GeneratorDef) -> GeneratorStateMachine {
        let mut states = Vec::new();

        // Initial state
        let init_id = self.alloc_state();
        let first_yield_id = self.alloc_state();
        states.push(SMState {
            id: init_id,
            label: format!("{}_init", gen_def.name),
            transition: SMTransition::Goto(first_yield_id),
        });

        // Create a yield state for each yield point
        for (i, _yp) in gen_def.yield_points.iter().enumerate() {
            let yield_id = if i == 0 {
                first_yield_id
            } else {
                self.alloc_state()
            };
            let next_id = if i + 1 < gen_def.yield_points.len() {
                yield_id + 1
            } else if gen_def.is_infinite {
                first_yield_id // loop back
            } else {
                let ret_id = self.alloc_state();
                states.push(SMState {
                    id: ret_id,
                    label: format!("{}_done", gen_def.name),
                    transition: SMTransition::Return,
                });
                ret_id
            };
            if i == 0 {
                states.push(SMState {
                    id: yield_id,
                    label: format!("{}_yield_{}", gen_def.name, i),
                    transition: SMTransition::Yield {
                        next_state: next_id,
                    },
                });
            } else {
                states.push(SMState {
                    id: yield_id,
                    label: format!("{}_yield_{}", gen_def.name, i),
                    transition: SMTransition::Yield {
                        next_state: next_id,
                    },
                });
            }
        }

        // If no yield points, just return immediately
        if gen_def.yield_points.is_empty() {
            states.push(SMState {
                id: first_yield_id,
                label: format!("{}_done", gen_def.name),
                transition: SMTransition::Return,
            });
        }

        GeneratorStateMachine {
            name: gen_def.name.clone(),
            states,
            initial_state: init_id,
            yield_type: gen_def.yield_type.clone(),
            locals: Vec::new(),
        }
    }
}

impl Default for GeneratorTransformer {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Iterator Runtime (Eager Evaluation for Testing)
// ═══════════════════════════════════════════════════════════════════════

/// Simple transform functions for testing. Maps a function name to an
/// `IterValue -> IterValue` transform.
pub type TransformFn = fn(&IterValue) -> IterValue;
pub type PredicateFn = fn(&IterValue) -> bool;

/// Registry of named transform/predicate functions for iterator evaluation.
pub struct FunctionRegistry {
    transforms: HashMap<String, TransformFn>,
    predicates: HashMap<String, PredicateFn>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
            predicates: HashMap::new(),
        }
    }

    pub fn register_transform(&mut self, name: &str, f: TransformFn) {
        self.transforms.insert(name.to_string(), f);
    }

    pub fn register_predicate(&mut self, name: &str, f: PredicateFn) {
        self.predicates.insert(name.to_string(), f);
    }

    pub fn get_transform(&self, name: &str) -> Option<&TransformFn> {
        self.transforms.get(name)
    }

    pub fn get_predicate(&self, name: &str) -> Option<&PredicateFn> {
        self.predicates.get(name)
    }
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Eagerly evaluate an iterator pipeline, collecting all results.
pub fn evaluate_pipeline(
    pipeline: &IteratorPipeline,
    registry: &FunctionRegistry,
) -> Result<Vec<IterValue>, IteratorError> {
    // Step 1: Materialize the source
    let mut items = materialize_source(&pipeline.source)?;

    // Step 2: Apply each adapter in order
    for adapter in &pipeline.adapters {
        items = apply_adapter(&items, adapter, registry)?;
    }

    Ok(items)
}

fn materialize_source(source: &IteratorSource) -> Result<Vec<IterValue>, IteratorError> {
    match source {
        IteratorSource::Range { start, end, step } => {
            if *step == 0 {
                return Err(IteratorError::InvalidStep);
            }
            let mut vals = Vec::new();
            let mut cur = *start;
            if *step > 0 {
                while cur < *end {
                    vals.push(IterValue::Int(cur));
                    cur += step;
                }
            } else {
                while cur > *end {
                    vals.push(IterValue::Int(cur));
                    cur += step;
                }
            }
            Ok(vals)
        }
        IteratorSource::Collection(items) => Ok(items.clone()),
        IteratorSource::Empty => Ok(Vec::new()),
        IteratorSource::Repeat { value, count } => {
            let n = count.unwrap_or(0);
            Ok(vec![value.clone(); n])
        }
        IteratorSource::Counter { start, step } => {
            // For eager evaluation, limit counter to a finite batch
            let mut vals = Vec::new();
            let mut cur = *start;
            for _ in 0..1000 {
                vals.push(IterValue::Int(cur));
                cur += step;
            }
            Ok(vals)
        }
        IteratorSource::Generator(_) => Err(IteratorError::GeneratorNotSupported),
    }
}

fn apply_adapter(
    items: &[IterValue],
    adapter: &IteratorAdapter,
    registry: &FunctionRegistry,
) -> Result<Vec<IterValue>, IteratorError> {
    match adapter {
        IteratorAdapter::Map { func_name } => {
            let f = registry
                .get_transform(func_name)
                .ok_or_else(|| IteratorError::UnknownFunction(func_name.clone()))?;
            Ok(items.iter().map(|v| f(v)).collect())
        }
        IteratorAdapter::Filter { predicate_name } => {
            let p = registry
                .get_predicate(predicate_name)
                .ok_or_else(|| IteratorError::UnknownFunction(predicate_name.clone()))?;
            Ok(items.iter().filter(|v| p(v)).cloned().collect())
        }
        IteratorAdapter::Take { count } => {
            Ok(items.iter().take(*count).cloned().collect())
        }
        IteratorAdapter::Skip { count } => {
            Ok(items.iter().skip(*count).cloned().collect())
        }
        IteratorAdapter::Enumerate => {
            Ok(items
                .iter()
                .enumerate()
                .map(|(i, v)| IterValue::Pair(Box::new(IterValue::Int(i as i64)), Box::new(v.clone())))
                .collect())
        }
        IteratorAdapter::Zip { other } => {
            let other_items = materialize_source(other)?;
            Ok(items
                .iter()
                .zip(other_items.iter())
                .map(|(a, b)| IterValue::Pair(Box::new(a.clone()), Box::new(b.clone())))
                .collect())
        }
        IteratorAdapter::Chain { other } => {
            let other_items = materialize_source(other)?;
            let mut result = items.to_vec();
            result.extend(other_items);
            Ok(result)
        }
        IteratorAdapter::Dedup => {
            let mut result = Vec::new();
            for item in items {
                if result.last() != Some(item) {
                    result.push(item.clone());
                }
            }
            Ok(result)
        }
        IteratorAdapter::Reverse => {
            let mut result = items.to_vec();
            result.reverse();
            Ok(result)
        }
        IteratorAdapter::TakeWhile { predicate_name } => {
            let p = registry
                .get_predicate(predicate_name)
                .ok_or_else(|| IteratorError::UnknownFunction(predicate_name.clone()))?;
            Ok(items.iter().take_while(|v| p(v)).cloned().collect())
        }
        IteratorAdapter::SkipWhile { predicate_name } => {
            let p = registry
                .get_predicate(predicate_name)
                .ok_or_else(|| IteratorError::UnknownFunction(predicate_name.clone()))?;
            Ok(items.iter().skip_while(|v| p(v)).cloned().collect())
        }
        IteratorAdapter::FlatMap { func_name } => {
            let f = registry
                .get_transform(func_name)
                .ok_or_else(|| IteratorError::UnknownFunction(func_name.clone()))?;
            let mut result = Vec::new();
            for item in items {
                let mapped = f(item);
                if let IterValue::List(inner) = mapped {
                    result.extend(inner);
                } else {
                    result.push(mapped);
                }
            }
            Ok(result)
        }
        IteratorAdapter::Inspect { func_name } => {
            let f = registry
                .get_transform(func_name)
                .ok_or_else(|| IteratorError::UnknownFunction(func_name.clone()))?;
            for item in items {
                let _ = f(item);
            }
            Ok(items.to_vec())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Terminal Operations (Consumers)
// ═══════════════════════════════════════════════════════════════════════

/// Collect an evaluated pipeline into a list value.
pub fn collect(items: &[IterValue]) -> IterValue {
    IterValue::List(items.to_vec())
}

/// Count items.
pub fn count(items: &[IterValue]) -> usize {
    items.len()
}

/// Sum all integer items.
pub fn sum_ints(items: &[IterValue]) -> i64 {
    items
        .iter()
        .filter_map(|v| {
            if let IterValue::Int(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .sum()
}

/// Find the first item matching a predicate name.
pub fn find_first(
    items: &[IterValue],
    predicate_name: &str,
    registry: &FunctionRegistry,
) -> Option<IterValue> {
    let p = registry.get_predicate(predicate_name)?;
    items.iter().find(|v| p(v)).cloned()
}

/// Check if any item matches a predicate.
pub fn any(
    items: &[IterValue],
    predicate_name: &str,
    registry: &FunctionRegistry,
) -> Option<bool> {
    let p = registry.get_predicate(predicate_name)?;
    Some(items.iter().any(|v| p(v)))
}

/// Check if all items match a predicate.
pub fn all(
    items: &[IterValue],
    predicate_name: &str,
    registry: &FunctionRegistry,
) -> Option<bool> {
    let p = registry.get_predicate(predicate_name)?;
    Some(items.iter().all(|v| p(v)))
}

/// Fold/reduce items with an accumulator.
pub fn fold(items: &[IterValue], init: IterValue, op: fn(&IterValue, &IterValue) -> IterValue) -> IterValue {
    items.iter().fold(init, |acc, v| op(&acc, v))
}

/// Get the first item.
pub fn first(items: &[IterValue]) -> Option<&IterValue> {
    items.first()
}

/// Get the last item.
pub fn last(items: &[IterValue]) -> Option<&IterValue> {
    items.last()
}

/// Get the nth item.
pub fn nth(items: &[IterValue], n: usize) -> Option<&IterValue> {
    items.get(n)
}

// ═══════════════════════════════════════════════════════════════════════
//  Errors
// ═══════════════════════════════════════════════════════════════════════

/// Errors from iterator/generator operations.
#[derive(Debug, Clone, PartialEq)]
pub enum IteratorError {
    /// An unknown function name was referenced.
    UnknownFunction(String),
    /// A generator was referenced but runtime doesn't support it in eager mode.
    GeneratorNotSupported,
    /// Invalid step (zero).
    InvalidStep,
    /// Iterator exhausted unexpectedly.
    Exhausted,
    /// Type mismatch in iterator operation.
    TypeMismatch { expected: String, got: String },
}

impl fmt::Display for IteratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IteratorError::UnknownFunction(name) => {
                write!(f, "unknown iterator function `{}`", name)
            }
            IteratorError::GeneratorNotSupported => {
                write!(f, "generator sources not supported in eager mode")
            }
            IteratorError::InvalidStep => write!(f, "iterator step cannot be zero"),
            IteratorError::Exhausted => write!(f, "iterator exhausted"),
            IteratorError::TypeMismatch { expected, got } => {
                write!(f, "iterator type mismatch: expected {}, got {}", expected, got)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Utility
// ═══════════════════════════════════════════════════════════════════════

/// Create a range source from start..end with step 1.
pub fn range(start: i64, end: i64) -> IteratorSource {
    IteratorSource::Range {
        start,
        end,
        step: 1,
    }
}

/// Create a range source with a custom step.
pub fn range_step(start: i64, end: i64, step: i64) -> IteratorSource {
    IteratorSource::Range { start, end, step }
}

/// Create a collection source from a vector of integers.
pub fn from_ints(values: &[i64]) -> IteratorSource {
    IteratorSource::Collection(values.iter().map(|v| IterValue::Int(*v)).collect())
}

/// Create an empty source.
pub fn empty() -> IteratorSource {
    IteratorSource::Empty
}

/// Create a repeat source.
pub fn repeat(value: IterValue, count: usize) -> IteratorSource {
    IteratorSource::Repeat {
        value,
        count: Some(count),
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── IterValue basics ───────────────────────────────────────────

    #[test]
    fn test_iter_value_display_int() {
        assert_eq!(IterValue::Int(42).to_string(), "42");
    }

    #[test]
    fn test_iter_value_display_float() {
        assert_eq!(IterValue::Float(3.14).to_string(), "3.14");
    }

    #[test]
    fn test_iter_value_display_str() {
        assert_eq!(IterValue::Str("hello".to_string()).to_string(), "\"hello\"");
    }

    #[test]
    fn test_iter_value_display_bool() {
        assert_eq!(IterValue::Bool(true).to_string(), "true");
    }

    #[test]
    fn test_iter_value_display_list() {
        let v = IterValue::List(vec![IterValue::Int(1), IterValue::Int(2)]);
        assert_eq!(v.to_string(), "[1, 2]");
    }

    #[test]
    fn test_iter_value_display_pair() {
        let v = IterValue::Pair(Box::new(IterValue::Int(0)), Box::new(IterValue::Str("x".to_string())));
        assert_eq!(v.to_string(), "(0, \"x\")");
    }

    #[test]
    fn test_iter_value_display_void() {
        assert_eq!(IterValue::Void.to_string(), "void");
    }

    #[test]
    fn test_iter_value_type_name() {
        assert_eq!(IterValue::Int(0).type_name(), "i64");
        assert_eq!(IterValue::Float(0.0).type_name(), "f64");
        assert_eq!(IterValue::Bool(false).type_name(), "bool");
        assert_eq!(IterValue::Str(String::new()).type_name(), "str");
        assert_eq!(IterValue::List(vec![]).type_name(), "list");
        assert_eq!(IterValue::Void.type_name(), "void");
    }

    // ── ItemType ───────────────────────────────────────────────────

    #[test]
    fn test_item_type_display() {
        assert_eq!(ItemType::I64.to_string(), "i64");
        assert_eq!(ItemType::Any.to_string(), "any");
        let pair = ItemType::Pair(Box::new(ItemType::I64), Box::new(ItemType::Str));
        assert_eq!(pair.to_string(), "(i64, str)");
    }

    // ── IteratorAdapter display ────────────────────────────────────

    #[test]
    fn test_adapter_display() {
        assert_eq!(
            IteratorAdapter::Map {
                func_name: "double".to_string()
            }
            .to_string(),
            ".map(double)"
        );
        assert_eq!(
            IteratorAdapter::Take { count: 5 }.to_string(),
            ".take(5)"
        );
        assert_eq!(IteratorAdapter::Enumerate.to_string(), ".enumerate()");
        assert_eq!(IteratorAdapter::Dedup.to_string(), ".dedup()");
        assert_eq!(IteratorAdapter::Reverse.to_string(), ".reverse()");
    }

    // ── Pipeline builder ───────────────────────────────────────────

    #[test]
    fn test_pipeline_builder() {
        let pipe = IteratorPipeline::new(range(0, 10))
            .filter("is_even")
            .map("double")
            .take(3);
        assert_eq!(pipe.adapter_count(), 3);
    }

    // ── GeneratorState display ─────────────────────────────────────

    #[test]
    fn test_generator_state_display() {
        let y = GeneratorState::Yielded(IterValue::Int(5));
        assert_eq!(y.to_string(), "Yielded(5)");

        let c = GeneratorState::Completed(None);
        assert_eq!(c.to_string(), "Completed");

        let c2 = GeneratorState::Completed(Some(IterValue::Int(42)));
        assert_eq!(c2.to_string(), "Completed(42)");
    }

    // ── SMTransition display ───────────────────────────────────────

    #[test]
    fn test_sm_transition_display() {
        assert_eq!(
            SMTransition::Yield { next_state: 2 }.to_string(),
            "yield → S2"
        );
        assert_eq!(SMTransition::Goto(3).to_string(), "goto S3");
        assert_eq!(SMTransition::Return.to_string(), "return");
        assert_eq!(
            SMTransition::Branch {
                true_state: 1,
                false_state: 2
            }
            .to_string(),
            "branch ? S1 : S2"
        );
    }

    // ── Generator transformer ──────────────────────────────────────

    #[test]
    fn test_generator_lower_simple() {
        let mut transformer = GeneratorTransformer::new();
        let gen_def = GeneratorDef {
            name: "counter".to_string(),
            yield_type: ItemType::I64,
            return_type: None,
            yield_points: vec![
                YieldPoint {
                    state_id: 0,
                    has_value: true,
                },
            ],
            is_infinite: false,
        };
        let sm = transformer.lower(&gen_def);
        assert_eq!(sm.name, "counter");
        assert!(sm.state_count() >= 2); // init + yield + done
        assert!(sm.has_state(sm.initial_state));
    }

    #[test]
    fn test_generator_lower_infinite() {
        let mut transformer = GeneratorTransformer::new();
        let gen_def = GeneratorDef {
            name: "forever".to_string(),
            yield_type: ItemType::I64,
            return_type: None,
            yield_points: vec![
                YieldPoint {
                    state_id: 0,
                    has_value: true,
                },
            ],
            is_infinite: true,
        };
        let sm = transformer.lower(&gen_def);
        let yield_states = sm.yield_states();
        assert!(!yield_states.is_empty());
        // Infinite generator should have no return states
        assert!(sm.return_states().is_empty());
    }

    #[test]
    fn test_generator_lower_multiple_yields() {
        let mut transformer = GeneratorTransformer::new();
        let gen_def = GeneratorDef {
            name: "multi".to_string(),
            yield_type: ItemType::I64,
            return_type: None,
            yield_points: vec![
                YieldPoint {
                    state_id: 0,
                    has_value: true,
                },
                YieldPoint {
                    state_id: 1,
                    has_value: true,
                },
                YieldPoint {
                    state_id: 2,
                    has_value: true,
                },
            ],
            is_infinite: false,
        };
        let sm = transformer.lower(&gen_def);
        assert!(sm.yield_states().len() >= 3);
    }

    #[test]
    fn test_generator_lower_no_yields() {
        let mut transformer = GeneratorTransformer::new();
        let gen_def = GeneratorDef {
            name: "empty_gen".to_string(),
            yield_type: ItemType::I64,
            return_type: None,
            yield_points: vec![],
            is_infinite: false,
        };
        let sm = transformer.lower(&gen_def);
        assert!(!sm.return_states().is_empty());
    }

    #[test]
    fn test_generator_yield_count() {
        let gen_def = GeneratorDef {
            name: "test".to_string(),
            yield_type: ItemType::I64,
            return_type: None,
            yield_points: vec![
                YieldPoint { state_id: 0, has_value: true },
                YieldPoint { state_id: 1, has_value: true },
            ],
            is_infinite: false,
        };
        assert_eq!(gen_def.yield_count(), 2);
    }

    // ── Range source evaluation ────────────────────────────────────

    #[test]
    fn test_evaluate_range() {
        let pipeline = IteratorPipeline::new(range(0, 5));
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], IterValue::Int(0));
        assert_eq!(result[4], IterValue::Int(4));
    }

    #[test]
    fn test_evaluate_range_step() {
        let pipeline = IteratorPipeline::new(range_step(0, 10, 2));
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result, vec![
            IterValue::Int(0),
            IterValue::Int(2),
            IterValue::Int(4),
            IterValue::Int(6),
            IterValue::Int(8),
        ]);
    }

    #[test]
    fn test_evaluate_range_negative_step() {
        let pipeline = IteratorPipeline::new(range_step(5, 0, -1));
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], IterValue::Int(5));
        assert_eq!(result[4], IterValue::Int(1));
    }

    #[test]
    fn test_evaluate_range_zero_step_error() {
        let pipeline = IteratorPipeline::new(range_step(0, 10, 0));
        let registry = FunctionRegistry::new();
        assert!(matches!(
            evaluate_pipeline(&pipeline, &registry),
            Err(IteratorError::InvalidStep)
        ));
    }

    // ── Collection source ──────────────────────────────────────────

    #[test]
    fn test_evaluate_collection() {
        let pipeline = IteratorPipeline::new(from_ints(&[10, 20, 30]));
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[1], IterValue::Int(20));
    }

    // ── Empty source ───────────────────────────────────────────────

    #[test]
    fn test_evaluate_empty() {
        let pipeline = IteratorPipeline::new(empty());
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert!(result.is_empty());
    }

    // ── Repeat source ──────────────────────────────────────────────

    #[test]
    fn test_evaluate_repeat() {
        let pipeline = IteratorPipeline::new(repeat(IterValue::Int(7), 3));
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result, vec![IterValue::Int(7), IterValue::Int(7), IterValue::Int(7)]);
    }

    // ── Take adapter ───────────────────────────────────────────────

    #[test]
    fn test_evaluate_take() {
        let pipeline = IteratorPipeline::new(range(0, 100)).take(3);
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result.len(), 3);
    }

    // ── Skip adapter ───────────────────────────────────────────────

    #[test]
    fn test_evaluate_skip() {
        let pipeline = IteratorPipeline::new(range(0, 5)).skip(2);
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result, vec![IterValue::Int(2), IterValue::Int(3), IterValue::Int(4)]);
    }

    // ── Map adapter ────────────────────────────────────────────────

    fn double_int(v: &IterValue) -> IterValue {
        if let IterValue::Int(n) = v {
            IterValue::Int(n * 2)
        } else {
            v.clone()
        }
    }

    #[test]
    fn test_evaluate_map() {
        let pipeline = IteratorPipeline::new(from_ints(&[1, 2, 3])).map("double");
        let mut registry = FunctionRegistry::new();
        registry.register_transform("double", double_int);
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result, vec![IterValue::Int(2), IterValue::Int(4), IterValue::Int(6)]);
    }

    // ── Filter adapter ─────────────────────────────────────────────

    fn is_even(v: &IterValue) -> bool {
        if let IterValue::Int(n) = v {
            n % 2 == 0
        } else {
            false
        }
    }

    #[test]
    fn test_evaluate_filter() {
        let pipeline = IteratorPipeline::new(range(0, 6)).filter("is_even");
        let mut registry = FunctionRegistry::new();
        registry.register_predicate("is_even", is_even);
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result, vec![IterValue::Int(0), IterValue::Int(2), IterValue::Int(4)]);
    }

    // ── Enumerate adapter ──────────────────────────────────────────

    #[test]
    fn test_evaluate_enumerate() {
        let pipeline = IteratorPipeline::new(from_ints(&[10, 20])).enumerate();
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            IterValue::Pair(Box::new(IterValue::Int(0)), Box::new(IterValue::Int(10)))
        );
        assert_eq!(
            result[1],
            IterValue::Pair(Box::new(IterValue::Int(1)), Box::new(IterValue::Int(20)))
        );
    }

    // ── Zip adapter ────────────────────────────────────────────────

    #[test]
    fn test_evaluate_zip() {
        let pipeline =
            IteratorPipeline::new(from_ints(&[1, 2, 3])).zip(from_ints(&[10, 20, 30]));
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0],
            IterValue::Pair(Box::new(IterValue::Int(1)), Box::new(IterValue::Int(10)))
        );
    }

    // ── Chain adapter ──────────────────────────────────────────────

    #[test]
    fn test_evaluate_chain() {
        let pipeline =
            IteratorPipeline::new(from_ints(&[1, 2])).chain(from_ints(&[3, 4]));
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(
            result,
            vec![IterValue::Int(1), IterValue::Int(2), IterValue::Int(3), IterValue::Int(4)]
        );
    }

    // ── Dedup adapter ──────────────────────────────────────────────

    #[test]
    fn test_evaluate_dedup() {
        let pipeline = IteratorPipeline::new(from_ints(&[1, 1, 2, 2, 3, 1, 1])).dedup();
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(
            result,
            vec![IterValue::Int(1), IterValue::Int(2), IterValue::Int(3), IterValue::Int(1)]
        );
    }

    // ── Reverse adapter ────────────────────────────────────────────

    #[test]
    fn test_evaluate_reverse() {
        let pipeline = IteratorPipeline::new(from_ints(&[1, 2, 3])).reverse();
        let registry = FunctionRegistry::new();
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(
            result,
            vec![IterValue::Int(3), IterValue::Int(2), IterValue::Int(1)]
        );
    }

    // ── Combined pipeline ──────────────────────────────────────────

    #[test]
    fn test_evaluate_filter_map_take() {
        let pipeline = IteratorPipeline::new(range(0, 20))
            .filter("is_even")
            .map("double")
            .take(3);
        let mut registry = FunctionRegistry::new();
        registry.register_predicate("is_even", is_even);
        registry.register_transform("double", double_int);
        let result = evaluate_pipeline(&pipeline, &registry).unwrap();
        assert_eq!(result, vec![IterValue::Int(0), IterValue::Int(4), IterValue::Int(8)]);
    }

    // ── Terminal operations ────────────────────────────────────────

    #[test]
    fn test_collect_terminal() {
        let items = vec![IterValue::Int(1), IterValue::Int(2)];
        let collected = collect(&items);
        assert_eq!(
            collected,
            IterValue::List(vec![IterValue::Int(1), IterValue::Int(2)])
        );
    }

    #[test]
    fn test_count_terminal() {
        let items = vec![IterValue::Int(1), IterValue::Int(2), IterValue::Int(3)];
        assert_eq!(count(&items), 3);
    }

    #[test]
    fn test_sum_ints_terminal() {
        let items = vec![IterValue::Int(1), IterValue::Int(2), IterValue::Int(3)];
        assert_eq!(sum_ints(&items), 6);
    }

    #[test]
    fn test_first_last_nth() {
        let items = vec![IterValue::Int(10), IterValue::Int(20), IterValue::Int(30)];
        assert_eq!(first(&items), Some(&IterValue::Int(10)));
        assert_eq!(last(&items), Some(&IterValue::Int(30)));
        assert_eq!(nth(&items, 1), Some(&IterValue::Int(20)));
        assert_eq!(nth(&items, 5), None);
    }

    #[test]
    fn test_any_all() {
        let items = vec![IterValue::Int(2), IterValue::Int(4), IterValue::Int(6)];
        let mut registry = FunctionRegistry::new();
        registry.register_predicate("is_even", is_even);
        assert_eq!(all(&items, "is_even", &registry), Some(true));
        assert_eq!(any(&items, "is_even", &registry), Some(true));
    }

    #[test]
    fn test_find_first() {
        let items = vec![IterValue::Int(1), IterValue::Int(2), IterValue::Int(3)];
        let mut registry = FunctionRegistry::new();
        registry.register_predicate("is_even", is_even);
        let found = find_first(&items, "is_even", &registry);
        assert_eq!(found, Some(IterValue::Int(2)));
    }

    #[test]
    fn test_fold_sum() {
        let items = vec![IterValue::Int(1), IterValue::Int(2), IterValue::Int(3)];
        let result = fold(&items, IterValue::Int(0), |acc, v| {
            if let (IterValue::Int(a), IterValue::Int(b)) = (acc, v) {
                IterValue::Int(a + b)
            } else {
                acc.clone()
            }
        });
        assert_eq!(result, IterValue::Int(6));
    }

    // ── Error cases ────────────────────────────────────────────────

    #[test]
    fn test_unknown_function_error() {
        let pipeline = IteratorPipeline::new(range(0, 3)).map("nonexistent");
        let registry = FunctionRegistry::new();
        assert!(matches!(
            evaluate_pipeline(&pipeline, &registry),
            Err(IteratorError::UnknownFunction(_))
        ));
    }

    #[test]
    fn test_generator_source_error() {
        let pipeline = IteratorPipeline::new(IteratorSource::Generator("gen".to_string()));
        let registry = FunctionRegistry::new();
        assert!(matches!(
            evaluate_pipeline(&pipeline, &registry),
            Err(IteratorError::GeneratorNotSupported)
        ));
    }

    // ── IteratorError display ──────────────────────────────────────

    #[test]
    fn test_iterator_error_display() {
        let e = IteratorError::UnknownFunction("f".to_string());
        assert!(e.to_string().contains("f"));

        let e = IteratorError::GeneratorNotSupported;
        assert!(e.to_string().contains("generator"));

        let e = IteratorError::InvalidStep;
        assert!(e.to_string().contains("zero"));

        let e = IteratorError::Exhausted;
        assert!(e.to_string().contains("exhausted"));

        let e = IteratorError::TypeMismatch {
            expected: "i64".to_string(),
            got: "str".to_string(),
        };
        assert!(e.to_string().contains("i64"));
    }

    // ── Utility functions ──────────────────────────────────────────

    #[test]
    fn test_range_helper() {
        let src = range(0, 3);
        if let IteratorSource::Range { start, end, step } = src {
            assert_eq!(start, 0);
            assert_eq!(end, 3);
            assert_eq!(step, 1);
        } else {
            panic!("expected Range source");
        }
    }

    #[test]
    fn test_from_ints_helper() {
        let src = from_ints(&[1, 2, 3]);
        if let IteratorSource::Collection(items) = src {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected Collection source");
        }
    }

    #[test]
    fn test_empty_helper() {
        let src = empty();
        assert!(matches!(src, IteratorSource::Empty));
    }

    #[test]
    fn test_repeat_helper() {
        let src = repeat(IterValue::Int(5), 3);
        if let IteratorSource::Repeat { value, count } = src {
            assert_eq!(value, IterValue::Int(5));
            assert_eq!(count, Some(3));
        } else {
            panic!("expected Repeat source");
        }
    }
}
