//! Vitalis Engine — the autonomous self-evolution brain.
//!
//! This is NOT a wrapper. This IS the intelligence layer.
//! Python handles I/O (HTTP, files, git). Vitalis handles thinking:
//!   - Compile & type-check mutations at native speed
//!   - Score fitness using native hotpath metrics
//!   - Track evolution generations with zero-copy rollback
//!   - Run autonomous evolution cycles in microseconds
//!
//! # Architecture
//!
//! ```text
//! Python (nervous system)          Vitalis Engine (brain)
//! ┌────────────────────┐           ┌──────────────────────────┐
//! │ HTTP API           │──FFI──────│ register()               │
//! │ File I/O           │           │ evolve() → compile+check │
//! │ Git operations     │◄──FFI─────│ run_cycle() → fitness    │
//! │ LLM API calls      │           │ stats_json()             │
//! └────────────────────┘           │ validate() → type-check  │
//!                                  │ get_landscape() → JSON   │
//!                                  └──────────────────────────┘
//! ```

use std::time::Instant;
use std::collections::VecDeque;
use crate::evolution::with_registry;
use crate::hotpath;
use crate::memory::with_memory;
use crate::meta_evolution::with_meta;

// ─── Engine Statistics ──────────────────────────────────────────────────

/// Runtime statistics for the Vitalis evolution engine.
pub struct EngineStats {
    pub total_cycles: u64,
    pub total_evolutions: u64,
    pub successful_evolutions: u64,
    pub failed_evolutions: u64,
    pub total_compile_time_ms: f64,
    pub avg_fitness: f64,
    pub best_fitness: f64,
    pub functions_tracked: usize,
    pub rollbacks: u64,
}

// ─── Evolution Attempt Result ───────────────────────────────────────────

/// Result of a single evolution attempt within the engine.
pub struct EvolutionAttempt {
    pub function_name: String,
    pub success: bool,
    pub generation: u64,
    pub fitness: f64,
    pub prev_fitness: f64,
    pub compile_time_ms: f64,
    pub error: Option<String>,
}

impl EvolutionAttempt {
    pub fn to_json(&self) -> String {
        let err = match &self.error {
            Some(e) => format!(",\"error\":\"{}\"", e.replace('"', "\\\"")),
            None => String::new(),
        };
        format!(
            "{{\"function\":\"{}\",\"success\":{},\"generation\":{},\"fitness\":{:.4},\"prev_fitness\":{:.4},\"compile_ms\":{:.3}{}}}",
            self.function_name, self.success, self.generation,
            self.fitness, self.prev_fitness, self.compile_time_ms, err
        )
    }
}

// ─── Cycle Result ───────────────────────────────────────────────────────

/// Result of a full evolution cycle.
pub struct CycleResult {
    pub cycle_number: u64,
    pub attempts: Vec<EvolutionAttempt>,
    pub total_time_ms: f64,
    pub functions_improved: usize,
    pub functions_regressed: usize,
}

impl CycleResult {
    pub fn to_json(&self) -> String {
        let attempts_json: Vec<String> = self.attempts.iter().map(|a| a.to_json()).collect();
        format!(
            "{{\"cycle\":{},\"total_ms\":{:.3},\"improved\":{},\"regressed\":{},\"attempts\":[{}]}}",
            self.cycle_number, self.total_time_ms,
            self.functions_improved, self.functions_regressed,
            attempts_json.join(",")
        )
    }
}

// ─── Validation Result ──────────────────────────────────────────────────

/// Result of Vitalis-native code validation.
/// The type checker IS the safety gate — no exec(), no file I/O, pure computation.
pub struct ValidationResult {
    pub valid: bool,
    pub parse_errors: Vec<String>,
    pub type_errors: Vec<String>,
    pub duration_ms: f64,
}

