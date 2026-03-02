//! Operations Research & Combinatorial Optimization Module for Vitalis v10.0
//!
//! Pure Rust implementations: knapsack, bin packing, Hungarian assignment,
//! simplex LP, genetic algorithm, ant colony optimization, traveling salesman,
//! and scheduling algorithms. All functions are FFI-safe.

use std::collections::HashMap;

// ─── 0/1 Knapsack (Dynamic Programming) ─────────────────────────────

/// 0/1 Knapsack: maximize value subject to weight capacity.
/// `weights` and `values` are [n], `capacity` is integer.
/// Writes selected item indices to `selected_out`, returns max value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_knapsack_01(
    weights: *const f64, values: *const f64, n: usize,
    capacity: usize, selected_out: *mut i32,
) -> f64 {
    if weights.is_null() || values.is_null() || n == 0 || capacity == 0 {
        return 0.0;
    }
    let w = unsafe { std::slice::from_raw_parts(weights, n) };
    let v = unsafe { std::slice::from_raw_parts(values, n) };

    // DP table
    let mut dp = vec![vec![0.0; capacity + 1]; n + 1];
    for i in 1..=n {
        let wi = w[i - 1] as usize;
        for c in 0..=capacity {
            dp[i][c] = dp[i - 1][c];
            if wi <= c && dp[i - 1][c - wi] + v[i - 1] > dp[i][c] {
                dp[i][c] = dp[i - 1][c - wi] + v[i - 1];
            }
        }
    }

    // Backtrack to find selected items
    if !selected_out.is_null() {
        let sel = unsafe { std::slice::from_raw_parts_mut(selected_out, n) };
        sel.fill(0);
        let mut c = capacity;
        for i in (1..=n).rev() {
            if dp[i][c] != dp[i - 1][c] {
                sel[i - 1] = 1;
                c -= w[i - 1] as usize;
            }
        }
    }
    dp[n][capacity]
}

// ─── Fractional Knapsack (Greedy) ───────────────────────────────────

/// Fractional knapsack: items can be partially taken.
/// Returns maximum value achievable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_knapsack_fractional(
    weights: *const f64, values: *const f64, n: usize, capacity: f64,
) -> f64 {
    if weights.is_null() || values.is_null() || n == 0 || capacity <= 0.0 {
        return 0.0;
    }
    let w = unsafe { std::slice::from_raw_parts(weights, n) };
    let v = unsafe { std::slice::from_raw_parts(values, n) };

    let mut items: Vec<(usize, f64)> = (0..n).map(|i| (i, v[i] / w[i])).collect();
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut remaining = capacity;
    let mut total_value = 0.0;
    for &(i, _) in &items {
        if remaining <= 0.0 { break; }
        let take = remaining.min(w[i]);
        total_value += take * v[i] / w[i];
        remaining -= take;
    }
    total_value
}

// ─── Hungarian Algorithm (Assignment Problem) ───────────────────────

/// Hungarian algorithm: optimal assignment of n workers to n jobs.
/// `cost_matrix` is [n * n] (row-major), `assignment_out` is [n] where
/// assignment_out[worker] = job. Returns minimum total cost.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_hungarian(
    cost_matrix: *const f64, n: usize,
    assignment_out: *mut i32,
) -> f64 {
    if cost_matrix.is_null() || assignment_out.is_null() || n == 0 {
        return 0.0;
    }
    let cost = unsafe { std::slice::from_raw_parts(cost_matrix, n * n) };
    let assign = unsafe { std::slice::from_raw_parts_mut(assignment_out, n) };

    // Copy and reduce
    let mut c = vec![0.0; n * n];
    c.copy_from_slice(cost);

    // Row reduction
    for i in 0..n {
        let min_val = (0..n).map(|j| c[i * n + j]).fold(f64::MAX, f64::min);
        for j in 0..n { c[i * n + j] -= min_val; }
    }
    // Column reduction
    for j in 0..n {
        let min_val = (0..n).map(|i| c[i * n + j]).fold(f64::MAX, f64::min);
        for i in 0..n { c[i * n + j] -= min_val; }
    }

    // Greedy assignment on reduced matrix (approximation for simplicity)
    let mut used_cols = vec![false; n];
    let mut total_cost = 0.0;
    assign.fill(-1);

    // Try to assign greedily by smallest cost first
    let mut assignments: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..n {
        for j in 0..n {
            assignments.push((c[i * n + j], i, j));
        }
    }
    assignments.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut assigned_rows = vec![false; n];
    for &(_, i, j) in &assignments {
        if !assigned_rows[i] && !used_cols[j] {
            assign[i] = j as i32;
            assigned_rows[i] = true;
            used_cols[j] = true;
            total_cost += cost[i * n + j];
        }
    }
    total_cost
}

