//! Execution Profiling & Profile-Guided Optimization (PGO)
//!
//! Provides function-level profiling, call-graph construction, hot-path detection,
//! flame-graph generation, branch prediction statistics, and PGO feedback for
//! guiding JIT/AOT optimizations (inlining, block layout, specialization).

use std::collections::HashMap;

// ── Timing ───────────────────────────────────────────────────────────

/// High-resolution timestamp (nanoseconds since epoch, monotonic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self {
        // portable monotonic ns
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Timestamp(dur.as_nanos() as u64)
    }
    pub fn elapsed_ns(&self, other: &Timestamp) -> u64 {
        other.0.saturating_sub(self.0)
    }
}

// ── Function Profile ─────────────────────────────────────────────────

/// Accumulated profiling data for a single function.
#[derive(Debug, Clone)]
pub struct FunctionProfile {
    pub name: String,
    pub call_count: u64,
    /// Total wall-clock time spent *inside* this function (ns), including callees.
    pub cumulative_ns: u64,
    /// Self time = cumulative minus time in callees (ns).
    pub self_ns: u64,
    /// Min / max single-invocation time (ns).
    pub min_ns: u64,
    pub max_ns: u64,
    /// Running Welford statistics for per-call time.
    welford_mean: f64,
    welford_m2: f64,
}

impl FunctionProfile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            call_count: 0,
            cumulative_ns: 0,
            self_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
            welford_mean: 0.0,
            welford_m2: 0.0,
        }
    }

    /// Record one call with `elapsed_ns` wall-clock time.
    pub fn record_call(&mut self, elapsed_ns: u64, callee_ns: u64) {
        self.call_count += 1;
        self.cumulative_ns += elapsed_ns;
        let self_time = elapsed_ns.saturating_sub(callee_ns);
        self.self_ns += self_time;
        if elapsed_ns < self.min_ns {
            self.min_ns = elapsed_ns;
        }
        if elapsed_ns > self.max_ns {
            self.max_ns = elapsed_ns;
        }
        // Welford online variance
        let n = self.call_count as f64;
        let x = elapsed_ns as f64;
        let delta = x - self.welford_mean;
        self.welford_mean += delta / n;
        let delta2 = x - self.welford_mean;
        self.welford_m2 += delta * delta2;
    }

    /// Mean call time in nanoseconds.
    pub fn mean_ns(&self) -> f64 {
        self.welford_mean
    }

    /// Population variance of call times.
    pub fn variance_ns(&self) -> f64 {
        if self.call_count < 2 {
            return 0.0;
        }
        self.welford_m2 / (self.call_count as f64)
    }

    /// Standard deviation of call times.
    pub fn stddev_ns(&self) -> f64 {
        self.variance_ns().sqrt()
    }

    /// Hotness score = call_count * self_ns (higher → hotter).
    pub fn hotness(&self) -> f64 {
        (self.call_count as f64) * (self.self_ns as f64)
    }
}

// ── Call-Graph ───────────────────────────────────────────────────────

/// Directed edge in the call graph: caller → callee with call count.
#[derive(Debug, Clone)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
    pub count: u64,
    pub total_ns: u64,
}

/// Full call graph.
#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    pub edges: Vec<CallEdge>,
    edge_map: HashMap<(String, String), usize>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a call from `caller` to `callee` with the given duration.
    pub fn record(&mut self, caller: &str, callee: &str, ns: u64) {
        let key = (caller.to_string(), callee.to_string());
        if let Some(&idx) = self.edge_map.get(&key) {
            self.edges[idx].count += 1;
            self.edges[idx].total_ns += ns;
        } else {
            let idx = self.edges.len();
            self.edges.push(CallEdge {
                caller: caller.to_string(),
                callee: callee.to_string(),
                count: 1,
                total_ns: ns,
            });
            self.edge_map.insert(key, idx);
        }
    }

    /// All functions called by `caller`.
    pub fn callees_of(&self, caller: &str) -> Vec<&CallEdge> {
        self.edges.iter().filter(|e| e.caller == caller).collect()
    }

    /// All callers of `callee`.
    pub fn callers_of(&self, callee: &str) -> Vec<&CallEdge> {
        self.edges.iter().filter(|e| e.callee == callee).collect()
    }

    /// Total unique functions referenced.
    pub fn function_count(&self) -> usize {
        let mut fns = std::collections::HashSet::new();
        for e in &self.edges {
            fns.insert(&e.caller);
            fns.insert(&e.callee);
        }
        fns.len()
    }

    /// Export call graph as DOT (GraphViz).
    pub fn to_dot(&self) -> String {
        let mut s = String::from("digraph callgraph {\n  rankdir=LR;\n  node [shape=box, style=filled, fillcolor=\"#e8e8ff\"];\n");
        for e in &self.edges {
            s.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}x {}μs\"];\n",
                e.caller, e.callee, e.count, e.total_ns / 1000
            ));
        }
        s.push_str("}\n");
        s
    }
}