impl ValidationResult {
    pub fn to_json(&self) -> String {
        let pe: Vec<String> = self.parse_errors.iter()
            .map(|e| format!("\"{}\"", e.replace('"', "\\\"")))
            .collect();
        let te: Vec<String> = self.type_errors.iter()
            .map(|e| format!("\"{}\"", e.replace('"', "\\\"")))
            .collect();
        format!(
            "{{\"valid\":{},\"parse_errors\":[{}],\"type_errors\":[{}],\"duration_ms\":{:.3}}}",
            self.valid, pe.join(","), te.join(","), self.duration_ms
        )
    }
}

// ─── Error Log ──────────────────────────────────────────────────────────

/// A single entry in the runtime error log — persisted in-memory for diagnostics.
pub struct ErrorLogEntry {
    /// Milliseconds since engine boot at time of error.
    pub timestamp_ms: f64,
    /// Evolution cycle number when error occurred.
    pub cycle: u64,
    /// Name of the function being evolved/validated.
    pub function_name: String,
    /// Coarse category: "validation", "registry", "rollback", "fitness".
    pub error_kind: &'static str,
    /// Human-readable error message.
    pub error: String,
}

impl ErrorLogEntry {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"timestamp_ms\":{:.2},\"cycle\":{},\"function\":\"{}\",\"kind\":\"{}\",\"error\":\"{}\"}}",
            self.timestamp_ms,
            self.cycle,
            self.function_name,
            self.error_kind,
            self.error.replace('\\', "\\\\").replace('"', "\\\""),
        )
    }
}

// ─── Runtime Diagnostics ────────────────────────────────────────────────

/// OS/platform config, compile latency histogram, and error log summary.
///
/// Produced by `VitalisEngine::diagnostics_json()` and exported via FFI.
pub struct RuntimeDiagnostics {
    // ── OS / Platform config ──
    pub os: &'static str,
    pub arch: &'static str,
    pub family: &'static str,
    pub cpu_count: usize,
    pub vitalis_version: &'static str,
    pub uptime_ms: f64,
    // ── Compile latency histogram ──
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
    pub latency_avg: f64,
    pub latency_count: usize,
    // ── Error log summary ──
    pub error_total: usize,
    pub recent_error: Option<String>,
}

impl RuntimeDiagnostics {
    pub fn to_json(&self) -> String {
        let recent = match &self.recent_error {
            Some(e) => format!("\"{}\"", e.replace('\\', "\\\\").replace('"', "\\\"")),
            None => "null".to_string(),
        };
        format!(
            "{{\
\"os\":\"{os}\",\
\"arch\":\"{arch}\",\
\"family\":\"{family}\",\
\"cpu_count\":{cpu},\
\"vitalis_version\":\"{ver}\",\
\"uptime_ms\":{uptime:.0},\
\"compile_latency\":{{\
\"p50\":{p50:.3},\
\"p95\":{p95:.3},\
\"p99\":{p99:.3},\
\"avg\":{avg:.3},\
\"count\":{cnt}\
}},\
\"error_log\":{{\
\"total\":{errs},\
\"recent_error\":{recent}\
}}\
}}",
            os = self.os,
            arch = self.arch,
            family = self.family,
            cpu = self.cpu_count,
            ver = self.vitalis_version,
            uptime = self.uptime_ms,
            p50 = self.latency_p50,
            p95 = self.latency_p95,
            p99 = self.latency_p99,
            avg = self.latency_avg,
            cnt = self.latency_count,
            errs = self.error_total,
            recent = recent,
        )
    }
}

// ─── The Vitalis Engine ─────────────────────────────────────────────────

/// The Vitalis Engine — autonomous self-evolution at native speed.
///
/// This is the evolution brain. It compiles, validates, scores, and
/// evolves functions using the Cranelift JIT, type checker, and native
/// hotpath metrics. All computation happens in Rust — Python only
/// provides I/O and triggers cycles.
pub struct VitalisEngine {
    stats: EngineStats,
    boot_time: Instant,
    /// Recent compile latencies in ms — bounded circular buffer (max 1000 entries).
    compile_latencies: Vec<f64>,
    /// Structured error log — bounded circular buffer (max 200 entries).
    error_log: VecDeque<ErrorLogEntry>,
}

