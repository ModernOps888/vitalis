//! Scoring & Fitness Evaluation Module for Vitalis v9.0
//!
//! Pure Rust implementations of code quality scoring, evolution fitness functions,
//! multi-objective optimization, benchmarking, and statistical evaluation.

// --- Code Quality Scoring ---

/// Maintainability Index (MI).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_maintainability_index(halstead_volume: f64, cyclomatic_complexity: f64, loc: f64) -> f64 {
    let v = if halstead_volume > 0.0 { halstead_volume.ln() } else { 0.0 };
    let l = if loc > 0.0 { loc.ln() } else { 0.0 };
    let mi = 171.0 - 5.2 * v - 0.23 * cyclomatic_complexity - 16.2 * l;
    (mi * 100.0 / 171.0).clamp(0.0, 100.0)
}

/// Technical debt ratio.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tech_debt_ratio(issues: f64, avg_fix_time_hours: f64, total_dev_time_hours: f64) -> f64 {
    if total_dev_time_hours <= 0.0 { return 0.0; }
    (issues * avg_fix_time_hours / total_dev_time_hours) * 100.0
}

/// Code quality composite score (0-100).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_code_quality_composite(
    cyclomatic: f64, cognitive: f64, loc: f64, num_functions: f64,
    issues: f64, test_coverage: f64, duplication: f64,
) -> f64 {
    let mut score = 100.0;
    let cc_per_func = if num_functions > 0.0 { cyclomatic / num_functions } else { cyclomatic };
    if cc_per_func > 10.0 { score -= (cc_per_func - 10.0) * 3.0; }
    if cc_per_func > 20.0 { score -= (cc_per_func - 20.0) * 5.0; }
    let cog_per_func = if num_functions > 0.0 { cognitive / num_functions } else { cognitive };
    if cog_per_func > 8.0 { score -= (cog_per_func - 8.0) * 2.0; }
    let issue_density = if loc > 0.0 { issues * 1000.0 / loc } else { 0.0 };
    score -= issue_density * 5.0;
    if test_coverage >= 80.0 { score += 5.0; } else if test_coverage < 50.0 { score -= (50.0 - test_coverage) * 0.5; }
    if duplication > 5.0 { score -= (duplication - 5.0) * 0.8; }
    score.clamp(0.0, 100.0)
}

/// Halstead metrics. out must hold 5 f64: (volume, difficulty, effort, time, bugs).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_halstead_metrics(n1: f64, n2: f64, eta1: f64, eta2: f64, out: *mut f64) {
    if out.is_null() { return; }
    let o = unsafe { std::slice::from_raw_parts_mut(out, 5) };
    let n = n1 + n2;
    let eta = eta1 + eta2;
    let volume = if eta > 0.0 { n * eta.log2() } else { 0.0 };
    let difficulty = if eta2 > 0.0 { (eta1 / 2.0) * (n2 / eta2) } else { 0.0 };
    let effort = volume * difficulty;
    o[0] = volume; o[1] = difficulty; o[2] = effort; o[3] = effort / 18.0; o[4] = volume / 3000.0;
}

// --- Evolution Fitness ---

/// Multi-objective weighted fitness.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_weighted_fitness(objectives: *const f64, weights: *const f64, n: usize) -> f64 {
    if objectives.is_null() || weights.is_null() || n == 0 { return 0.0; }
    let obj = unsafe { std::slice::from_raw_parts(objectives, n) };
    let w = unsafe { std::slice::from_raw_parts(weights, n) };
    let total_weight: f64 = w.iter().sum();
    if total_weight <= 0.0 { return 0.0; }
    obj.iter().zip(w.iter()).map(|(o, weight)| o * weight).sum::<f64>() / total_weight
}

/// Pareto dominance: does A dominate B? Returns 1 if yes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_pareto_dominates(a: *const f64, b: *const f64, n: usize) -> i32 {
    if a.is_null() || b.is_null() || n == 0 { return 0; }
    let av = unsafe { std::slice::from_raw_parts(a, n) };
    let bv = unsafe { std::slice::from_raw_parts(b, n) };
    let mut all_geq = true;
    let mut any_greater = false;
    for i in 0..n {
        if av[i] < bv[i] { all_geq = false; break; }
        if av[i] > bv[i] { any_greater = true; }
    }
    if all_geq && any_greater { 1 } else { 0 }
}

