//! Code Intelligence — Code embeddings, similarity, complexity analysis, and semantic search.
//!
//! Provides code representation learning, structural similarity metrics,
//! complexity prediction (cyclomatic, cognitive, Halstead), code pattern
//! detection, and semantic code search via embeddings.

use std::collections::HashMap;

// ── Code Metrics ────────────────────────────────────────────────────────

/// Code complexity metrics.
#[derive(Debug, Clone, Default)]
pub struct ComplexityMetrics {
    pub cyclomatic: usize,
    pub cognitive: usize,
    pub lines_of_code: usize,
    pub lines_of_comments: usize,
    pub blank_lines: usize,
    pub halstead_vocabulary: usize,
    pub halstead_length: usize,
    pub halstead_volume: f64,
    pub halstead_difficulty: f64,
    pub halstead_effort: f64,
    pub maintainability_index: f64,
    pub nesting_depth_max: usize,
    pub num_functions: usize,
    pub num_parameters_avg: f64,
}

/// Compute cyclomatic complexity from source code.
/// CC = E - N + 2P (simplified: count decision points + 1)
pub fn cyclomatic_complexity(source: &str) -> usize {
    let keywords = ["if", "else if", "elif", "while", "for", "case", "catch",
                     "&&", "||", "match", "?"];
    let mut complexity = 1; // Base complexity
    for keyword in &keywords {
        complexity += source.matches(keyword).count();
    }
    complexity
}

/// Compute cognitive complexity (Sonar-style).
/// Adds weight for nesting depth and cognitive overhead.
pub fn cognitive_complexity(source: &str) -> usize {
    let mut complexity = 0;
    let mut nesting = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        // Track nesting via braces
        nesting += trimmed.matches('{').count();
        let closing = trimmed.matches('}').count();

        // Add complexity for control flow with nesting weight
        let control_flow = ["if ", "else if ", "while ", "for ", "match "];
        for kw in &control_flow {
            if trimmed.starts_with(kw) || trimmed.contains(&format!(" {}", kw)) {
                complexity += 1 + nesting;
            }
        }

        // Logical operators add complexity
        complexity += trimmed.matches("&&").count();
        complexity += trimmed.matches("||").count();

        // Recursion adds extra
        if trimmed.contains("self.") && trimmed.contains('(') {
            complexity += 1;
        }

        nesting = nesting.saturating_sub(closing);
    }
    complexity
}

/// Compute Halstead metrics from operators and operands.
pub fn halstead_metrics(operators: &[String], operands: &[String]) -> (f64, f64, f64) {
    let mut unique_ops = std::collections::HashSet::new();
    let mut unique_opds = std::collections::HashSet::new();
    for op in operators { unique_ops.insert(op.clone()); }
    for od in operands { unique_opds.insert(od.clone()); }

    let n1 = unique_ops.len() as f64;  // Unique operators
    let n2 = unique_opds.len() as f64; // Unique operands
    let big_n1 = operators.len() as f64;  // Total operators
    let big_n2 = operands.len() as f64;   // Total operands

    let vocabulary = n1 + n2;
    let length = big_n1 + big_n2;
    let volume = if vocabulary > 0.0 { length * vocabulary.log2() } else { 0.0 };
    let difficulty = if n2 > 0.0 { (n1 / 2.0) * (big_n2 / n2) } else { 0.0 };
    let effort = volume * difficulty;

    (volume, difficulty, effort)
}

/// Compute maintainability index (Microsoft variant).
/// MI = 171 - 5.2 * ln(V) - 0.23 * CC - 16.2 * ln(LOC)
pub fn maintainability_index(halstead_volume: f64, cyclomatic: usize, loc: usize) -> f64 {
    let v = halstead_volume.max(1.0);
    let l = (loc as f64).max(1.0);
    let mi = 171.0 - 5.2 * v.ln() - 0.23 * cyclomatic as f64 - 16.2 * l.ln();
    mi.max(0.0).min(100.0) // Clamp to [0, 100]
}