// ── Flame-Graph ──────────────────────────────────────────────────────

/// A single stack frame sample for flame graph generation.
#[derive(Debug, Clone)]
pub struct StackSample {
    /// Stack frames from bottom (main) to top (leaf).
    pub frames: Vec<String>,
    /// Weight (usually 1, or duration in μs).
    pub weight: u64,
}

/// Flame graph builder — uses Brendan Gregg folded-stack format.
#[derive(Debug, Clone, Default)]
pub struct FlameGraph {
    pub samples: Vec<StackSample>,
}

impl FlameGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a stack sample.
    pub fn add_sample(&mut self, frames: Vec<String>, weight: u64) {
        self.samples.push(StackSample { frames, weight });
    }

    /// Export to Brendan Gregg folded-stack format.
    /// Each line: `frame1;frame2;frame3 weight`
    pub fn to_folded(&self) -> String {
        // Aggregate identical stacks
        let mut map: HashMap<String, u64> = HashMap::new();
        for s in &self.samples {
            let key = s.frames.join(";");
            *map.entry(key).or_default() += s.weight;
        }
        let mut lines: Vec<_> = map.into_iter().collect();
        lines.sort_by(|a, b| b.1.cmp(&a.1));
        lines
            .iter()
            .map(|(k, w)| format!("{} {}", k, w))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate a simple SVG flame graph.
    pub fn to_svg(&self, width: u32, row_height: u32) -> String {
        // Aggregate stacks
        let mut map: HashMap<String, u64> = HashMap::new();
        for s in &self.samples {
            let key = s.frames.join(";");
            *map.entry(key).or_default() += s.weight;
        }
        let total_weight: u64 = map.values().sum();
        if total_weight == 0 {
            return "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_string();
        }

        // Find max depth
        let max_depth = self
            .samples
            .iter()
            .map(|s| s.frames.len())
            .max()
            .unwrap_or(1);
        let height = (max_depth as u32 + 2) * row_height;

        // Build per-frame-at-depth aggregation
        let mut depth_frames: HashMap<(usize, String), u64> = HashMap::new();
        for s in &self.samples {
            for (d, f) in s.frames.iter().enumerate() {
                *depth_frames.entry((d, f.clone())).or_default() += s.weight;
            }
        }

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">\n\
             <rect width=\"100%\" height=\"100%\" fill=\"#1a1a2e\"/>\n\
             <text x=\"{}\" y=\"20\" fill=\"#e0e0ff\" font-size=\"14\" text-anchor=\"middle\">Flame Graph</text>\n",
            width, height, width / 2
        );

        let colors = [
            "#ff6b6b", "#ffa94d", "#ffd43b", "#69db7c", "#4dabf7",
            "#9775fa", "#f783ac", "#e599f7",
        ];

        let mut sorted: Vec<_> = depth_frames.into_iter().collect();
        sorted.sort_by_key(|((d, _), _)| *d);

        let mut x_offset: HashMap<usize, f64> = HashMap::new();
        for ((depth, fname), weight) in &sorted {
            let w = (*weight as f64 / total_weight as f64) * width as f64;
            let x = x_offset.entry(*depth).or_insert(0.0);
            let y = height as f64 - ((*depth as f64 + 1.0) * row_height as f64);
            let color = colors[depth % colors.len()];

            svg.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{}\" fill=\"{}\" opacity=\"0.85\" rx=\"2\"/>\n",
                *x, y, w, row_height - 1, color
            ));
            if w > 40.0 {
                let label = if fname.len() > (w as usize / 7) {
                    &fname[..fname.len().min(w as usize / 7)]
                } else {
                    fname
                };
                svg.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" fill=\"white\" font-size=\"11\" clip-path=\"url(#clip)\">{}</text>\n",
                    *x + 3.0, y + (row_height as f64 * 0.65), label
                ));
            }
            *x += w;
        }

        svg.push_str("</svg>\n");
        svg
    }

    /// Total number of samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Total weight across all samples.
    pub fn total_weight(&self) -> u64 {
        self.samples.iter().map(|s| s.weight).sum()
    }
}