/// Non-dominated sorting rank.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_pareto_rank(solutions: *const f64, n_pop: usize, n_obj: usize, ranks: *mut u32) {
    if solutions.is_null() || ranks.is_null() || n_pop == 0 || n_obj == 0 { return; }
    let sol = unsafe { std::slice::from_raw_parts(solutions, n_pop * n_obj) };
    let r = unsafe { std::slice::from_raw_parts_mut(ranks, n_pop) };
    let mut assigned = vec![false; n_pop];
    let mut rank = 0u32;
    let mut remaining = n_pop;
    while remaining > 0 {
        let mut front = Vec::new();
        for i in 0..n_pop {
            if assigned[i] { continue; }
            let mut dominated = false;
            for j in 0..n_pop {
                if i == j || assigned[j] { continue; }
                let mut all_geq = true;
                let mut any_gt = false;
                for k in 0..n_obj {
                    if sol[j*n_obj+k] < sol[i*n_obj+k] { all_geq = false; break; }
                    if sol[j*n_obj+k] > sol[i*n_obj+k] { any_gt = true; }
                }
                if all_geq && any_gt { dominated = true; break; }
            }
            if !dominated { front.push(i); }
        }
        if front.is_empty() {
            for i in 0..n_pop { if !assigned[i] { r[i] = rank; assigned[i] = true; remaining -= 1; } }
            break;
        }
        for &i in &front { r[i] = rank; assigned[i] = true; remaining -= 1; }
        rank += 1;
    }
}

// --- Rating Systems ---

/// ELO rating update. out must hold 2 f64s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_elo_update(rating_a: f64, rating_b: f64, result: f64, k_factor: f64, out: *mut f64) {
    if out.is_null() { return; }
    let o = unsafe { std::slice::from_raw_parts_mut(out, 2) };
    let ea = 1.0 / (1.0 + 10.0f64.powf((rating_b - rating_a) / 400.0));
    let eb = 1.0 - ea;
    o[0] = rating_a + k_factor * (result - ea);
    o[1] = rating_b + k_factor * ((1.0 - result) - eb);
}

/// ELO expected score for A vs B.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_elo_expected(rating_a: f64, rating_b: f64) -> f64 {
    1.0 / (1.0 + 10.0f64.powf((rating_b - rating_a) / 400.0))
}

// --- Statistical Tests ---

/// Welch's t-test statistic.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_welch_t(a: *const f64, n_a: usize, b: *const f64, n_b: usize) -> f64 {
    if a.is_null() || b.is_null() || n_a < 2 || n_b < 2 { return 0.0; }
    let av = unsafe { std::slice::from_raw_parts(a, n_a) };
    let bv = unsafe { std::slice::from_raw_parts(b, n_b) };
    let mean_a: f64 = av.iter().sum::<f64>() / n_a as f64;
    let mean_b: f64 = bv.iter().sum::<f64>() / n_b as f64;
    let var_a: f64 = av.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>() / (n_a - 1) as f64;
    let var_b: f64 = bv.iter().map(|x| (x - mean_b).powi(2)).sum::<f64>() / (n_b - 1) as f64;
    let se = (var_a / n_a as f64 + var_b / n_b as f64).sqrt();
    if se < 1e-15 { return 0.0; }
    (mean_a - mean_b) / se
}