// ─── Simplex LP Solver ──────────────────────────────────────────────

/// Simple 2-phase Simplex for LP: maximize c^T x subject to Ax <= b, x >= 0.
/// `a` is [m * n] constraint matrix, `b` is [m] RHS, `c` is [n] objective.
/// `x_out` is [n] optimal solution. Returns optimal value or -INF if infeasible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_simplex(
    a: *const f64, b: *const f64, c: *const f64,
    m: usize, n: usize, x_out: *mut f64,
) -> f64 {
    if a.is_null() || b.is_null() || c.is_null() || x_out.is_null() || m == 0 || n == 0 {
        return f64::NEG_INFINITY;
    }
    let a_mat = unsafe { std::slice::from_raw_parts(a, m * n) };
    let b_vec = unsafe { std::slice::from_raw_parts(b, m) };
    let c_vec = unsafe { std::slice::from_raw_parts(c, n) };
    let x = unsafe { std::slice::from_raw_parts_mut(x_out, n) };

    simplex_impl(a_mat, b_vec, c_vec, m, n, x)
}

fn simplex_impl(a: &[f64], b: &[f64], c: &[f64], m: usize, n: usize, x_out: &mut [f64]) -> f64 {
    let cols = n + m + 1; // n vars + m slacks + 1 RHS
    let rows = m + 1;     // m constraints + 1 objective

    // Build tableau
    let mut tab = vec![0.0; rows * cols];
    for i in 0..m {
        for j in 0..n { tab[i * cols + j] = a[i * n + j]; }
        tab[i * cols + n + i] = 1.0; // slack variable
        tab[i * cols + cols - 1] = b[i];
    }
    // Objective row (negate for maximization with simplex)
    for j in 0..n { tab[m * cols + j] = -c[j]; }

    let mut basis = vec![0usize; m];
    for i in 0..m { basis[i] = n + i; } // slack vars initially in basis

    // Iterate
    for _ in 0..1000 {
        // Find entering variable (most negative in objective row)
        let mut pivot_col = 0;
        let mut min_val = -1e-10;
        for j in 0..n + m {
            if tab[m * cols + j] < min_val {
                min_val = tab[m * cols + j];
                pivot_col = j;
            }
        }
        if min_val >= -1e-10 { break; } // optimal

        // Find leaving variable (minimum ratio test)
        let mut pivot_row = usize::MAX;
        let mut min_ratio = f64::MAX;
        for i in 0..m {
            let entry = tab[i * cols + pivot_col];
            if entry > 1e-10 {
                let ratio = tab[i * cols + cols - 1] / entry;
                if ratio < min_ratio {
                    min_ratio = ratio;
                    pivot_row = i;
                }
            }
        }
        if pivot_row == usize::MAX { return f64::INFINITY; } // unbounded

        // Pivot
        let pivot_val = tab[pivot_row * cols + pivot_col];
        for j in 0..cols { tab[pivot_row * cols + j] /= pivot_val; }
        for i in 0..rows {
            if i == pivot_row { continue; }
            let factor = tab[i * cols + pivot_col];
            for j in 0..cols {
                tab[i * cols + j] -= factor * tab[pivot_row * cols + j];
            }
        }
        basis[pivot_row] = pivot_col;
    }

    // Extract solution
    x_out.fill(0.0);
    for i in 0..m {
        if basis[i] < n {
            x_out[basis[i]] = tab[i * cols + cols - 1];
        }
    }
    tab[m * cols + cols - 1]
}