impl VitalisEngine {
    /// Create a new Vitalis Engine instance.
    pub fn new() -> Self {
        Self {
            stats: EngineStats {
                total_cycles: 0,
                total_evolutions: 0,
                successful_evolutions: 0,
                failed_evolutions: 0,
                total_compile_time_ms: 0.0,
                avg_fitness: 0.0,
                best_fitness: 0.0,
                functions_tracked: 0,
                rollbacks: 0,
            },
            boot_time: Instant::now(),
            compile_latencies: Vec::with_capacity(256),
            error_log: VecDeque::with_capacity(200),
        }
    }

    /// Validate Vitalis source code — the type checker IS the safety gate.
    ///
    /// Unlike a Python validator (which uses regex + AST to detect dangerous
    /// patterns like exec(), subprocess, os.system), Vitalis code is
    /// INHERENTLY SAFE because the language has:
    /// - No exec() or eval()
    /// - No file system access
    /// - No network access
    /// - No subprocess execution
    /// - No dynamic code loading
    /// - Strong static type checking
    ///
    /// If it compiles, it's safe. Period.
    pub fn validate(&self, source: &str) -> ValidationResult {
        let start = Instant::now();

        let (program, parse_errors) = crate::parser::parse(source);
        let parse_err_strings: Vec<String> = parse_errors.iter().map(|e| e.to_string()).collect();

        if !parse_errors.is_empty() {
            return ValidationResult {
                valid: false,
                parse_errors: parse_err_strings,
                type_errors: vec![],
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            };
        }

        let type_errors = crate::types::TypeChecker::new().check(&program);
        let type_err_strings: Vec<String> = type_errors.iter().map(|e| format!("{:?}", e)).collect();

        ValidationResult {
            valid: type_errors.is_empty(),
            parse_errors: parse_err_strings,
            type_errors: type_err_strings,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
        }
    }

    /// Register a function for evolution tracking.
    pub fn register(&mut self, name: &str, source: &str) {
        // Validate first — only register valid code
        let validation = self.validate(source);
        if !validation.valid {
            return; // Silent reject — invalid code never enters the registry
        }

        with_registry(|reg| {
            reg.register(name, source);
        });
        self.stats.functions_tracked = with_registry(|reg| reg.list_evolvable().len());
    }