/// Full complexity analysis of source code.
pub fn analyze_complexity(source: &str) -> ComplexityMetrics {
    let mut metrics = ComplexityMetrics::default();
    let mut max_nesting = 0;
    let mut current_nesting = 0;
    let mut func_count = 0;
    let mut param_total = 0;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            metrics.blank_lines += 1;
        } else if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            metrics.lines_of_comments += 1;
        } else {
            metrics.lines_of_code += 1;
        }

        current_nesting += trimmed.matches('{').count();
        max_nesting = max_nesting.max(current_nesting);
        current_nesting = current_nesting.saturating_sub(trimmed.matches('}').count());

        // Count functions
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.contains(" fn ") {
            func_count += 1;
            // Count parameters
            if let Some(start) = trimmed.find('(') {
                if let Some(end) = trimmed.find(')') {
                    let params = &trimmed[start+1..end];
                    if !params.is_empty() {
                        param_total += params.split(',').count();
                    }
                }
            }
        }
    }

    metrics.cyclomatic = cyclomatic_complexity(source);
    metrics.cognitive = cognitive_complexity(source);
    metrics.nesting_depth_max = max_nesting;
    metrics.num_functions = func_count;
    metrics.num_parameters_avg = if func_count > 0 {
        param_total as f64 / func_count as f64
    } else { 0.0 };

    // Simplified Halstead (using unique tokens as proxy)
    let tokens: Vec<&str> = source.split_whitespace().collect();
    let unique: std::collections::HashSet<&str> = tokens.iter().cloned().collect();
    let vocab = unique.len();
    let length = tokens.len();
    let volume = if vocab > 0 { length as f64 * (vocab as f64).log2() } else { 0.0 };
    metrics.halstead_vocabulary = vocab;
    metrics.halstead_length = length;
    metrics.halstead_volume = volume;
    metrics.maintainability_index = maintainability_index(volume, metrics.cyclomatic, metrics.lines_of_code);

    metrics
}

// ── Code Embeddings ─────────────────────────────────────────────────────

/// Simple bag-of-tokens code embedding.
pub fn code_embedding(source: &str, vocab: &HashMap<String, usize>, dim: usize) -> Vec<f64> {
    let mut embedding = vec![0.0; dim];
    let mut count = 0;

    for token in source.split_whitespace() {
        let token_lower = token.to_lowercase();
        if let Some(&idx) = vocab.get(&token_lower) {
            let bucket = idx % dim;
            embedding[bucket] += 1.0;
            count += 1;
        }
    }

    // Normalize
    if count > 0 {
        let norm: f64 = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for v in embedding.iter_mut() {
                *v /= norm;
            }
        }
    }
    embedding
}

/// Build vocabulary from multiple source files.
pub fn build_code_vocab(sources: &[&str]) -> HashMap<String, usize> {
    let mut vocab = HashMap::new();
    let mut idx = 0;
    for source in sources {
        for token in source.split_whitespace() {
            let token_lower = token.to_lowercase();
            if !vocab.contains_key(&token_lower) {
                vocab.insert(token_lower, idx);
                idx += 1;
            }
        }
    }
    vocab
}

/// Cosine similarity between two embeddings.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Find most similar code from a database.
pub fn find_similar_code(
    query_embedding: &[f64],
    database: &[(String, Vec<f64>)],
    top_k: usize,
) -> Vec<(String, f64)> {
    let mut similarities: Vec<(String, f64)> = database.iter()
        .map(|(name, emb)| (name.clone(), cosine_similarity(query_embedding, emb)))
        .collect();
    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    similarities.truncate(top_k);
    similarities
}

// ── Code Pattern Detection ──────────────────────────────────────────────

/// Detected code pattern/smell.
#[derive(Debug, Clone, PartialEq)]
pub enum CodePattern {
    LongFunction(usize),       // LOC threshold
    DeepNesting(usize),        // Depth threshold
    GodClass(usize),           // Number of methods threshold
    DuplicatedBlock(String),   // Repeated code snippet
    ComplexCondition(usize),   // Number of boolean operators
    TooManyParams(usize),      // Parameter count
    UnusedVariable(String),    // Variable name
}

/// Detect common code patterns/smells.
pub fn detect_patterns(source: &str) -> Vec<CodePattern> {
    let mut patterns = Vec::new();
    let metrics = analyze_complexity(source);

    if metrics.lines_of_code > 50 {
        patterns.push(CodePattern::LongFunction(metrics.lines_of_code));
    }
    if metrics.nesting_depth_max > 4 {
        patterns.push(CodePattern::DeepNesting(metrics.nesting_depth_max));
    }
    if metrics.num_parameters_avg > 5.0 {
        patterns.push(CodePattern::TooManyParams(metrics.num_parameters_avg as usize));
    }

    // Detect complex conditions
    for line in source.lines() {
        let bool_ops = line.matches("&&").count() + line.matches("||").count();
        if bool_ops >= 3 {
            patterns.push(CodePattern::ComplexCondition(bool_ops));
        }
    }

    // Simple duplicate detection (n-gram based)
    let lines: Vec<&str> = source.lines().collect();
    let window = 3;
    let mut seen_blocks: HashMap<String, usize> = HashMap::new();
    for i in 0..lines.len().saturating_sub(window) {
        let block = lines[i..i+window].join("\n").trim().to_string();
        if block.len() > 20 { // Ignore trivial blocks
            *seen_blocks.entry(block.clone()).or_insert(0) += 1;
        }
    }
    for (block, count) in &seen_blocks {
        if *count > 1 {
            patterns.push(CodePattern::DuplicatedBlock(block.clone()));
        }
    }

    patterns
}