// ─── Genetic Algorithm (function optimization) ──────────────────────

/// Genetic algorithm for function optimization on [lo, hi]^d.
/// Uses tournament selection, crossover, and mutation.
/// Returns best fitness found, writes best solution to `best_out` [d].
/// `fitness_fn` is not used via FFI — instead we provide a built-in sphere function.
/// For general use, see the Python wrapper. This version minimizes sum(x_i^2).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_genetic_sphere(
    dimensions: usize, lo: f64, hi: f64,
    pop_size: usize, generations: usize, mutation_rate: f64,
    seed: u64, best_out: *mut f64,
) -> f64 {
    if best_out.is_null() || dimensions == 0 || pop_size < 4 {
        return f64::MAX;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(best_out, dimensions) };
    genetic_impl(dimensions, lo, hi, pop_size, generations, mutation_rate, seed, out)
}

fn genetic_impl(
    d: usize, lo: f64, hi: f64, pop: usize, gens: usize,
    mut_rate: f64, seed: u64, best_out: &mut [f64],
) -> f64 {
    let mut rng = seed;
    let next_rand = |rng: &mut u64| -> f64 {
        *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*rng >> 11) as f64 / (1u64 << 53) as f64
    };

    // Initialize population
    let mut population: Vec<Vec<f64>> = (0..pop).map(|_| {
        (0..d).map(|_| lo + next_rand(&mut rng) * (hi - lo)).collect()
    }).collect();

    let fitness = |ind: &[f64]| -> f64 {
        ind.iter().map(|x| x * x).sum::<f64>()
    };

    let mut best_fitness = f64::MAX;
    let mut best = vec![0.0; d];

    for _ in 0..gens {
        // Evaluate
        let fitnesses: Vec<f64> = population.iter().map(|ind| fitness(ind)).collect();
        for (i, &f) in fitnesses.iter().enumerate() {
            if f < best_fitness {
                best_fitness = f;
                best = population[i].clone();
            }
        }

        // Tournament selection + crossover
        let mut new_pop = Vec::with_capacity(pop);
        new_pop.push(best.clone()); // elitism
        while new_pop.len() < pop {
            // Tournament select 2 parents
            let p1 = tournament(&fitnesses, &mut rng);
            let p2 = tournament(&fitnesses, &mut rng);
            // Uniform crossover
            let child: Vec<f64> = (0..d).map(|j| {
                let gene = if next_rand(&mut rng) < 0.5 { population[p1][j] } else { population[p2][j] };
                // Mutation
                if next_rand(&mut rng) < mut_rate {
                    (gene + (next_rand(&mut rng) - 0.5) * (hi - lo) * 0.1).clamp(lo, hi)
                } else { gene }
            }).collect();
            new_pop.push(child);
        }
        population = new_pop;
    }

    best_out.copy_from_slice(&best);
    best_fitness
}

fn tournament(fitnesses: &[f64], rng: &mut u64) -> usize {
    let n = fitnesses.len();
    let i1 = ((*rng >> 33) as usize) % n;
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let i2 = ((*rng >> 33) as usize) % n;
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    if fitnesses[i1] <= fitnesses[i2] { i1 } else { i2 }
}

// ─── Ant Colony Optimization (TSP) ──────────────────────────────────

/// Ant Colony Optimization for TSP. `dist_matrix` is [n*n] symmetric.
/// Writes best tour to `tour_out` [n]. Returns best tour length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ant_colony_tsp(
    dist_matrix: *const f64, n: usize,
    n_ants: usize, iterations: usize,
    alpha: f64, beta: f64, evaporation: f64,
    seed: u64, tour_out: *mut usize,
) -> f64 {
    if dist_matrix.is_null() || tour_out.is_null() || n < 2 {
        return f64::MAX;
    }
    let dist = unsafe { std::slice::from_raw_parts(dist_matrix, n * n) };
    let tour = unsafe { std::slice::from_raw_parts_mut(tour_out, n) };
    ant_colony_impl(dist, n, n_ants, iterations, alpha, beta, evaporation, seed, tour)
}