/// Cohen's d effect size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cohens_d(a: *const f64, n_a: usize, b: *const f64, n_b: usize) -> f64 {
    if a.is_null() || b.is_null() || n_a < 2 || n_b < 2 { return 0.0; }
    let av = unsafe { std::slice::from_raw_parts(a, n_a) };
    let bv = unsafe { std::slice::from_raw_parts(b, n_b) };
    let mean_a: f64 = av.iter().sum::<f64>() / n_a as f64;
    let mean_b: f64 = bv.iter().sum::<f64>() / n_b as f64;
    let var_a: f64 = av.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>() / (n_a - 1) as f64;
    let var_b: f64 = bv.iter().map(|x| (x - mean_b).powi(2)).sum::<f64>() / (n_b - 1) as f64;
    let sp = (((n_a - 1) as f64 * var_a + (n_b - 1) as f64 * var_b) / (n_a + n_b - 2) as f64).sqrt();
    if sp < 1e-15 { return 0.0; }
    (mean_a - mean_b) / sp
}

/// Mann-Whitney U statistic.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mann_whitney_u(a: *const f64, n_a: usize, b: *const f64, n_b: usize) -> f64 {
    if a.is_null() || b.is_null() || n_a == 0 || n_b == 0 { return 0.0; }
    let av = unsafe { std::slice::from_raw_parts(a, n_a) };
    let bv = unsafe { std::slice::from_raw_parts(b, n_b) };
    let mut u = 0.0;
    for ai in av { for bi in bv {
        if ai > bi { u += 1.0; } else if (ai - bi).abs() < 1e-15 { u += 0.5; }
    }}
    u
}

// --- A/B Testing ---

/// Conversion rate with Wilson CI. out must hold 3 f64s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_conversion_rate(successes: f64, trials: f64, z: f64, out: *mut f64) {
    if out.is_null() || trials <= 0.0 { return; }
    let o = unsafe { std::slice::from_raw_parts_mut(out, 3) };
    let p = successes / trials;
    let n = trials;
    let z2 = z * z;
    let denom = 1.0 + z2/n;
    let center = (p + z2/(2.0*n)) / denom;
    let margin = z * (p*(1.0-p)/n + z2/(4.0*n*n)).sqrt() / denom;
    o[0] = p; o[1] = (center - margin).max(0.0); o[2] = (center + margin).min(1.0);
}

/// Bayesian A/B: P(A > B).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bayesian_ab(a_succ: f64, a_fail: f64, b_succ: f64, b_fail: f64) -> f64 {
    let alpha_a = a_succ + 1.0; let beta_a = a_fail + 1.0;
    let alpha_b = b_succ + 1.0; let beta_b = b_fail + 1.0;
    let mean_a = alpha_a / (alpha_a + beta_a);
    let mean_b = alpha_b / (alpha_b + beta_b);
    let var_a = (alpha_a * beta_a) / ((alpha_a + beta_a).powi(2) * (alpha_a + beta_a + 1.0));
    let var_b = (alpha_b * beta_b) / ((alpha_b + beta_b).powi(2) * (alpha_b + beta_b + 1.0));
    let std_diff = (var_a + var_b).sqrt();
    if std_diff < 1e-15 { return 0.5; }
    let z = (mean_a - mean_b) / std_diff;
    normal_cdf(z)
}

fn normal_cdf(x: f64) -> f64 { 0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2)) }

fn erf(x: f64) -> f64 {
    let a = [0.254829592, -0.284496736, 1.421413741, -1.453152027, 1.061405429];
    let p = 0.3275911;
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a[4]*t + a[3])*t) + a[2])*t + a[1])*t + a[0])*t * (-x*x).exp();
    sign * y
}

// --- Regression Detection ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_regression_score(current: f64, baseline: f64) -> f64 {
    if baseline.abs() < 1e-15 { return 0.0; }
    ((current - baseline) / baseline) * 100.0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_regression_count(current: *const f64, baseline: *const f64, n: usize, threshold_pct: f64) -> usize {
    if current.is_null() || baseline.is_null() || n == 0 { return 0; }
    let c = unsafe { std::slice::from_raw_parts(current, n) };
    let b = unsafe { std::slice::from_raw_parts(baseline, n) };
    let mut count = 0;
    for i in 0..n {
        if b[i].abs() < 1e-15 { continue; }
        let pct = ((c[i] - b[i]) / b[i]).abs() * 100.0;
        if pct > threshold_pct { count += 1; }
    }
    count
}