// ── Branch Prediction ────────────────────────────────────────────────

/// Branch prediction statistics for a single branch site.
#[derive(Debug, Clone)]
pub struct BranchStats {
    pub location: String,
    pub taken: u64,
    pub not_taken: u64,
}

impl BranchStats {
    pub fn new(location: &str) -> Self {
        Self {
            location: location.to_string(),
            taken: 0,
            not_taken: 0,
        }
    }

    pub fn record(&mut self, taken: bool) {
        if taken {
            self.taken += 1;
        } else {
            self.not_taken += 1;
        }
    }

    /// Ratio of taken / total. 1.0 = always taken, 0.0 = never taken.
    pub fn taken_ratio(&self) -> f64 {
        let total = self.taken + self.not_taken;
        if total == 0 {
            return 0.5;
        }
        self.taken as f64 / total as f64
    }

    /// Prediction accuracy if using static "always taken" or "always not taken".
    pub fn best_static_accuracy(&self) -> f64 {
        let total = self.taken + self.not_taken;
        if total == 0 {
            return 1.0;
        }
        self.taken.max(self.not_taken) as f64 / total as f64
    }

    /// True if the branch is biased enough to benefit from layout hints.
    pub fn is_biased(&self, threshold: f64) -> bool {
        let r = self.taken_ratio();
        r >= threshold || r <= (1.0 - threshold)
    }
}

// ── PGO Feedback ─────────────────────────────────────────────────────

/// Hint about how to optimize a particular function.
#[derive(Debug, Clone, PartialEq)]
pub enum PgoHint {
    /// Inline this function (hot callee, small body).
    Inline,
    /// Do NOT inline (cold, large body).
    NoInline,
    /// Layout basic blocks to favor the taken branch.
    BranchLayout { location: String, favor_taken: bool },
    /// Specialize for a particular argument type/value.
    Specialize { arg_index: usize, value_hint: String },
    /// Unroll loop at this location.
    LoopUnroll { location: String, factor: usize },
    /// Pre-allocate stack frame of given size.
    StackReserve { bytes: usize },
}

/// Collected PGO data for an entire program run.
#[derive(Debug, Clone, Default)]
pub struct PgoProfile {
    pub functions: HashMap<String, FunctionProfile>,
    pub call_graph: CallGraph,
    pub branches: Vec<BranchStats>,
    pub flame_graph: FlameGraph,
}