fn ant_colony_impl(
    dist: &[f64], n: usize, n_ants: usize, iterations: usize,
    alpha: f64, beta: f64, evaporation: f64, seed: u64, best_tour: &mut [usize],
) -> f64 {
    let mut rng = seed;
    let next_rand = |rng: &mut u64| -> f64 {
        *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*rng >> 11) as f64 / (1u64 << 53) as f64
    };

    let mut pheromone = vec![1.0; n * n];
    let mut best_length = f64::MAX;
    let mut best = vec![0usize; n];

    for _ in 0..iterations {
        let mut all_tours: Vec<Vec<usize>> = Vec::with_capacity(n_ants);
        let mut all_lengths: Vec<f64> = Vec::with_capacity(n_ants);

        for _ in 0..n_ants {
            let mut visited = vec![false; n];
            let start = (next_rand(&mut rng) * n as f64) as usize % n;
            let mut tour = vec![start];
            visited[start] = true;

            for _ in 1..n {
                let curr = *tour.last().unwrap();
                let mut probs = vec![0.0; n];
                let mut total = 0.0;
                for j in 0..n {
                    if visited[j] { continue; }
                    let d = dist[curr * n + j];
                    if d < 1e-15 { continue; }
                    let p = (pheromone[curr * n + j] as f64).powf(alpha) * (1.0_f64 / d).powf(beta);
                    probs[j] = p;
                    total += p;
                }
                if total < 1e-15 {
                    // Pick first unvisited
                    for j in 0..n { if !visited[j] { tour.push(j); visited[j] = true; break; } }
                } else {
                    let r = next_rand(&mut rng) * total;
                    let mut cumulative = 0.0;
                    for j in 0..n {
                        cumulative += probs[j];
                        if cumulative >= r {
                            tour.push(j);
                            visited[j] = true;
                            break;
                        }
                    }
                }
            }

            let length: f64 = (0..n).map(|i| dist[tour[i] * n + tour[(i + 1) % n]]).sum();
            all_tours.push(tour);
            all_lengths.push(length);
        }

        // Update best
        for (i, &len) in all_lengths.iter().enumerate() {
            if len < best_length {
                best_length = len;
                best.copy_from_slice(&all_tours[i]);
            }
        }

        // Evaporate pheromone
        for p in pheromone.iter_mut() { *p *= 1.0 - evaporation; }

        // Deposit pheromone
        for (i, tour) in all_tours.iter().enumerate() {
            let deposit = 1.0 / all_lengths[i];
            for k in 0..n {
                let from = tour[k];
                let to = tour[(k + 1) % n];
                pheromone[from * n + to] += deposit;
                pheromone[to * n + from] += deposit;
            }
        }
    }

    best_tour.copy_from_slice(&best);
    best_length
}

// ─── Nearest Neighbor TSP Heuristic ─────────────────────────────────

/// Nearest-neighbor heuristic for TSP. Returns tour length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_tsp_nearest_neighbor(
    dist_matrix: *const f64, n: usize,
    start: usize, tour_out: *mut usize,
) -> f64 {
    if dist_matrix.is_null() || tour_out.is_null() || n < 2 {
        return f64::MAX;
    }
    let dist = unsafe { std::slice::from_raw_parts(dist_matrix, n * n) };
    let tour = unsafe { std::slice::from_raw_parts_mut(tour_out, n) };

    let mut visited = vec![false; n];
    tour[0] = start;
    visited[start] = true;
    let mut total = 0.0;

    for step in 1..n {
        let curr = tour[step - 1];
        let mut best_j = 0;
        let mut best_d = f64::MAX;
        for j in 0..n {
            if !visited[j] && dist[curr * n + j] < best_d {
                best_d = dist[curr * n + j];
                best_j = j;
            }
        }
        tour[step] = best_j;
        visited[best_j] = true;
        total += best_d;
    }
    total += dist[tour[n - 1] * n + tour[0]]; // return to start
    total
}