// ── FFI Interface ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_code_cyclomatic(source: *const u8, len: i64) -> i64 {
    let s = unsafe { std::slice::from_raw_parts(source, len as usize) };
    let text = std::str::from_utf8(s).unwrap_or("");
    cyclomatic_complexity(text) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_code_cognitive(source: *const u8, len: i64) -> i64 {
    let s = unsafe { std::slice::from_raw_parts(source, len as usize) };
    let text = std::str::from_utf8(s).unwrap_or("");
    cognitive_complexity(text) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_code_maintainability(volume: f64, cyclomatic: i64, loc: i64) -> f64 {
    maintainability_index(volume, cyclomatic as usize, loc as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn vitalis_code_similarity(a: *const f64, b: *const f64, dim: i64) -> f64 {
    let va = unsafe { std::slice::from_raw_parts(a, dim as usize) };
    let vb = unsafe { std::slice::from_raw_parts(b, dim as usize) };
    cosine_similarity(va, vb)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyclomatic_simple() {
        let code = "fn main() { let x = 1; }";
        assert_eq!(cyclomatic_complexity(code), 1); // No branches
    }

    #[test]
    fn test_cyclomatic_with_branches() {
        let code = "fn f() { if x > 0 { } else if y > 0 { } while z { } }";
        let cc = cyclomatic_complexity(code);
        assert!(cc >= 3); // Base + if + else if + while
    }

    #[test]
    fn test_cognitive_complexity() {
        let code = "fn f() {\n  if x {\n    if y {\n      do_thing();\n    }\n  }\n}";
        let cc = cognitive_complexity(code);
        assert!(cc >= 2); // Nested if adds nesting weight
    }

    #[test]
    fn test_halstead_metrics() {
        let ops = vec!["+".to_string(), "-".to_string(), "+".to_string()];
        let opds = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let (volume, difficulty, effort) = halstead_metrics(&ops, &opds);
        assert!(volume > 0.0);
        assert!(difficulty > 0.0);
        assert!(effort > 0.0);
    }

    #[test]
    fn test_maintainability_index() {
        let mi = maintainability_index(100.0, 5, 50);
        assert!(mi > 0.0 && mi <= 100.0);
    }

    #[test]
    fn test_analyze_complexity() {
        let code = "fn main() {\n  let x = 1;\n  if x > 0 {\n    println!(\"positive\");\n  }\n}\n";
        let metrics = analyze_complexity(code);
        assert!(metrics.lines_of_code > 0);
        assert!(metrics.cyclomatic >= 2);
        assert!(metrics.num_functions >= 1);
    }

    #[test]
    fn test_code_embedding() {
        let sources = vec!["fn hello() { let x = 1; }"];
        let vocab = build_code_vocab(&sources);
        let emb = code_embedding(sources[0], &vocab, 16);
        assert_eq!(emb.len(), 16);
        // Should be normalized
        let norm: f64 = emb.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 { assert!((norm - 1.0).abs() < 1e-6); }
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 1.0];
        let b = vec![1.0, 0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-10);
    }

    #[test]
    fn test_find_similar_code() {
        let db = vec![
            ("func_a".to_string(), vec![1.0, 0.0, 0.0]),
            ("func_b".to_string(), vec![0.0, 1.0, 0.0]),
            ("func_c".to_string(), vec![0.9, 0.1, 0.0]),
        ];
        let query = vec![1.0, 0.0, 0.0];
        let results = find_similar_code(&query, &db, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "func_a"); // Most similar
    }

    #[test]
    fn test_detect_patterns_deep_nesting() {
        let code = "fn f() {\n  if a {\n    if b {\n      if c {\n        if d {\n          if e {\n            x();\n          }\n        }\n      }\n    }\n  }\n}";
        let patterns = detect_patterns(code);
        assert!(patterns.iter().any(|p| matches!(p, CodePattern::DeepNesting(_))));
    }

    #[test]
    fn test_build_vocab() {
        let sources = vec!["fn hello(x: i32)", "fn world(y: f64)"];
        let vocab = build_code_vocab(&sources);
        assert!(vocab.contains_key("fn"));
        assert!(vocab.len() > 0);
    }

    #[test]
    fn test_ffi_cyclomatic() {
        let code = b"fn f() { if x { } }";
        let cc = vitalis_code_cyclomatic(code.as_ptr(), code.len() as i64);
        assert!(cc >= 2);
    }

    #[test]
    fn test_ffi_similarity() {
        let a = [1.0f64, 0.0, 0.0];
        let b = [1.0f64, 0.0, 0.0];
        let sim = vitalis_code_similarity(a.as_ptr(), b.as_ptr(), 3);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_metrics_blank_and_comments() {
        let code = "// comment\n\nfn main() {}\n\n// another\n";
        let metrics = analyze_complexity(code);
        assert!(metrics.lines_of_comments >= 2);
        assert!(metrics.blank_lines >= 2);
    }
}