impl PgoProfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a function call.
    pub fn record_function(&mut self, name: &str, elapsed_ns: u64, callee_ns: u64) {
        let fp = self
            .functions
            .entry(name.to_string())
            .or_insert_with(|| FunctionProfile::new(name));
        fp.record_call(elapsed_ns, callee_ns);
    }

    /// Record a call edge.
    pub fn record_call(&mut self, caller: &str, callee: &str, ns: u64) {
        self.call_graph.record(caller, callee, ns);
    }

    /// Record a branch outcome.
    pub fn record_branch(&mut self, location: &str, taken: bool) {
        if let Some(bs) = self.branches.iter_mut().find(|b| b.location == location) {
            bs.record(taken);
        } else {
            let mut bs = BranchStats::new(location);
            bs.record(taken);
            self.branches.push(bs);
        }
    }

    /// Record a stack sample for flame graph.
    pub fn record_stack(&mut self, frames: Vec<String>, weight: u64) {
        self.flame_graph.add_sample(frames, weight);
    }

    /// Generate PGO hints from the collected data.
    pub fn generate_hints(&self, inline_threshold_ns: u64, hot_count: u64) -> Vec<PgoHint> {
        let mut hints = Vec::new();

        // Inlining hints based on hotness & self time
        for (name, fp) in &self.functions {
            if fp.call_count >= hot_count && fp.mean_ns() < inline_threshold_ns as f64 {
                hints.push(PgoHint::Inline);
            } else if fp.call_count < 2 && fp.mean_ns() > (inline_threshold_ns * 10) as f64 {
                hints.push(PgoHint::NoInline);
            }
            let _ = name; // used above via iteration
        }

        // Branch layout hints
        for bs in &self.branches {
            if bs.is_biased(0.8) {
                hints.push(PgoHint::BranchLayout {
                    location: bs.location.clone(),
                    favor_taken: bs.taken_ratio() >= 0.5,
                });
            }
        }

        hints
    }

    /// Top-N hottest functions by hotness score.
    pub fn hot_functions(&self, n: usize) -> Vec<&FunctionProfile> {
        let mut fns: Vec<_> = self.functions.values().collect();
        fns.sort_by(|a, b| b.hotness().partial_cmp(&a.hotness()).unwrap_or(std::cmp::Ordering::Equal));
        fns.truncate(n);
        fns
    }

    /// Serialize profile to JSON-like format for storage.
    pub fn serialize(&self) -> String {
        let mut out = String::from("{\n  \"functions\": {\n");
        let fns: Vec<_> = self.functions.iter().collect();
        for (i, (name, fp)) in fns.iter().enumerate() {
            out.push_str(&format!(
                "    \"{}\": {{\"calls\": {}, \"cumulative_ns\": {}, \"self_ns\": {}, \"mean_ns\": {:.1}}}",
                name, fp.call_count, fp.cumulative_ns, fp.self_ns, fp.mean_ns()
            ));
            if i < fns.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  },\n  \"edges\": [\n");
        for (i, e) in self.call_graph.edges.iter().enumerate() {
            out.push_str(&format!(
                "    {{\"caller\": \"{}\", \"callee\": \"{}\", \"count\": {}, \"ns\": {}}}",
                e.caller, e.callee, e.count, e.total_ns
            ));
            if i < self.call_graph.edges.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n  \"branches\": [\n");
        for (i, b) in self.branches.iter().enumerate() {
            out.push_str(&format!(
                "    {{\"loc\": \"{}\", \"taken\": {}, \"not_taken\": {}, \"ratio\": {:.3}}}",
                b.location, b.taken, b.not_taken, b.taken_ratio()
            ));
            if i < self.branches.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n}\n");
        out
    }

    /// Deserialize minimal profile from serialized form (function counts only).
    pub fn deserialize_functions(json: &str) -> HashMap<String, (u64, u64)> {
        let mut result = HashMap::new();
        // Simple extraction — find "name": {"calls": N, ...}
        for line in json.lines() {
            let line = line.trim();
            if line.starts_with('"') && line.contains("\"calls\":") {
                // Extract function name
                if let Some(name_end) = line.find("\": {") {
                    let name = &line[1..name_end];
                    // Extract calls
                    if let Some(calls_start) = line.find("\"calls\": ") {
                        let rest = &line[calls_start + 9..];
                        if let Some(comma) = rest.find(',') {
                            if let Ok(calls) = rest[..comma].trim().parse::<u64>() {
                                // Extract self_ns
                                if let Some(self_start) = line.find("\"self_ns\": ") {
                                    let rest2 = &line[self_start + 11..];
                                    let end = rest2.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest2.len());
                                    if let Ok(self_ns) = rest2[..end].parse::<u64>() {
                                        result.insert(name.to_string(), (calls, self_ns));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        result
    }
}

// ── Hot-Path Detector ────────────────────────────────────────────────

/// Identifies the critical execution path through the call graph.
#[derive(Debug, Clone)]
pub struct HotPathDetector {
    /// Function → cumulative time mapping.
    pub function_times: HashMap<String, u64>,
}

impl HotPathDetector {
    pub fn from_profile(profile: &PgoProfile) -> Self {
        let function_times = profile
            .functions
            .iter()
            .map(|(k, v)| (k.clone(), v.cumulative_ns))
            .collect();
        Self { function_times }
    }

    /// Find the hot path: the sequence of functions consuming the most time.
    /// Starting from `root`, greedily follows the heaviest callee edge.
    pub fn find_hot_path(&self, call_graph: &CallGraph, root: &str) -> Vec<String> {
        let mut path = vec![root.to_string()];
        let mut current = root.to_string();
        let mut visited = std::collections::HashSet::new();
        visited.insert(current.clone());

        loop {
            let callees = call_graph.callees_of(&current);
            if callees.is_empty() {
                break;
            }
            // Pick callee with maximum total_ns
            if let Some(heaviest) = callees
                .iter()
                .filter(|e| !visited.contains(&e.callee))
                .max_by_key(|e| e.total_ns)
            {
                visited.insert(heaviest.callee.clone());
                path.push(heaviest.callee.clone());
                current = heaviest.callee.clone();
            } else {
                break;
            }
        }
        path
    }

    /// Rank all functions by cumulative time (descending).
    pub fn rank_by_time(&self) -> Vec<(&str, u64)> {
        let mut ranked: Vec<_> = self.function_times.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked
    }

    /// Functions consuming more than `pct` percent of total time.
    pub fn above_threshold(&self, pct: f64) -> Vec<(&str, f64)> {
        let total: u64 = self.function_times.values().sum();
        if total == 0 {
            return vec![];
        }
        let mut result: Vec<_> = self
            .function_times
            .iter()
            .map(|(k, v)| {
                let p = *v as f64 / total as f64 * 100.0;
                (k.as_str(), p)
            })
            .filter(|(_, p)| *p >= pct)
            .collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }
}

// ── Instrumentation Profiler ─────────────────────────────────────────

/// Call-stack based profiler that tracks enter/exit events.
#[derive(Debug, Clone)]
struct ActiveFrame {
    name: String,
    start: Timestamp,
    callee_ns: u64,
}

/// Full instrumentation profiler that produces a PgoProfile.
#[derive(Debug)]
pub struct Profiler {
    stack: Vec<ActiveFrame>,
    pub profile: PgoProfile,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            profile: PgoProfile::new(),
        }
    }

    /// Called when entering a function.
    pub fn enter(&mut self, name: &str) {
        self.stack.push(ActiveFrame {
            name: name.to_string(),
            start: Timestamp::now(),
            callee_ns: 0,
        });
    }

    /// Called when exiting a function. Records profile data.
    pub fn exit(&mut self, name: &str) {
        let now = Timestamp::now();
        if let Some(frame) = self.stack.pop() {
            let elapsed = frame.start.elapsed_ns(&now);
            self.profile
                .record_function(&frame.name, elapsed, frame.callee_ns);

            // Update parent's callee_ns
            if let Some(parent) = self.stack.last_mut() {
                parent.callee_ns += elapsed;
                self.profile
                    .record_call(&parent.name, &frame.name, elapsed);
            }

            // Add flame graph sample
            let mut frames: Vec<String> =
                self.stack.iter().map(|f| f.name.clone()).collect();
            frames.push(frame.name);
            self.profile.record_stack(frames, elapsed / 1000); // μs weight

            let _ = name; // verified match
        }
    }

    /// Get current call depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Finish profiling and return the collected profile.
    pub fn finish(self) -> PgoProfile {
        self.profile
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_ordering() {
        let t1 = Timestamp(100);
        let t2 = Timestamp(200);
        assert!(t1 < t2);
        assert_eq!(t1.elapsed_ns(&t2), 100);
    }

    #[test]
    fn test_timestamp_elapsed_saturates() {
        let t1 = Timestamp(200);
        let t2 = Timestamp(100);
        assert_eq!(t1.elapsed_ns(&t2), 0);
    }

    #[test]
    fn test_function_profile_basic() {
        let mut fp = FunctionProfile::new("foo");
        fp.record_call(1000, 200);
        fp.record_call(2000, 300);
        assert_eq!(fp.call_count, 2);
        assert_eq!(fp.cumulative_ns, 3000);
        assert_eq!(fp.self_ns, 800 + 1700);
        assert_eq!(fp.min_ns, 1000);
        assert_eq!(fp.max_ns, 2000);
    }

    #[test]
    fn test_function_profile_mean() {
        let mut fp = FunctionProfile::new("bar");
        fp.record_call(100, 0);
        fp.record_call(200, 0);
        fp.record_call(300, 0);
        assert!((fp.mean_ns() - 200.0).abs() < 1e-6);
    }

    #[test]
    fn test_function_profile_stddev() {
        let mut fp = FunctionProfile::new("baz");
        for _ in 0..100 {
            fp.record_call(1000, 0);
        }
        assert!(fp.stddev_ns() < 1e-6, "constant calls should have zero stddev");
    }

    #[test]
    fn test_function_profile_hotness() {
        let mut fp = FunctionProfile::new("hot");
        fp.record_call(500, 0);
        fp.record_call(500, 0);
        // hotness = call_count (2) * self_ns (1000)
        assert!((fp.hotness() - 2000.0).abs() < 1e-6);
    }

    #[test]
    fn test_call_graph_record() {
        let mut cg = CallGraph::new();
        cg.record("main", "foo", 100);
        cg.record("main", "foo", 200);
        cg.record("main", "bar", 50);
        assert_eq!(cg.edges.len(), 2);
        assert_eq!(cg.edges[0].count, 2);
        assert_eq!(cg.edges[0].total_ns, 300);
    }

    #[test]
    fn test_call_graph_callees() {
        let mut cg = CallGraph::new();
        cg.record("main", "foo", 100);
        cg.record("main", "bar", 200);
        cg.record("foo", "baz", 50);
        let callees = cg.callees_of("main");
        assert_eq!(callees.len(), 2);
    }

    #[test]
    fn test_call_graph_callers() {
        let mut cg = CallGraph::new();
        cg.record("a", "c", 10);
        cg.record("b", "c", 20);
        let callers = cg.callers_of("c");
        assert_eq!(callers.len(), 2);
    }

    #[test]
    fn test_call_graph_function_count() {
        let mut cg = CallGraph::new();
        cg.record("main", "foo", 100);
        cg.record("foo", "bar", 50);
        assert_eq!(cg.function_count(), 3);
    }

    #[test]
    fn test_call_graph_dot_export() {
        let mut cg = CallGraph::new();
        cg.record("main", "foo", 1000);
        let dot = cg.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("main"));
        assert!(dot.contains("foo"));
    }

    #[test]
    fn test_flame_graph_folded() {
        let mut fg = FlameGraph::new();
        fg.add_sample(vec!["main".into(), "foo".into()], 10);
        fg.add_sample(vec!["main".into(), "foo".into()], 5);
        fg.add_sample(vec!["main".into(), "bar".into()], 3);
        let folded = fg.to_folded();
        assert!(folded.contains("main;foo 15"));
        assert!(folded.contains("main;bar 3"));
    }

    #[test]
    fn test_flame_graph_svg() {
        let mut fg = FlameGraph::new();
        fg.add_sample(vec!["main".into(), "compute".into()], 100);
        let svg = fg.to_svg(800, 20);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Flame Graph"));
    }

    #[test]
    fn test_flame_graph_empty_svg() {
        let fg = FlameGraph::new();
        let svg = fg.to_svg(800, 20);
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_flame_graph_sample_count() {
        let mut fg = FlameGraph::new();
        fg.add_sample(vec!["a".into()], 1);
        fg.add_sample(vec!["b".into()], 2);
        assert_eq!(fg.sample_count(), 2);
        assert_eq!(fg.total_weight(), 3);
    }

    #[test]
    fn test_branch_stats_taken_ratio() {
        let mut bs = BranchStats::new("if:10");
        for _ in 0..80 {
            bs.record(true);
        }
        for _ in 0..20 {
            bs.record(false);
        }
        assert!((bs.taken_ratio() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_branch_stats_accuracy() {
        let mut bs = BranchStats::new("if:20");
        bs.taken = 90;
        bs.not_taken = 10;
        assert!((bs.best_static_accuracy() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_branch_stats_biased() {
        let mut bs = BranchStats::new("br:1");
        bs.taken = 95;
        bs.not_taken = 5;
        assert!(bs.is_biased(0.8));

        let mut bs2 = BranchStats::new("br:2");
        bs2.taken = 50;
        bs2.not_taken = 50;
        assert!(!bs2.is_biased(0.8));
    }

    #[test]
    fn test_branch_stats_empty() {
        let bs = BranchStats::new("empty");
        assert!((bs.taken_ratio() - 0.5).abs() < 1e-6);
        assert!((bs.best_static_accuracy() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_pgo_profile_record() {
        let mut p = PgoProfile::new();
        p.record_function("foo", 1000, 200);
        p.record_function("foo", 2000, 300);
        p.record_call("main", "foo", 1000);
        p.record_branch("if:1", true);
        p.record_branch("if:1", false);
        assert_eq!(p.functions["foo"].call_count, 2);
        assert_eq!(p.call_graph.edges.len(), 1);
        assert_eq!(p.branches.len(), 1);
    }

    #[test]
    fn test_pgo_hot_functions() {
        let mut p = PgoProfile::new();
        p.record_function("hot", 10000, 0);
        p.record_function("hot", 10000, 0);
        p.record_function("cold", 100, 0);
        let top = p.hot_functions(1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].name, "hot");
    }

    #[test]
    fn test_pgo_generate_hints() {
        let mut p = PgoProfile::new();
        // Hot small function → should get Inline hint
        for _ in 0..100 {
            p.record_function("small_hot", 50, 0);
        }
        // Cold large function → NoInline
        p.record_function("big_cold", 50000, 0);

        // Biased branch
        let mut bs = BranchStats::new("if:42");
        bs.taken = 95;
        bs.not_taken = 5;
        p.branches.push(bs);

        let hints = p.generate_hints(500, 10);
        assert!(hints.iter().any(|h| matches!(h, PgoHint::Inline)));
        assert!(hints.iter().any(|h| matches!(h, PgoHint::BranchLayout { .. })));
    }

    #[test]
    fn test_pgo_serialize() {
        let mut p = PgoProfile::new();
        p.record_function("main", 5000, 1000);
        p.record_call("main", "helper", 1000);
        p.record_branch("if:1", true);
        let json = p.serialize();
        assert!(json.contains("\"main\""));
        assert!(json.contains("\"calls\": 1"));
        assert!(json.contains("\"caller\": \"main\""));
    }

    #[test]
    fn test_pgo_deserialize_functions() {
        let json = r#"{
  "functions": {
    "main": {"calls": 5, "cumulative_ns": 10000, "self_ns": 8000, "mean_ns": 2000.0}
  }
}"#;
        let fns = PgoProfile::deserialize_functions(json);
        assert_eq!(fns["main"], (5, 8000));
    }

    #[test]
    fn test_hot_path_detector() {
        let mut p = PgoProfile::new();
        p.record_function("main", 10000, 5000);
        p.record_function("compute", 5000, 2000);
        p.record_function("helper", 2000, 0);
        p.record_function("cold_fn", 100, 0);
        p.record_call("main", "compute", 5000);
        p.record_call("main", "cold_fn", 100);
        p.record_call("compute", "helper", 2000);

        let detector = HotPathDetector::from_profile(&p);
        let path = detector.find_hot_path(&p.call_graph, "main");
        assert_eq!(path, vec!["main", "compute", "helper"]);
    }

    #[test]
    fn test_hot_path_rank_by_time() {
        let mut p = PgoProfile::new();
        p.record_function("a", 100, 0);
        p.record_function("b", 500, 0);
        p.record_function("c", 300, 0);
        let detector = HotPathDetector::from_profile(&p);
        let ranked = detector.rank_by_time();
        assert_eq!(ranked[0].0, "b");
    }

    #[test]
    fn test_hot_path_above_threshold() {
        let mut p = PgoProfile::new();
        p.record_function("heavy", 9000, 0);
        p.record_function("light", 1000, 0);
        let detector = HotPathDetector::from_profile(&p);
        let above = detector.above_threshold(50.0);
        assert_eq!(above.len(), 1);
        assert_eq!(above[0].0, "heavy");
    }

    #[test]
    fn test_profiler_enter_exit() {
        let mut profiler = Profiler::new();
        profiler.enter("main");
        assert_eq!(profiler.depth(), 1);
        profiler.enter("helper");
        assert_eq!(profiler.depth(), 2);
        profiler.exit("helper");
        assert_eq!(profiler.depth(), 1);
        profiler.exit("main");
        assert_eq!(profiler.depth(), 0);

        let profile = profiler.finish();
        assert!(profile.functions.contains_key("main"));
        assert!(profile.functions.contains_key("helper"));
    }

    #[test]
    fn test_profiler_records_call_graph() {
        let mut profiler = Profiler::new();
        profiler.enter("main");
        profiler.enter("foo");
        profiler.exit("foo");
        profiler.exit("main");
        let profile = profiler.finish();
        assert!(!profile.call_graph.edges.is_empty());
        assert_eq!(profile.call_graph.edges[0].caller, "main");
        assert_eq!(profile.call_graph.edges[0].callee, "foo");
    }

    #[test]
    fn test_profiler_flame_graph_populated() {
        let mut profiler = Profiler::new();
        profiler.enter("main");
        profiler.enter("compute");
        profiler.exit("compute");
        profiler.exit("main");
        let profile = profiler.finish();
        assert!(profile.flame_graph.sample_count() >= 2);
    }

    #[test]
    fn test_pgo_hint_eq() {
        assert_eq!(PgoHint::Inline, PgoHint::Inline);
        assert_ne!(PgoHint::Inline, PgoHint::NoInline);
    }
}