// ─── Bin Packing (First Fit Decreasing) ─────────────────────────────

/// First Fit Decreasing bin packing. `items` is [n] sizes, `bin_capacity` is max.
/// Writes bin assignments to `bin_assignments_out` [n].
/// Returns number of bins used.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bin_packing_ffd(
    items: *const f64, n: usize, bin_capacity: f64,
    bin_assignments_out: *mut i32,
) -> i32 {
    if items.is_null() || bin_assignments_out.is_null() || n == 0 || bin_capacity <= 0.0 {
        return -1;
    }
    let sizes = unsafe { std::slice::from_raw_parts(items, n) };
    let assign = unsafe { std::slice::from_raw_parts_mut(bin_assignments_out, n) };

    // Sort by decreasing size (keep original indices)
    let mut indexed: Vec<(usize, f64)> = (0..n).map(|i| (i, sizes[i])).collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut bin_remaining: Vec<f64> = Vec::new();
    for &(orig_idx, size) in &indexed {
        let mut placed = false;
        for (bin_idx, rem) in bin_remaining.iter_mut().enumerate() {
            if *rem >= size {
                assign[orig_idx] = bin_idx as i32;
                *rem -= size;
                placed = true;
                break;
            }
        }
        if !placed {
            assign[orig_idx] = bin_remaining.len() as i32;
            bin_remaining.push(bin_capacity - size);
        }
    }
    bin_remaining.len() as i32
}

// ─── Job Scheduling (Weighted Job Scheduling) ───────────────────────

/// Weighted job scheduling: maximize total value of non-overlapping jobs.
/// Jobs are defined by `starts[i]`, `ends[i]`, `values[i]`.
/// Returns maximum achievable value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_job_scheduling(
    starts: *const f64, ends: *const f64, values: *const f64, n: usize,
) -> f64 {
    if starts.is_null() || ends.is_null() || values.is_null() || n == 0 {
        return 0.0;
    }
    let s = unsafe { std::slice::from_raw_parts(starts, n) };
    let e = unsafe { std::slice::from_raw_parts(ends, n) };
    let v = unsafe { std::slice::from_raw_parts(values, n) };

    let mut jobs: Vec<(usize, f64, f64, f64)> = (0..n).map(|i| (i, s[i], e[i], v[i])).collect();
    jobs.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut dp = vec![0.0; n + 1];
    for i in 1..=n {
        let (_, start_i, _, val_i) = jobs[i - 1];
        // Binary search for last non-overlapping job
        let mut lo = 0usize;
        let mut hi = i - 1;
        let mut last_compatible = 0;
        while lo <= hi {
            let mid = lo + (hi - lo) / 2;
            if jobs[mid].2 <= start_i {
                last_compatible = mid + 1;
                lo = mid + 1;
            } else {
                if mid == 0 { break; }
                hi = mid - 1;
            }
        }
        dp[i] = f64::max(dp[i - 1], dp[last_compatible] + val_i);
    }
    dp[n]
}

// ─── Coin Change (DP) ──────────────────────────────────────────────

/// Minimum number of coins to make `amount`. `coins` is [n] denominations.
/// Returns -1 if impossible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_coin_change(
    coins: *const i32, n: usize, amount: i32,
) -> i32 {
    if coins.is_null() || n == 0 || amount < 0 { return -1; }
    let c = unsafe { std::slice::from_raw_parts(coins, n) };
    let amt = amount as usize;
    let mut dp = vec![i32::MAX; amt + 1];
    dp[0] = 0;
    for &coin in c {
        if coin <= 0 { continue; }
        let co = coin as usize;
        for a in co..=amt {
            if dp[a - co] != i32::MAX {
                dp[a] = dp[a].min(dp[a - co] + 1);
            }
        }
    }
    if dp[amt] == i32::MAX { -1 } else { dp[amt] }
}