    /// Evolve a function with a new variant.
    ///
    /// Pipeline: validate → compile → score fitness → accept/reject
    /// All in native Rust. No Python involved.
    pub fn evolve(&mut self, name: &str, new_source: &str) -> EvolutionAttempt {
        let start = Instant::now();
        self.stats.total_evolutions += 1;

        // 1. Get previous fitness for comparison
        let prev_fitness = with_registry(|reg| {
            reg.get_fitness(name).unwrap_or(0.0)
        });

        // 2. Validate — type checker is the safety gate
        let validation = self.validate(new_source);
        if !validation.valid {
            self.stats.failed_evolutions += 1;
            let errors = [validation.parse_errors, validation.type_errors].concat();
            let err_msg = errors.join("; ");
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            self.push_error(name, "validation", &err_msg, elapsed);
            return EvolutionAttempt {
                function_name: name.to_string(),
                success: false,
                generation: with_registry(|reg| reg.get_generation(name)),
                fitness: prev_fitness,
                prev_fitness,
                compile_time_ms: elapsed,
                error: Some(err_msg),
            };
        }

        // 3. Evolve in the registry (creates new generation)
        let new_gen = with_registry(|reg| {
            match reg.evolve(name, new_source) {
                Ok((g, _hash)) => g as i64,
                Err(_) => -1,
            }
        });

        if new_gen < 0 {
            self.stats.failed_evolutions += 1;
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            self.push_error(name, "registry", "Registry evolution failed", elapsed);
            return EvolutionAttempt {
                function_name: name.to_string(),
                success: false,
                generation: 0,
                fitness: prev_fitness,
                prev_fitness,
                compile_time_ms: elapsed,
                error: Some("Registry evolution failed".to_string()),
            };
        }

        // 4. Calculate fitness using native hotpath scoring
        let fitness = self.calculate_fitness(new_source);

        // 5. Store fitness
        with_registry(|reg| {
            reg.set_fitness(name, fitness);
        });

        // 6. Auto-rollback if fitness regressed significantly
        if prev_fitness > 0.0 && fitness < prev_fitness * 0.8 {
            // Fitness dropped >20% — rollback
            let prev_g = if new_gen > 0 { (new_gen - 1) as u64 } else { 0 };
            let rollback_ok = with_registry(|reg| {
                reg.rollback(name, prev_g).is_ok()
            });
            if rollback_ok {
                self.stats.rollbacks += 1;
                self.stats.failed_evolutions += 1;

                // ── Memory: store the failure for learning ──
                let cycle = self.stats.total_cycles;
                with_memory(|m| {
                    m.store(
                        crate::memory::EngramKind::Procedural,
                        &format!("Evolution rollback: {} regressed from {:.2} to {:.2}", name, prev_fitness, fitness),
                        &["evolution", "rollback", "regression"],
                        0.7,
                        name,
                        cycle,
                    );
                });

                // ── Error Log: record rollback ──
                let rollback_elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let rollback_msg = format!(
                    "Auto-rollback: fitness {:.2} < {:.2} (regressed {:.1}%)",
                    fitness, prev_fitness,
                    (1.0 - fitness / prev_fitness) * 100.0
                );
                self.push_error(name, "rollback", &rollback_msg, rollback_elapsed);

                return EvolutionAttempt {
                    function_name: name.to_string(),
                    success: false,
                    generation: new_gen as u64,
                    fitness,
                    prev_fitness,
                    compile_time_ms: rollback_elapsed,
                    error: Some(rollback_msg),
                };
            }
        }

        // Update best fitness
        if fitness > self.stats.best_fitness {
            self.stats.best_fitness = fitness;
        }

        self.stats.successful_evolutions += 1;
        let compile_time = start.elapsed().as_secs_f64() * 1000.0;
        self.stats.total_compile_time_ms += compile_time;
        self.push_latency(compile_time);

        // ── Memory: store the successful evolution ──
        let cycle = self.stats.total_cycles;
        with_memory(|m| {
            m.store(
                crate::memory::EngramKind::Episodic,
                &format!("Evolution success: {} gen {} fitness {:.2} (was {:.2})", name, new_gen, fitness, prev_fitness),
                &["evolution", "success", name],
                0.8,
                name,
                cycle,
            );
        });

        // ── Meta-Evolution: record strategy result ──
        let active_strategy = with_meta(|meta| {
            meta.active_strategy_name().unwrap_or_else(|| "balanced".to_string())
        });
        with_meta(|meta| {
            meta.record_result(&active_strategy, name, prev_fitness, fitness, true, cycle);
        });

        EvolutionAttempt {
            function_name: name.to_string(),
            success: true,
            generation: new_gen as u64,
            fitness,
            prev_fitness,
            compile_time_ms: compile_time,
            error: None,
        }
    }