// --- Composite Scores ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_geometric_mean(values: *const f64, n: usize) -> f64 {
    if values.is_null() || n == 0 { return 0.0; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let log_sum: f64 = v.iter().map(|x| if *x > 0.0 { x.ln() } else { 0.0 }).sum();
    (log_sum / n as f64).exp()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_harmonic_mean(values: *const f64, n: usize) -> f64 {
    if values.is_null() || n == 0 { return 0.0; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let reciprocal_sum: f64 = v.iter().map(|x| if x.abs() > 1e-15 { 1.0 / x } else { 0.0 }).sum();
    if reciprocal_sum.abs() < 1e-15 { return 0.0; }
    n as f64 / reciprocal_sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_power_mean(values: *const f64, weights: *const f64, n: usize, p: f64) -> f64 {
    if values.is_null() || weights.is_null() || n == 0 { return 0.0; }
    let v = unsafe { std::slice::from_raw_parts(values, n) };
    let w = unsafe { std::slice::from_raw_parts(weights, n) };
    let total_w: f64 = w.iter().sum();
    if total_w <= 0.0 { return 0.0; }
    if p.abs() < 1e-15 {
        let log_sum: f64 = v.iter().zip(w.iter()).map(|(vi, wi)| if *vi > 0.0 { wi * vi.ln() } else { 0.0 }).sum();
        return (log_sum / total_w).exp();
    }
    let sum: f64 = v.iter().zip(w.iter()).map(|(vi, wi)| wi * vi.powf(p)).sum();
    (sum / total_w).powf(1.0 / p)
}

// --- Benchmark Metrics ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_latency_score(p50: f64, p95: f64, p99: f64, target_p50: f64, target_p95: f64, target_p99: f64) -> f64 {
    let s50 = if target_p50 > 0.0 { (1.0 - p50/target_p50).max(0.0) * 40.0 } else { 40.0 };
    let s95 = if target_p95 > 0.0 { (1.0 - p95/target_p95).max(0.0) * 35.0 } else { 35.0 };
    let s99 = if target_p99 > 0.0 { (1.0 - p99/target_p99).max(0.0) * 25.0 } else { 25.0 };
    (s50 + s95 + s99).clamp(0.0, 100.0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_efficiency_ratio(useful_work: f64, total_resources: f64) -> f64 {
    if total_resources <= 0.0 { return 0.0; }
    useful_work / total_resources
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_throughput_efficiency(actual: f64, theoretical_max: f64) -> f64 {
    if theoretical_max <= 0.0 { return 0.0; }
    (actual / theoretical_max * 100.0).min(100.0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_system_health(dimensions: *const f64, weights: *const f64, n: usize) -> f64 {
    if dimensions.is_null() || weights.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(dimensions, n) };
    let w = unsafe { std::slice::from_raw_parts(weights, n) };
    let total_w: f64 = w.iter().sum();
    if total_w <= 0.0 { return 0.0; }
    let sum: f64 = d.iter().zip(w.iter()).map(|(di, wi)| di * wi).sum();
    (sum / total_w).clamp(0.0, 100.0)
}

// --- Fitness Landscape ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_decay_fitness(distance: f64, k: f64) -> f64 { (-k * distance).exp() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sigmoid_fitness(x: f64, k: f64, midpoint: f64) -> f64 {
    1.0 / (1.0 + (-k * (x - midpoint)).exp())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tournament_fitness(wins: f64, losses: f64, draws: f64) -> f64 {
    let total = wins + losses + draws;
    if total <= 0.0 { return 0.0; }
    let win_rate = (wins + draws * 0.5) / total;
    let consistency = 1.0 - (wins.min(losses) / total);
    win_rate * 0.7 + consistency * 0.3
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maintainability_index() {
        let mi = unsafe { vitalis_maintainability_index(100.0, 10.0, 50.0) };
        assert!(mi > 0.0 && mi <= 100.0);
    }

    #[test]
    fn test_tech_debt() {
        let td = unsafe { vitalis_tech_debt_ratio(10.0, 2.0, 100.0) };
        assert!((td - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_code_quality() {
        let s = unsafe { vitalis_code_quality_composite(20.0, 15.0, 500.0, 10.0, 3.0, 80.0, 5.0) };
        assert!(s > 0.0 && s <= 100.0);
    }

    #[test]
    fn test_halstead() {
        let mut out = [0.0f64; 5];
        unsafe { vitalis_halstead_metrics(50.0, 30.0, 10.0, 8.0, out.as_mut_ptr()); }
        assert!(out[0] > 0.0);
    }

    #[test]
    fn test_weighted_fitness() {
        let obj = [0.8, 0.6, 0.9];
        let w = [1.0, 2.0, 1.0];
        let f = unsafe { vitalis_weighted_fitness(obj.as_ptr(), w.as_ptr(), 3) };
        assert!((f - 0.725).abs() < 1e-10);
    }

    #[test]
    fn test_pareto_dominates() {
        let a = [3.0, 2.0];
        let b = [2.0, 1.0];
        assert_eq!(unsafe { vitalis_pareto_dominates(a.as_ptr(), b.as_ptr(), 2) }, 1);
        assert_eq!(unsafe { vitalis_pareto_dominates(b.as_ptr(), a.as_ptr(), 2) }, 0);
    }

    #[test]
    fn test_elo() {
        let mut out = [0.0f64; 2];
        unsafe { vitalis_elo_update(1500.0, 1500.0, 1.0, 32.0, out.as_mut_ptr()); }
        assert!(out[0] > 1500.0);
        assert!(out[1] < 1500.0);
    }

    #[test]
    fn test_elo_expected() {
        let e = unsafe { vitalis_elo_expected(1500.0, 1500.0) };
        assert!((e - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_welch_t() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [6.0, 7.0, 8.0, 9.0, 10.0];
        let t = unsafe { vitalis_welch_t(a.as_ptr(), 5, b.as_ptr(), 5) };
        assert!(t < -3.0);
    }

    #[test]
    fn test_cohens_d() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [6.0, 7.0, 8.0, 9.0, 10.0];
        let d = unsafe { vitalis_cohens_d(a.as_ptr(), 5, b.as_ptr(), 5) };
        assert!(d.abs() > 2.0);
    }

    #[test]
    fn test_mann_whitney() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let u = unsafe { vitalis_mann_whitney_u(a.as_ptr(), 3, b.as_ptr(), 3) };
        assert_eq!(u, 0.0);
    }

    #[test]
    fn test_conversion_rate() {
        let mut out = [0.0f64; 3];
        unsafe { vitalis_conversion_rate(50.0, 100.0, 1.96, out.as_mut_ptr()); }
        assert!((out[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_bayesian_ab() {
        let p = unsafe { vitalis_bayesian_ab(80.0, 20.0, 20.0, 80.0) };
        assert!(p > 0.95);
    }

    #[test]
    fn test_regression_score() {
        let s = unsafe { vitalis_regression_score(110.0, 100.0) };
        assert!((s - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_geometric_mean() {
        let v = [2.0, 8.0];
        let gm = unsafe { vitalis_geometric_mean(v.as_ptr(), 2) };
        assert!((gm - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_harmonic_mean() {
        let v = [2.0, 3.0, 6.0];
        let hm = unsafe { vitalis_harmonic_mean(v.as_ptr(), 3) };
        assert!((hm - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_sigmoid_fitness() {
        let f = unsafe { vitalis_sigmoid_fitness(5.0, 1.0, 5.0) };
        assert!((f - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_efficiency() {
        let e = unsafe { vitalis_efficiency_ratio(80.0, 100.0) };
        assert!((e - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_system_health() {
        let dims = [99.9, 95.0, 85.0, 97.0, 80.0];
        let weights = [0.3, 0.25, 0.2, 0.15, 0.1];
        let h = unsafe { vitalis_system_health(dims.as_ptr(), weights.as_ptr(), 5) };
        assert!(h > 85.0 && h <= 100.0);
    }
}