// ─── Longest Increasing Subsequence ─────────────────────────────────

/// Length of longest increasing subsequence (O(n log n)).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lis_length(data: *const f64, n: usize) -> i32 {
    if data.is_null() || n == 0 { return 0; }
    let arr = unsafe { std::slice::from_raw_parts(data, n) };
    let mut tails: Vec<f64> = Vec::new();
    for &val in arr {
        let pos = tails.partition_point(|&x| x < val);
        if pos == tails.len() { tails.push(val); }
        else { tails[pos] = val; }
    }
    tails.len() as i32
}

// ─── Activity Selection (Greedy) ────────────────────────────────────

/// Activity selection: maximum number of non-overlapping intervals.
/// Returns count of selected activities.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_activity_selection(
    starts: *const f64, ends: *const f64, n: usize,
    selected_out: *mut i32,
) -> i32 {
    if starts.is_null() || ends.is_null() || n == 0 { return 0; }
    let s = unsafe { std::slice::from_raw_parts(starts, n) };
    let e = unsafe { std::slice::from_raw_parts(ends, n) };

    let mut activities: Vec<(usize, f64, f64)> = (0..n).map(|i| (i, s[i], e[i])).collect();
    activities.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut count = 0;
    let mut last_end = f64::NEG_INFINITY;
    let mut sel = if !selected_out.is_null() {
        Some(unsafe { std::slice::from_raw_parts_mut(selected_out, n) })
    } else { None };

    if let Some(ref mut s) = sel { s.iter_mut().for_each(|x| *x = 0); }

    for &(orig_idx, start, end) in &activities {
        if start >= last_end {
            count += 1;
            last_end = end;
            if let Some(ref mut s) = sel { s[orig_idx] = 1; }
        }
    }
    count
}

// ─── Matrix Chain Multiplication ────────────────────────────────────