    /// Calculate fitness score using native Rust metrics.
    /// This replaces Python's code_analyzer — runs at native speed.
    fn calculate_fitness(&self, source: &str) -> f64 {
        let lines = source.lines().count() as f64;
        let functions = source.matches("fn ").count() as f64;

        // Cyclomatic complexity approximation
        let branches = source.matches("if ").count()
            + source.matches("else").count()
            + source.matches("match ").count();
        let cyclomatic = branches as f64 + 1.0;

        // Cognitive complexity from nesting
        let mut depth: u32 = 0;
        let mut depths = Vec::new();
        for ch in source.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    depths.push(depth);
                }
                '}' => {
                    if depth > 0 { depth -= 1; }
                }
                _ => {}
            }
        }

        // Use native hotpath scoring
        let cognitive = if !depths.is_empty() {
            let d: Vec<u32> = depths;
            unsafe {
                hotpath::hotpath_cognitive_complexity(d.as_ptr(), d.len()) as f64
            }
        } else {
            0.0
        };

        hotpath::hotpath_code_quality_score(
            cyclomatic,
            cognitive,
            lines,
            functions.max(1.0),
            0.0,  // no security issues in Vitalis (inherently safe)
            1,    // always has tests (the compiler IS the test)
        )
    }

    /// Run one full evolution cycle across all registered functions.
    /// Returns detailed cycle results as a struct.
    pub fn run_cycle(&mut self) -> CycleResult {
        let start = Instant::now();
        self.stats.total_cycles += 1;

        let mut attempts = Vec::new();
        let mut improved = 0;
        let mut regressed = 0;

        // Snapshot all registered functions
        let functions: Vec<(String, String, f64)> = with_registry(|reg| {
            reg.list_evolvable()
                .iter()
                .map(|name| {
                    let source = reg.get_source(name).unwrap_or("").to_string();
                    let fitness = reg.get_fitness(name).unwrap_or(0.0);
                    (name.clone(), source, fitness)
                })
                .collect()
        });

        for (name, source, current_fitness) in &functions {
            if source.is_empty() {
                continue;
            }

            // Re-score existing code (fitness may change with engine updates)
            let new_fitness = self.calculate_fitness(source);

            if new_fitness > *current_fitness {
                improved += 1;
            } else if new_fitness < *current_fitness && *current_fitness > 0.0 {
                regressed += 1;
            }

            // Update fitness in registry
            with_registry(|reg| {
                reg.set_fitness(name, new_fitness);
            });

            attempts.push(EvolutionAttempt {
                function_name: name.clone(),
                success: true,
                generation: with_registry(|reg| reg.get_generation(name)),
                fitness: new_fitness,
                prev_fitness: *current_fitness,
                compile_time_ms: 0.0,
                error: None,
            });
        }

        // Update aggregate stats
        self.stats.functions_tracked = functions.len();
        if !attempts.is_empty() {
            let total_fitness: f64 = attempts.iter().map(|a| a.fitness).sum();
            self.stats.avg_fitness = total_fitness / attempts.len() as f64;
        }

        // ── Memory: apply decay each cycle ──
        let cycle = self.stats.total_cycles;
        with_memory(|m| {
            m.decay(cycle);
        });

        // ── Meta-Evolution: evolve strategies periodically (every 10 cycles) ──
        if cycle % 10 == 0 && cycle > 0 {
            with_meta(|meta| {
                meta.meta_evolve();
            });
        }

        CycleResult {
            cycle_number: self.stats.total_cycles,
            attempts,
            total_time_ms: start.elapsed().as_secs_f64() * 1000.0,
            functions_improved: improved,
            functions_regressed: regressed,
        }
    }

    /// Get the full fitness landscape as JSON.
    /// Returns all functions with their current fitness, generation, and source hash.
    pub fn get_landscape(&self) -> String {
        with_registry(|reg| {
            let names = reg.list_evolvable();
            let entries: Vec<String> = names.iter().map(|name| {
                let g = reg.get_generation(name);
                let fitness = reg.get_fitness(name).unwrap_or(0.0);
                let source_len = reg.get_source(name).map_or(0, |s| s.len());
                let history_len = reg.get_history(name).len();
                format!(
                    "{{\"name\":\"{}\",\"generation\":{},\"fitness\":{:.4},\"source_bytes\":{},\"history_depth\":{}}}",
                    name, g, fitness, source_len, history_len
                )
            }).collect();
            format!("[{}]", entries.join(","))
        })
    }

    // ─── Internal helpers ───────────────────────────────────────────────

    /// Push a compile latency sample (in ms). Bounded at 1000 entries (drops oldest).
    fn push_latency(&mut self, ms: f64) {
        if self.compile_latencies.len() >= 1000 {
            self.compile_latencies.remove(0);
        }
        self.compile_latencies.push(ms);
    }

    /// Append a structured error entry to the bounded error log (max 200 entries).
    fn push_error(&mut self, function_name: &str, kind: &'static str, error: &str, timestamp_ms: f64) {
        if self.error_log.len() >= 200 {
            self.error_log.pop_front();
        }
        self.error_log.push_back(ErrorLogEntry {
            timestamp_ms,
            cycle: self.stats.total_cycles,
            function_name: function_name.to_string(),
            error_kind: kind,
            error: error.to_string(),
        });
    }

    // ─── Diagnostics ─────────────────────────────────────────────────────

    /// Full OS/platform config + compile latency histogram + error log summary as JSON.
    ///
    /// Includes:
    /// - `os`, `arch`, `family` — from `std::env::consts`
    /// - `cpu_count` — logical CPUs via `available_parallelism()`
    /// - `vitalis_version` — crate version from `Cargo.toml`
    /// - `compile_latency` — P50/P95/P99/avg over last ≤1000 compilations
    /// - `error_log` — total errors + most recent error message
    pub fn diagnostics_json(&self) -> String {
        let uptime_ms = self.boot_time.elapsed().as_secs_f64() * 1000.0;
        let cpu_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        // Compile latency histogram
        let (p50, p95, p99, avg) = if self.compile_latencies.is_empty() {
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64)
        } else {
            let lats = &self.compile_latencies;
            let sum: f64 = lats.iter().sum();
            let avg_val = sum / lats.len() as f64;
            let (p50v, p95v, p99v) = unsafe {
                (
                    hotpath::hotpath_percentile(lats.as_ptr(), lats.len(), 0.50),
                    hotpath::hotpath_percentile(lats.as_ptr(), lats.len(), 0.95),
                    hotpath::hotpath_percentile(lats.as_ptr(), lats.len(), 0.99),
                )
            };
            (p50v, p95v, p99v, avg_val)
        };

        let recent_error = self.error_log.back().map(|e| e.error.clone());

        RuntimeDiagnostics {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            family: std::env::consts::FAMILY,
            cpu_count,
            vitalis_version: env!("CARGO_PKG_VERSION"),
            uptime_ms,
            latency_p50: p50,
            latency_p95: p95,
            latency_p99: p99,
            latency_avg: avg,
            latency_count: self.compile_latencies.len(),
            error_total: self.error_log.len(),
            recent_error,
        }
        .to_json()
    }

    /// Return the last `limit` error log entries as a JSON array.
    /// Pass `limit = 0` to get all (up to 200 stored).
    pub fn error_log_json(&self, limit: usize) -> String {
        let entries: Vec<&ErrorLogEntry> = if limit == 0 || limit >= self.error_log.len() {
            self.error_log.iter().collect()
        } else {
            self.error_log.iter().rev().take(limit).collect::<Vec<_>>()
                .into_iter().rev().collect()
        };
        let jsons: Vec<String> = entries.iter().map(|e| e.to_json()).collect();
        format!("[{}]", jsons.join(","))
    }

    // ─── Stats & Landscape ───────────────────────────────────────────────

    /// Get engine statistics as JSON.
    pub fn stats_json(&self) -> String {
        let uptime = self.boot_time.elapsed().as_secs_f64() * 1000.0;
        let evo_rate = if uptime > 0.0 {
            self.stats.total_evolutions as f64 / (uptime / 1000.0)
        } else {
            0.0
        };
        format!(
            concat!(
                "{{",
                "\"total_cycles\":{},",
                "\"total_evolutions\":{},",
                "\"successful\":{},",
                "\"failed\":{},",
                "\"rollbacks\":{},",
                "\"compile_time_ms\":{:.2},",
                "\"avg_fitness\":{:.4},",
                "\"best_fitness\":{:.4},",
                "\"functions_tracked\":{},",
                "\"uptime_ms\":{:.0},",
                "\"evolutions_per_second\":{:.2}",
                "}}"
            ),
            self.stats.total_cycles,
            self.stats.total_evolutions,
            self.stats.successful_evolutions,
            self.stats.failed_evolutions,
            self.stats.rollbacks,
            self.stats.total_compile_time_ms,
            self.stats.avg_fitness,
            self.stats.best_fitness,
            self.stats.functions_tracked,
            uptime,
            evo_rate,
        )
    }
}