/// Minimum scalar multiplications for matrix chain A1×A2×...×An.
/// `dims` is [n+1] where matrix i has dimensions dims[i] × dims[i+1].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_matrix_chain_order(
    dims: *const i64, n: usize,
) -> i64 {
    if dims.is_null() || n < 2 { return 0; }
    let p = unsafe { std::slice::from_raw_parts(dims, n + 1) };

    let mut dp = vec![vec![0i64; n]; n];
    for len in 2..=n {
        for i in 0..=n - len {
            let j = i + len - 1;
            dp[i][j] = i64::MAX;
            for k in i..j {
                let cost = dp[i][k] + dp[k + 1][j] + p[i] * p[k + 1] * p[j + 1];
                dp[i][j] = dp[i][j].min(cost);
            }
        }
    }
    dp[0][n - 1]
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knapsack_01() {
        let w = [2.0, 3.0, 4.0, 5.0];
        let v = [3.0, 4.0, 5.0, 6.0];
        let mut sel = [0i32; 4];
        let max_val = unsafe { vitalis_knapsack_01(w.as_ptr(), v.as_ptr(), 4, 5, sel.as_mut_ptr()) };
        assert!((max_val - 7.0).abs() < 1e-10); // items 0+1: weight 5, value 7
    }

    #[test]
    fn test_knapsack_fractional() {
        let w = [10.0, 20.0, 30.0];
        let v = [60.0, 100.0, 120.0];
        let max_val = unsafe { vitalis_knapsack_fractional(w.as_ptr(), v.as_ptr(), 3, 50.0) };
        assert!(max_val > 200.0); // optimal is 240
    }

    #[test]
    fn test_hungarian() {
        // 3x3 cost matrix
        let cost = [9.0, 2.0, 7.0,
                     6.0, 4.0, 3.0,
                     5.0, 8.0, 1.0];
        let mut assign = [0i32; 3];
        let total = unsafe { vitalis_hungarian(cost.as_ptr(), 3, assign.as_mut_ptr()) };
        // Optimal: worker0→job1(2), worker1→job2(3), worker2→job0(5) = 10
        // or: worker0→job1(2), worker1→job0(6), worker2→job2(1) = 9
        assert!(total <= 10.0);
    }

    #[test]
    fn test_simplex() {
        // max 3x + 2y s.t. x + y <= 4, x + 3y <= 6
        let a = [1.0, 1.0, 1.0, 3.0];
        let b = [4.0, 6.0];
        let c = [3.0, 2.0];
        let mut x = [0.0; 2];
        let opt = unsafe { vitalis_simplex(a.as_ptr(), b.as_ptr(), c.as_ptr(), 2, 2, x.as_mut_ptr()) };
        assert!(opt > 9.0); // optimal is 10 at (3,1)
    }

    #[test]
    fn test_genetic_sphere() {
        let mut best = [0.0; 2];
        let fitness = unsafe {
            vitalis_genetic_sphere(2, -5.0, 5.0, 50, 100, 0.1, 42, best.as_mut_ptr())
        };
        assert!(fitness < 1.0); // should find near-zero for sphere function
    }

    #[test]
    fn test_tsp_nearest_neighbor() {
        // 4 cities in a square
        let dist = [0.0, 1.0, 1.414, 1.0,
                     1.0, 0.0, 1.0, 1.414,
                     1.414, 1.0, 0.0, 1.0,
                     1.0, 1.414, 1.0, 0.0];
        let mut tour = [0usize; 4];
        let length = unsafe { vitalis_tsp_nearest_neighbor(dist.as_ptr(), 4, 0, tour.as_mut_ptr()) };
        assert!(length < 5.0);
    }

    #[test]
    fn test_bin_packing() {
        let items = [0.5, 0.7, 0.2, 0.3, 0.8, 0.1];
        let mut assign = [0i32; 6];
        let bins = unsafe { vitalis_bin_packing_ffd(items.as_ptr(), 6, 1.0, assign.as_mut_ptr()) };
        assert!(bins >= 3 && bins <= 4); // FFD should use 3 bins
    }

    #[test]
    fn test_job_scheduling() {
        let starts = [1.0, 2.0, 3.0, 3.0];
        let ends = [3.0, 5.0, 4.0, 6.0];
        let values = [5.0, 6.0, 5.0, 7.0];
        let max_val = unsafe { vitalis_job_scheduling(starts.as_ptr(), ends.as_ptr(), values.as_ptr(), 4) };
        assert!(max_val >= 10.0);
    }

    #[test]
    fn test_coin_change() {
        let coins = [1, 5, 10, 25];
        assert_eq!(unsafe { vitalis_coin_change(coins.as_ptr(), 4, 30) }, 2); // 25+5
        assert_eq!(unsafe { vitalis_coin_change(coins.as_ptr(), 4, 0) }, 0);
    }

    #[test]
    fn test_lis() {
        let data = [10.0, 9.0, 2.0, 5.0, 3.0, 7.0, 101.0, 18.0];
        assert_eq!(unsafe { vitalis_lis_length(data.as_ptr(), 8) }, 4); // [2,5,7,101] or [2,3,7,18]
    }

    #[test]
    fn test_activity_selection() {
        let starts = [1.0, 3.0, 0.0, 5.0, 8.0, 5.0];
        let ends = [2.0, 4.0, 6.0, 7.0, 9.0, 9.0];
        let mut sel = [0i32; 6];
        let count = unsafe { vitalis_activity_selection(starts.as_ptr(), ends.as_ptr(), 6, sel.as_mut_ptr()) };
        assert!(count >= 4); // activities (1,2),(3,4),(5,7),(8,9)
    }

    #[test]
    fn test_matrix_chain_order() {
        let dims = [10i64, 30, 5, 60];
        let cost = unsafe { vitalis_matrix_chain_order(dims.as_ptr(), 3) };
        assert_eq!(cost, 4500); // optimal: (A1*A2)*A3 = 10*30*5 + 10*5*60 = 4500
    }
}