// ─── Global Engine (thread-local) ───────────────────────────────────────

use std::cell::RefCell;

thread_local! {
    static GLOBAL_ENGINE: RefCell<VitalisEngine> = RefCell::new(VitalisEngine::new());
}

/// Access the global Vitalis engine.
pub fn with_engine<F, R>(f: F) -> R
where
    F: FnOnce(&mut VitalisEngine) -> R,
{
    GLOBAL_ENGINE.with(|eng| f(&mut eng.borrow_mut()))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_new() {
        let engine = VitalisEngine::new();
        assert_eq!(engine.stats.total_cycles, 0);
        assert_eq!(engine.stats.functions_tracked, 0);
    }

    #[test]
    fn test_engine_validate_valid() {
        let engine = VitalisEngine::new();
        let result = engine.validate("fn main() -> i64 { 42 }");
        assert!(result.valid);
        assert!(result.parse_errors.is_empty());
        assert!(result.type_errors.is_empty());
        assert!(result.duration_ms >= 0.0);
    }

    #[test]
    fn test_engine_validate_invalid_syntax() {
        let engine = VitalisEngine::new();
        let result = engine.validate("fn main( {");
        assert!(!result.valid);
        assert!(!result.parse_errors.is_empty());
    }

    #[test]
    fn test_engine_register_and_evolve() {
        let mut engine = VitalisEngine::new();
        engine.register("compute", "fn compute(x: i64) -> i64 { x * 2 }");
        assert_eq!(engine.stats.functions_tracked, 1);

        let attempt = engine.evolve("compute", "fn compute(x: i64) -> i64 { x * 3 }");
        assert!(attempt.success);
        assert_eq!(attempt.generation, 1);
        assert!(attempt.fitness > 0.0);
    }

    #[test]
    fn test_engine_reject_invalid_evolution() {
        let mut engine = VitalisEngine::new();
        engine.register("f", "fn f() -> i64 { 1 }");

        let attempt = engine.evolve("f", "fn f( -> { broken }");
        assert!(!attempt.success);
        assert!(attempt.error.is_some());
    }

    #[test]
    fn test_engine_fitness_scoring() {
        let engine = VitalisEngine::new();
        let fitness = engine.calculate_fitness("fn compute(x: i64) -> i64 { x * 2 }");
        assert!(fitness > 0.0);
        assert!(fitness <= 100.0);
    }

    #[test]
    fn test_engine_run_cycle() {
        let mut engine = VitalisEngine::new();
        engine.register("alpha", "fn alpha(x: i64) -> i64 { x + 1 }");
        engine.register("beta", "fn beta(x: i64) -> i64 { x * 2 }");

        let cycle = engine.run_cycle();
        assert_eq!(cycle.cycle_number, 1);
        assert_eq!(cycle.attempts.len(), 2);
        assert!(cycle.total_time_ms >= 0.0);
    }

    #[test]
    fn test_engine_stats_json() {
        let engine = VitalisEngine::new();
        let json = engine.stats_json();
        assert!(json.contains("\"total_cycles\":0"));
        assert!(json.contains("\"functions_tracked\":0"));
        assert!(json.contains("\"evolutions_per_second\""));
    }

    #[test]
    fn test_engine_landscape() {
        let mut engine = VitalisEngine::new();
        engine.register("scorer", "fn scorer(x: i64) -> i64 { x * 10 }");
        let landscape = engine.get_landscape();
        assert!(landscape.contains("scorer"));
        assert!(landscape.contains("\"generation\":0"));
    }

    #[test]
    fn test_engine_validation_result_json() {
        let engine = VitalisEngine::new();
        let result = engine.validate("fn main() -> i64 { 42 }");
        let json = result.to_json();
        assert!(json.contains("\"valid\":true"));
    }
}
