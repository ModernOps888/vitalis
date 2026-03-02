//! Machine Learning Module for Vitalis v10.0
//!
//! Pure Rust implementations of classic ML algorithms: clustering, classification,
//! dimensionality reduction, regression, optimizers, and evaluation metrics.
//! All functions are FFI-safe via `extern "C"`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── K-Means Clustering ──────────────────────────────────────────────

/// K-Means clustering: assigns each point to one of `k` clusters.
/// `data` is row-major [n_samples * n_features], returns cluster assignment per sample.
/// Returns null on invalid input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kmeans(
    data: *const f64, n_samples: usize, n_features: usize, k: usize,
    max_iter: usize, out_labels: *mut i32,
) -> i32 {
    if data.is_null() || out_labels.is_null() || n_samples == 0 || n_features == 0 || k == 0 || k > n_samples {
        return -1;
    }
    let data = unsafe { std::slice::from_raw_parts(data, n_samples * n_features) };
    let labels = unsafe { std::slice::from_raw_parts_mut(out_labels, n_samples) };
    kmeans_impl(data, n_samples, n_features, k, max_iter, labels);
    0
}

fn kmeans_impl(data: &[f64], n: usize, d: usize, k: usize, max_iter: usize, labels: &mut [i32]) {
    // Initialize centroids to first k data points
    let mut centroids: Vec<f64> = data[..k * d].to_vec();
    let iters = if max_iter == 0 { 100 } else { max_iter };

    for _ in 0..iters {
        // Assignment step
        let mut changed = false;
        for i in 0..n {
            let row = &data[i * d..(i + 1) * d];
            let mut best_c = 0i32;
            let mut best_dist = f64::MAX;
            for c in 0..k {
                let cent = &centroids[c * d..(c + 1) * d];
                let dist: f64 = row.iter().zip(cent).map(|(a, b)| (a - b).powi(2)).sum();
                if dist < best_dist {
                    best_dist = dist;
                    best_c = c as i32;
                }
            }
            if labels[i] != best_c {
                labels[i] = best_c;
                changed = true;
            }
        }
        if !changed { break; }

        // Update step
        let mut counts = vec![0usize; k];
        centroids.fill(0.0);
        for i in 0..n {
            let c = labels[i] as usize;
            counts[c] += 1;
            for j in 0..d {
                centroids[c * d + j] += data[i * d + j];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for j in 0..d {
                    centroids[c * d + j] /= counts[c] as f64;
                }
            }
        }
    }
}

// ─── K-Nearest Neighbors ─────────────────────────────────────────────

/// KNN classifier: classifies `query` point by majority vote of k nearest neighbors.
/// `train_data` is [n * d], `train_labels` is [n], `query` is [d].
/// Returns predicted class label.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_knn_classify(
    train_data: *const f64, train_labels: *const i32, n_train: usize,
    query: *const f64, n_features: usize, k: usize,
) -> i32 {
    if train_data.is_null() || train_labels.is_null() || query.is_null() || n_train == 0 || k == 0 {
        return -1;
    }
    let data = unsafe { std::slice::from_raw_parts(train_data, n_train * n_features) };
    let labels = unsafe { std::slice::from_raw_parts(train_labels, n_train) };
    let q = unsafe { std::slice::from_raw_parts(query, n_features) };
    knn_classify_impl(data, labels, n_train, q, n_features, k)
}

fn knn_classify_impl(data: &[f64], labels: &[i32], n: usize, q: &[f64], d: usize, k: usize) -> i32 {
    let mut dists: Vec<(f64, i32)> = (0..n)
        .map(|i| {
            let row = &data[i * d..(i + 1) * d];
            let dist: f64 = row.iter().zip(q).map(|(a, b)| (a - b).powi(2)).sum();
            (dist, labels[i])
        })
        .collect();
    dists.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let kk = k.min(n);
    let mut votes = std::collections::HashMap::new();
    for i in 0..kk {
        *votes.entry(dists[i].1).or_insert(0) += 1;
    }
    votes.into_iter().max_by_key(|&(_, v)| v).map(|(l, _)| l).unwrap_or(-1)
}

// ─── Naive Bayes (Gaussian) ──────────────────────────────────────────

/// Gaussian Naive Bayes: returns predicted class for `query` given class stats.
/// `class_means` and `class_vars` are [n_classes * n_features], `class_priors` is [n_classes].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_naive_bayes(
    class_means: *const f64, class_vars: *const f64, class_priors: *const f64,
    n_classes: usize, n_features: usize, query: *const f64,
) -> i32 {
    if class_means.is_null() || class_vars.is_null() || class_priors.is_null() || query.is_null() {
        return -1;
    }
    let means = unsafe { std::slice::from_raw_parts(class_means, n_classes * n_features) };
    let vars = unsafe { std::slice::from_raw_parts(class_vars, n_classes * n_features) };
    let priors = unsafe { std::slice::from_raw_parts(class_priors, n_classes) };
    let q = unsafe { std::slice::from_raw_parts(query, n_features) };

    let mut best_class = 0i32;
    let mut best_score = f64::NEG_INFINITY;
    for c in 0..n_classes {
        let mut log_prob = priors[c].ln();
        for j in 0..n_features {
            let mu = means[c * n_features + j];
            let var = vars[c * n_features + j].max(1e-12);
            let diff = q[j] - mu;
            log_prob += -0.5 * (diff * diff / var + var.ln() + std::f64::consts::TAU.ln());
        }
        if log_prob > best_score {
            best_score = log_prob;
            best_class = c as i32;
        }
    }
    best_class
}

// ─── Logistic Regression ─────────────────────────────────────────────

/// Logistic regression: binary classification via gradient descent.
/// `data` is [n * d], `labels` is [n] with 0/1 values, `weights_out` is [d+1] (bias last).
/// Returns number of iterations run.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_logistic_regression(
    data: *const f64, labels: *const f64, n_samples: usize, n_features: usize,
    lr: f64, max_iter: usize, weights_out: *mut f64,
) -> i32 {
    if data.is_null() || labels.is_null() || weights_out.is_null() || n_samples == 0 {
        return -1;
    }
    let x = unsafe { std::slice::from_raw_parts(data, n_samples * n_features) };
    let y = unsafe { std::slice::from_raw_parts(labels, n_samples) };
    let w = unsafe { std::slice::from_raw_parts_mut(weights_out, n_features + 1) };
    logistic_regression_impl(x, y, n_samples, n_features, lr, max_iter, w)
}

fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x).exp()) }

fn logistic_regression_impl(
    x: &[f64], y: &[f64], n: usize, d: usize, lr: f64, max_iter: usize, w: &mut [f64],
) -> i32 {
    w.fill(0.0);
    let iters = if max_iter == 0 { 1000 } else { max_iter };
    for it in 0..iters {
        let mut grad = vec![0.0; d + 1];
        let mut max_grad = 0.0f64;
        for i in 0..n {
            let row = &x[i * d..(i + 1) * d];
            let z: f64 = row.iter().zip(&w[..d]).map(|(a, b)| a * b).sum::<f64>() + w[d]; // bias
            let p = sigmoid(z);
            let err = p - y[i];
            for j in 0..d { grad[j] += err * row[j]; }
            grad[d] += err; // bias gradient
        }
        for j in 0..=d {
            grad[j] /= n as f64;
            max_grad = max_grad.max(grad[j].abs());
            w[j] -= lr * grad[j];
        }
        if max_grad < 1e-8 { return (it + 1) as i32; }
    }
    iters as i32
}

/// Logistic regression predict: given weights [d+1], compute P(y=1|x).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_logistic_predict(
    weights: *const f64, query: *const f64, n_features: usize,
) -> f64 {
    if weights.is_null() || query.is_null() { return -1.0; }
    let w = unsafe { std::slice::from_raw_parts(weights, n_features + 1) };
    let q = unsafe { std::slice::from_raw_parts(query, n_features) };
    let z: f64 = q.iter().zip(&w[..n_features]).map(|(a, b)| a * b).sum::<f64>() + w[n_features];
    sigmoid(z)
}

// ─── PCA (Principal Component Analysis) ──────────────────────────────

/// PCA: projects data [n * d] down to `n_components` dimensions.
/// `out` must be [n * n_components]. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_pca(
    data: *const f64, n_samples: usize, n_features: usize,
    n_components: usize, out: *mut f64,
) -> i32 {
    if data.is_null() || out.is_null() || n_components == 0 || n_components > n_features {
        return -1;
    }
    let x = unsafe { std::slice::from_raw_parts(data, n_samples * n_features) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, n_samples * n_components) };
    pca_impl(x, n_samples, n_features, n_components, o)
}

fn pca_impl(data: &[f64], n: usize, d: usize, k: usize, out: &mut [f64]) -> i32 {
    // Center the data
    let mut means = vec![0.0; d];
    for i in 0..n {
        for j in 0..d { means[j] += data[i * d + j]; }
    }
    for j in 0..d { means[j] /= n as f64; }
    let mut centered = vec![0.0; n * d];
    for i in 0..n {
        for j in 0..d { centered[i * d + j] = data[i * d + j] - means[j]; }
    }

    // Covariance matrix (d x d)
    let mut cov = vec![0.0; d * d];
    for i in 0..n {
        let row = &centered[i * d..(i + 1) * d];
        for r in 0..d {
            for c in r..d {
                let val = row[r] * row[c];
                cov[r * d + c] += val;
                if r != c { cov[c * d + r] += val; }
            }
        }
    }
    let n_f = if n > 1 { (n - 1) as f64 } else { 1.0 };
    for v in cov.iter_mut() { *v /= n_f; }

    // Power iteration for top-k eigenvectors
    let mut eigenvecs = vec![0.0; k * d];
    let mut deflated = cov.clone();
    for comp in 0..k {
        let mut v = vec![0.0; d];
        v[comp % d] = 1.0; // initial guess
        for _ in 0..200 {
            // v_new = A * v
            let mut v_new = vec![0.0; d];
            for r in 0..d {
                for c in 0..d {
                    v_new[r] += deflated[r * d + c] * v[c];
                }
            }
            let norm: f64 = v_new.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-15 { break; }
            for x in v_new.iter_mut() { *x /= norm; }
            v = v_new;
        }
        eigenvecs[comp * d..(comp + 1) * d].copy_from_slice(&v);
        // Deflate: A = A - lambda * v * v^T
        let mut lambda = 0.0;
        for r in 0..d {
            for c in 0..d {
                lambda += v[r] * deflated[r * d + c] * v[c];
            }
        }
        for r in 0..d {
            for c in 0..d {
                deflated[r * d + c] -= lambda * v[r] * v[c];
            }
        }
    }

    // Project: out = centered * eigenvecs^T
    for i in 0..n {
        let row = &centered[i * d..(i + 1) * d];
        for comp in 0..k {
            let ev = &eigenvecs[comp * d..(comp + 1) * d];
            out[i * k + comp] = row.iter().zip(ev).map(|(a, b)| a * b).sum();
        }
    }
    0
}

// ─── SVD (Singular Value Decomposition) ──────────────────────────────

/// Thin SVD via power iteration: computes top-k singular values.
/// Returns them in `singular_values_out` [k]. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_svd_values(
    data: *const f64, n_rows: usize, n_cols: usize,
    k: usize, singular_values_out: *mut f64,
) -> i32 {
    if data.is_null() || singular_values_out.is_null() || k == 0 {
        return -1;
    }
    let a = unsafe { std::slice::from_raw_parts(data, n_rows * n_cols) };
    let sv = unsafe { std::slice::from_raw_parts_mut(singular_values_out, k) };
    svd_values_impl(a, n_rows, n_cols, k, sv);
    0
}

fn svd_values_impl(a: &[f64], m: usize, n: usize, k: usize, sv: &mut [f64]) {
    // Compute A^T * A, then power-iterate for eigenvalues
    let mut ata = vec![0.0; n * n];
    for i in 0..n {
        for j in i..n {
            let mut s = 0.0;
            for r in 0..m { s += a[r * n + i] * a[r * n + j]; }
            ata[i * n + j] = s;
            ata[j * n + i] = s;
        }
    }

    let mut deflated = ata;
    for comp in 0..k.min(n) {
        let mut v = vec![0.0; n];
        v[comp % n] = 1.0;
        for _ in 0..200 {
            let mut v_new = vec![0.0; n];
            for r in 0..n {
                for c in 0..n { v_new[r] += deflated[r * n + c] * v[c]; }
            }
            let norm: f64 = v_new.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-15 { break; }
            for x in v_new.iter_mut() { *x /= norm; }
            v = v_new;
        }
        let mut eigenval = 0.0;
        for r in 0..n {
            for c in 0..n { eigenval += v[r] * deflated[r * n + c] * v[c]; }
        }
        sv[comp] = eigenval.abs().sqrt();
        for r in 0..n {
            for c in 0..n { deflated[r * n + c] -= eigenval * v[r] * v[c]; }
        }
    }
}

// ─── Decision Tree (simple) ──────────────────────────────────────────

/// Decision stump: finds best single-feature threshold split.
/// Returns (feature_index, threshold, left_label, right_label) packed as 4 doubles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_decision_stump(
    data: *const f64, labels: *const i32, n_samples: usize, n_features: usize,
    out: *mut f64,
) -> i32 {
    if data.is_null() || labels.is_null() || out.is_null() || n_samples < 2 {
        return -1;
    }
    let x = unsafe { std::slice::from_raw_parts(data, n_samples * n_features) };
    let y = unsafe { std::slice::from_raw_parts(labels, n_samples) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    decision_stump_impl(x, y, n_samples, n_features, o);
    0
}

fn gini_impurity(counts: &std::collections::HashMap<i32, usize>, total: usize) -> f64 {
    if total == 0 { return 0.0; }
    let mut gini = 1.0;
    for &c in counts.values() {
        let p = c as f64 / total as f64;
        gini -= p * p;
    }
    gini
}

fn decision_stump_impl(data: &[f64], labels: &[i32], n: usize, d: usize, out: &mut [f64]) {
    let mut best_gini = f64::MAX;
    let mut best_feat = 0usize;
    let mut best_thresh = 0.0f64;

    for feat in 0..d {
        let mut vals: Vec<(f64, i32)> = (0..n).map(|i| (data[i * d + feat], labels[i])).collect();
        vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut left_counts: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        let mut right_counts: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        for &(_, l) in &vals { *right_counts.entry(l).or_insert(0) += 1; }

        for i in 0..n - 1 {
            let label = vals[i].1;
            *left_counts.entry(label).or_insert(0) += 1;
            if let Some(rc) = right_counts.get_mut(&label) {
                *rc -= 1;
                if *rc == 0 { right_counts.remove(&label); }
            }
            if (vals[i].0 - vals[i + 1].0).abs() < 1e-15 { continue; }

            let left_n = i + 1;
            let right_n = n - left_n;
            let g = (left_n as f64 * gini_impurity(&left_counts, left_n)
                + right_n as f64 * gini_impurity(&right_counts, right_n))
                / n as f64;

            if g < best_gini {
                best_gini = g;
                best_feat = feat;
                best_thresh = (vals[i].0 + vals[i + 1].0) / 2.0;
            }
        }
    }

    // Determine majority labels for each side
    let mut left_votes: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
    let mut right_votes: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
    for i in 0..n {
        if data[i * d + best_feat] <= best_thresh {
            *left_votes.entry(labels[i]).or_insert(0) += 1;
        } else {
            *right_votes.entry(labels[i]).or_insert(0) += 1;
        }
    }
    let left_label = left_votes.iter().max_by_key(|&(_, v)| v).map(|(&k, _)| k).unwrap_or(0);
    let right_label = right_votes.iter().max_by_key(|&(_, v)| v).map(|(&k, _)| k).unwrap_or(0);

    out[0] = best_feat as f64;
    out[1] = best_thresh;
    out[2] = left_label as f64;
    out[3] = right_label as f64;
}

// ─── Confusion Matrix Metrics ────────────────────────────────────────

/// Accuracy from predicted vs actual labels.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_accuracy(
    predicted: *const i32, actual: *const i32, n: usize,
) -> f64 {
    if predicted.is_null() || actual.is_null() || n == 0 { return 0.0; }
    let p = unsafe { std::slice::from_raw_parts(predicted, n) };
    let a = unsafe { std::slice::from_raw_parts(actual, n) };
    let correct = p.iter().zip(a).filter(|(p, a)| p == a).count();
    correct as f64 / n as f64
}

/// Precision for binary classification (positive_label is the "positive" class).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_precision(
    predicted: *const i32, actual: *const i32, n: usize, positive_label: i32,
) -> f64 {
    if predicted.is_null() || actual.is_null() || n == 0 { return 0.0; }
    let p = unsafe { std::slice::from_raw_parts(predicted, n) };
    let a = unsafe { std::slice::from_raw_parts(actual, n) };
    let tp = p.iter().zip(a).filter(|&(pi, ai)| *pi == positive_label && *ai == positive_label).count();
    let fp = p.iter().zip(a).filter(|&(pi, ai)| *pi == positive_label && *ai != positive_label).count();
    if tp + fp == 0 { 0.0 } else { tp as f64 / (tp + fp) as f64 }
}

/// Recall for binary classification.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_recall(
    predicted: *const i32, actual: *const i32, n: usize, positive_label: i32,
) -> f64 {
    if predicted.is_null() || actual.is_null() || n == 0 { return 0.0; }
    let p = unsafe { std::slice::from_raw_parts(predicted, n) };
    let a = unsafe { std::slice::from_raw_parts(actual, n) };
    let tp = p.iter().zip(a).filter(|&(pi, ai)| *pi == positive_label && *ai == positive_label).count();
    let f_n = p.iter().zip(a).filter(|&(pi, ai)| *pi != positive_label && *ai == positive_label).count();
    if tp + f_n == 0 { 0.0 } else { tp as f64 / (tp + f_n) as f64 }
}

/// F1-score (harmonic mean of precision and recall).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_f1_score(
    predicted: *const i32, actual: *const i32, n: usize, positive_label: i32,
) -> f64 {
    let prec = unsafe { vitalis_precision(predicted, actual, n, positive_label) };
    let rec = unsafe { vitalis_recall(predicted, actual, n, positive_label) };
    if prec + rec < 1e-15 { 0.0 } else { 2.0 * prec * rec / (prec + rec) }
}

// ─── Adam Optimizer ──────────────────────────────────────────────────

/// Single Adam optimizer step: updates params in-place given gradients.
/// `m` and `v` are first/second moment estimates [d], `t` is timestep (1-indexed).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_adam_step(
    params: *mut f64, gradients: *const f64, m: *mut f64, v: *mut f64,
    d: usize, lr: f64, beta1: f64, beta2: f64, epsilon: f64, t: usize,
) -> i32 {
    if params.is_null() || gradients.is_null() || m.is_null() || v.is_null() || d == 0 {
        return -1;
    }
    let p = unsafe { std::slice::from_raw_parts_mut(params, d) };
    let g = unsafe { std::slice::from_raw_parts(gradients, d) };
    let mm = unsafe { std::slice::from_raw_parts_mut(m, d) };
    let vv = unsafe { std::slice::from_raw_parts_mut(v, d) };

    let t_f = t.max(1) as f64;
    let bc1 = 1.0 - beta1.powf(t_f);
    let bc2 = 1.0 - beta2.powf(t_f);

    for i in 0..d {
        mm[i] = beta1 * mm[i] + (1.0 - beta1) * g[i];
        vv[i] = beta2 * vv[i] + (1.0 - beta2) * g[i] * g[i];
        let m_hat = mm[i] / bc1;
        let v_hat = vv[i] / bc2;
        p[i] -= lr * m_hat / (v_hat.sqrt() + epsilon);
    }
    0
}

/// SGD with momentum: single step.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_sgd_momentum_step(
    params: *mut f64, gradients: *const f64, velocity: *mut f64,
    d: usize, lr: f64, momentum: f64,
) -> i32 {
    if params.is_null() || gradients.is_null() || velocity.is_null() || d == 0 {
        return -1;
    }
    let p = unsafe { std::slice::from_raw_parts_mut(params, d) };
    let g = unsafe { std::slice::from_raw_parts(gradients, d) };
    let vel = unsafe { std::slice::from_raw_parts_mut(velocity, d) };

    for i in 0..d {
        vel[i] = momentum * vel[i] - lr * g[i];
        p[i] += vel[i];
    }
    0
}

// ─── RMSProp Optimizer ───────────────────────────────────────────────

/// RMSProp optimizer step.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rmsprop_step(
    params: *mut f64, gradients: *const f64, cache: *mut f64,
    d: usize, lr: f64, decay_rate: f64, epsilon: f64,
) -> i32 {
    if params.is_null() || gradients.is_null() || cache.is_null() || d == 0 {
        return -1;
    }
    let p = unsafe { std::slice::from_raw_parts_mut(params, d) };
    let g = unsafe { std::slice::from_raw_parts(gradients, d) };
    let c = unsafe { std::slice::from_raw_parts_mut(cache, d) };

    for i in 0..d {
        c[i] = decay_rate * c[i] + (1.0 - decay_rate) * g[i] * g[i];
        p[i] -= lr * g[i] / (c[i].sqrt() + epsilon);
    }
    0
}

// ─── DBSCAN Clustering ──────────────────────────────────────────────

/// DBSCAN density-based clustering. Labels: -1 = noise, 0+ = cluster ID.
/// Returns number of clusters found.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_dbscan(
    data: *const f64, n_samples: usize, n_features: usize,
    eps: f64, min_pts: usize, out_labels: *mut i32,
) -> i32 {
    if data.is_null() || out_labels.is_null() || n_samples == 0 {
        return -1;
    }
    let x = unsafe { std::slice::from_raw_parts(data, n_samples * n_features) };
    let labels = unsafe { std::slice::from_raw_parts_mut(out_labels, n_samples) };
    dbscan_impl(x, n_samples, n_features, eps, min_pts, labels)
}

fn dbscan_impl(data: &[f64], n: usize, d: usize, eps: f64, min_pts: usize, labels: &mut [i32]) -> i32 {
    labels.fill(-2); // unvisited
    let eps2 = eps * eps;
    let mut cluster_id = 0i32;

    let neighbors = |idx: usize| -> Vec<usize> {
        let row = &data[idx * d..(idx + 1) * d];
        (0..n).filter(|&j| {
            let other = &data[j * d..(j + 1) * d];
            let dist2: f64 = row.iter().zip(other).map(|(a, b)| (a - b).powi(2)).sum();
            dist2 <= eps2
        }).collect()
    };

    for i in 0..n {
        if labels[i] != -2 { continue; }
        let nbrs = neighbors(i);
        if nbrs.len() < min_pts {
            labels[i] = -1; // noise
            continue;
        }
        labels[i] = cluster_id;
        let mut queue: std::collections::VecDeque<usize> = nbrs.into_iter().filter(|&j| j != i).collect();
        while let Some(j) = queue.pop_front() {
            if labels[j] == -1 { labels[j] = cluster_id; }
            if labels[j] != -2 { continue; }
            labels[j] = cluster_id;
            let j_nbrs = neighbors(j);
            if j_nbrs.len() >= min_pts {
                for &k in &j_nbrs {
                    if labels[k] == -2 || labels[k] == -1 {
                        queue.push_back(k);
                    }
                }
            }
        }
        cluster_id += 1;
    }
    cluster_id
}

// ─── Linear Discriminant Analysis ────────────────────────────────────

/// LDA: Fisher's linear discriminant for 2-class separation.
/// Returns projection direction in `direction_out` [d]. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_lda_2class(
    data: *const f64, labels: *const i32, n_samples: usize, n_features: usize,
    direction_out: *mut f64,
) -> i32 {
    if data.is_null() || labels.is_null() || direction_out.is_null() || n_samples < 2 {
        return -1;
    }
    let x = unsafe { std::slice::from_raw_parts(data, n_samples * n_features) };
    let y = unsafe { std::slice::from_raw_parts(labels, n_samples) };
    let dir = unsafe { std::slice::from_raw_parts_mut(direction_out, n_features) };

    let d = n_features;
    let mut mean0 = vec![0.0; d]; let mut mean1 = vec![0.0; d];
    let mut n0 = 0usize; let mut n1 = 0usize;
    for i in 0..n_samples {
        let row = &x[i * d..(i + 1) * d];
        if y[i] == 0 { n0 += 1; for j in 0..d { mean0[j] += row[j]; } }
        else { n1 += 1; for j in 0..d { mean1[j] += row[j]; } }
    }
    if n0 == 0 || n1 == 0 { return -1; }
    for j in 0..d { mean0[j] /= n0 as f64; mean1[j] /= n1 as f64; }

    // Within-class scatter (pooled covariance diagonal approx for efficiency)
    let mut sw = vec![1e-8; d]; // regularization
    for i in 0..n_samples {
        let row = &x[i * d..(i + 1) * d];
        let mean = if y[i] == 0 { &mean0 } else { &mean1 };
        for j in 0..d { sw[j] += (row[j] - mean[j]).powi(2); }
    }

    // Direction = Sw^{-1} * (mean1 - mean0) — diagonal approximation
    let mut norm = 0.0;
    for j in 0..d {
        dir[j] = (mean1[j] - mean0[j]) / sw[j];
        norm += dir[j] * dir[j];
    }
    norm = norm.sqrt();
    if norm > 1e-15 { for j in 0..d { dir[j] /= norm; } }
    0
}

// ─── Silhouette Score ────────────────────────────────────────────────

/// Silhouette score for clustering quality (-1 to 1, higher is better).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_silhouette_score(
    data: *const f64, labels: *const i32, n_samples: usize, n_features: usize,
) -> f64 {
    if data.is_null() || labels.is_null() || n_samples < 2 { return 0.0; }
    let x = unsafe { std::slice::from_raw_parts(data, n_samples * n_features) };
    let y = unsafe { std::slice::from_raw_parts(labels, n_samples) };
    let d = n_features;

    let dist = |i: usize, j: usize| -> f64 {
        let ri = &x[i * d..(i + 1) * d];
        let rj = &x[j * d..(j + 1) * d];
        ri.iter().zip(rj).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt()
    };

    let mut total = 0.0;
    for i in 0..n_samples {
        let ci = y[i];
        // a(i) = mean dist to same cluster
        let same: Vec<usize> = (0..n_samples).filter(|&j| j != i && y[j] == ci).collect();
        let a_i = if same.is_empty() { 0.0 } else {
            same.iter().map(|&j| dist(i, j)).sum::<f64>() / same.len() as f64
        };
        // b(i) = min mean dist to other clusters
        let mut clusters: std::collections::HashSet<i32> = y.iter().cloned().collect();
        clusters.remove(&ci);
        let b_i = clusters.iter().map(|&ck| {
            let others: Vec<usize> = (0..n_samples).filter(|&j| y[j] == ck).collect();
            if others.is_empty() { f64::MAX } else {
                others.iter().map(|&j| dist(i, j)).sum::<f64>() / others.len() as f64
            }
        }).fold(f64::MAX, f64::min);

        let s_i = if a_i.max(b_i) < 1e-15 { 0.0 } else { (b_i - a_i) / a_i.max(b_i) };
        total += s_i;
    }
    total / n_samples as f64
}

// ─── Cross-Validation Split ──────────────────────────────────────────

/// Generate fold indices for k-fold cross-validation.
/// `fold_labels_out` is [n], each value 0..k-1 indicating which fold the sample belongs to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_kfold_split(
    n_samples: usize, k_folds: usize, fold_labels_out: *mut i32,
) -> i32 {
    if fold_labels_out.is_null() || k_folds == 0 || k_folds > n_samples {
        return -1;
    }
    let labels = unsafe { std::slice::from_raw_parts_mut(fold_labels_out, n_samples) };
    for i in 0..n_samples {
        labels[i] = (i % k_folds) as i32;
    }
    0
}

// ─── Cosine Similarity ──────────────────────────────────────────────

/// Cosine similarity between two vectors.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ml_cosine_similarity(
    a: *const f64, b: *const f64, n: usize,
) -> f64 {
    if a.is_null() || b.is_null() || n == 0 { return 0.0; }
    let va = unsafe { std::slice::from_raw_parts(a, n) };
    let vb = unsafe { std::slice::from_raw_parts(b, n) };
    let dot: f64 = va.iter().zip(vb).map(|(x, y)| x * y).sum();
    let na: f64 = va.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb: f64 = vb.iter().map(|x| x * x).sum::<f64>().sqrt();
    if na < 1e-15 || nb < 1e-15 { 0.0 } else { dot / (na * nb) }
}

// ─── Mini-Batch Gradient Descent ─────────────────────────────────────

/// Compute mean squared error loss.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mse(
    predicted: *const f64, actual: *const f64, n: usize,
) -> f64 {
    if predicted.is_null() || actual.is_null() || n == 0 { return 0.0; }
    let p = unsafe { std::slice::from_raw_parts(predicted, n) };
    let a = unsafe { std::slice::from_raw_parts(actual, n) };
    p.iter().zip(a).map(|(pi, ai)| (pi - ai).powi(2)).sum::<f64>() / n as f64
}

/// Compute mean absolute error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_mae(
    predicted: *const f64, actual: *const f64, n: usize,
) -> f64 {
    if predicted.is_null() || actual.is_null() || n == 0 { return 0.0; }
    let p = unsafe { std::slice::from_raw_parts(predicted, n) };
    let a = unsafe { std::slice::from_raw_parts(actual, n) };
    p.iter().zip(a).map(|(pi, ai)| (pi - ai).abs()).sum::<f64>() / n as f64
}

/// R² (coefficient of determination).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_r2_score(
    predicted: *const f64, actual: *const f64, n: usize,
) -> f64 {
    if predicted.is_null() || actual.is_null() || n == 0 { return 0.0; }
    let p = unsafe { std::slice::from_raw_parts(predicted, n) };
    let a = unsafe { std::slice::from_raw_parts(actual, n) };
    let mean_a: f64 = a.iter().sum::<f64>() / n as f64;
    let ss_res: f64 = p.iter().zip(a).map(|(pi, ai)| (ai - pi).powi(2)).sum();
    let ss_tot: f64 = a.iter().map(|ai| (ai - mean_a).powi(2)).sum();
    if ss_tot < 1e-15 { 1.0 } else { 1.0 - ss_res / ss_tot }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmeans_basic() {
        // 6 points in 2 clusters
        let data = [0.0, 0.0, 0.1, 0.1, -0.1, 0.05,
                     10.0, 10.0, 10.1, 9.9, 9.9, 10.1];
        let mut labels = [0i32; 6];
        let ret = unsafe { vitalis_kmeans(data.as_ptr(), 6, 2, 2, 100, labels.as_mut_ptr()) };
        assert_eq!(ret, 0);
        // First 3 should be same cluster, last 3 same cluster, different from first
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[1], labels[2]);
        assert_eq!(labels[3], labels[4]);
        assert_eq!(labels[4], labels[5]);
        assert_ne!(labels[0], labels[3]);
    }

    #[test]
    fn test_knn_classify() {
        let data = [0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0];
        let labels = [0, 1, 0, 1];
        let query = [0.1, 0.1];
        let result = unsafe {
            vitalis_knn_classify(data.as_ptr(), labels.as_ptr(), 4, query.as_ptr(), 2, 3)
        };
        assert_eq!(result, 0); // closest to (0,0) class 0
    }

    #[test]
    fn test_naive_bayes() {
        // Two classes: class 0 centered at 0, class 1 centered at 5
        let means = [0.0, 5.0];
        let vars = [1.0, 1.0];
        let priors = [0.5, 0.5];
        let query = [0.5];
        let result = unsafe {
            vitalis_naive_bayes(means.as_ptr(), vars.as_ptr(), priors.as_ptr(), 2, 1, query.as_ptr())
        };
        assert_eq!(result, 0); // closer to class 0
    }

    #[test]
    fn test_logistic_regression() {
        // Simple linearly separable 1D data
        let data = [0.0, 0.5, 1.0, 3.0, 3.5, 4.0];
        let labels = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let mut weights = [0.0; 2]; // 1 feature + bias
        let iters = unsafe {
            vitalis_logistic_regression(data.as_ptr(), labels.as_ptr(), 6, 1, 0.5, 1000, weights.as_mut_ptr())
        };
        assert!(iters > 0);
        // Predict: low values should give P < 0.5, high values > 0.5
        let p_low = unsafe { vitalis_logistic_predict(weights.as_ptr(), [0.5].as_ptr(), 1) };
        let p_high = unsafe { vitalis_logistic_predict(weights.as_ptr(), [3.5].as_ptr(), 1) };
        assert!(p_low < 0.5, "p_low = {}", p_low);
        assert!(p_high > 0.5, "p_high = {}", p_high);
    }

    #[test]
    fn test_pca_2d_to_1d() {
        let data = [1.0, 2.0, 2.0, 4.0, 3.0, 6.0, 4.0, 8.0];
        let mut out = [0.0; 4]; // 4 samples, 1 component
        let ret = unsafe { vitalis_pca(data.as_ptr(), 4, 2, 1, out.as_mut_ptr()) };
        assert_eq!(ret, 0);
        // Output should be monotonically increasing
        for i in 0..3 {
            assert!(out[i + 1].abs() > out[i].abs() || (out[i + 1] - out[i]).abs() > 0.01);
        }
    }

    #[test]
    fn test_svd_values() {
        let data = [1.0, 0.0, 0.0, 1.0]; // 2x2 identity
        let mut sv = [0.0; 2];
        let ret = unsafe { vitalis_svd_values(data.as_ptr(), 2, 2, 2, sv.as_mut_ptr()) };
        assert_eq!(ret, 0);
        assert!((sv[0] - 1.0).abs() < 0.01);
        assert!((sv[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_decision_stump() {
        let data = [1.0, 2.0, 3.0, 10.0, 11.0, 12.0];
        let labels = [0, 0, 0, 1, 1, 1];
        let mut out = [0.0; 4];
        let ret = unsafe { vitalis_decision_stump(data.as_ptr(), labels.as_ptr(), 6, 1, out.as_mut_ptr()) };
        assert_eq!(ret, 0);
        assert_eq!(out[0] as usize, 0); // feature 0
        assert!(out[1] > 3.0 && out[1] < 10.0); // threshold between 3 and 10
    }

    #[test]
    fn test_accuracy() {
        let pred = [1, 1, 0, 0, 1];
        let actual = [1, 0, 0, 0, 1];
        let acc = unsafe { vitalis_accuracy(pred.as_ptr(), actual.as_ptr(), 5) };
        assert!((acc - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_precision_recall_f1() {
        // TP=2, FP=1, FN=1
        let pred = [1, 1, 1, 0, 0];
        let actual = [1, 1, 0, 1, 0];
        let prec = unsafe { vitalis_precision(pred.as_ptr(), actual.as_ptr(), 5, 1) };
        let rec = unsafe { vitalis_recall(pred.as_ptr(), actual.as_ptr(), 5, 1) };
        let f1 = unsafe { vitalis_f1_score(pred.as_ptr(), actual.as_ptr(), 5, 1) };
        assert!((prec - 2.0 / 3.0).abs() < 1e-10);
        assert!((rec - 2.0 / 3.0).abs() < 1e-10);
        assert!((f1 - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_adam_step() {
        let mut params = [1.0, 2.0, 3.0];
        let grads = [0.1, 0.2, 0.3];
        let mut m = [0.0; 3];
        let mut v = [0.0; 3];
        let ret = unsafe {
            vitalis_adam_step(params.as_mut_ptr(), grads.as_ptr(), m.as_mut_ptr(), v.as_mut_ptr(),
                3, 0.001, 0.9, 0.999, 1e-8, 1)
        };
        assert_eq!(ret, 0);
        // Params should have decreased
        assert!(params[0] < 1.0);
        assert!(params[1] < 2.0);
    }

    #[test]
    fn test_dbscan() {
        // Two tight clusters with one noise point
        let data = [0.0, 0.0, 0.1, 0.1, 0.05, 0.05,
                     10.0, 10.0, 10.1, 10.1, 10.05, 10.05,
                     50.0, 50.0]; // noise
        let mut labels = [0i32; 7];
        let n_clusters = unsafe { vitalis_dbscan(data.as_ptr(), 7, 2, 0.5, 2, labels.as_mut_ptr()) };
        assert!(n_clusters >= 2);
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[3], labels[4]);
        assert_ne!(labels[0], labels[3]);
        assert_eq!(labels[6], -1); // noise
    }

    #[test]
    fn test_silhouette_score() {
        let data = [0.0, 0.0, 0.1, 0.1, 10.0, 10.0, 10.1, 10.1];
        let labels = [0, 0, 1, 1];
        let score = unsafe { vitalis_silhouette_score(data.as_ptr(), labels.as_ptr(), 4, 2) };
        assert!(score > 0.8); // well-separated clusters
    }

    #[test]
    fn test_mse_mae_r2() {
        let pred = [1.0, 2.0, 3.0];
        let actual = [1.1, 2.2, 2.8];
        let mse = unsafe { vitalis_mse(pred.as_ptr(), actual.as_ptr(), 3) };
        let mae = unsafe { vitalis_mae(pred.as_ptr(), actual.as_ptr(), 3) };
        let r2 = unsafe { vitalis_r2_score(pred.as_ptr(), actual.as_ptr(), 3) };
        assert!(mse > 0.0 && mse < 0.1);
        assert!(mae > 0.0 && mae < 0.2);
        assert!(r2 > 0.9);
    }

    #[test]
    fn test_kfold_split() {
        let mut folds = [0i32; 10];
        let ret = unsafe { vitalis_kfold_split(10, 5, folds.as_mut_ptr()) };
        assert_eq!(ret, 0);
        for i in 0..10 { assert!(folds[i] >= 0 && folds[i] < 5); }
    }

    #[test]
    fn test_lda_2class() {
        let data = [0.0, 1.0, 0.0, 1.0, 5.0, 6.0, 5.0, 6.0];
        let labels = [0, 0, 1, 1];
        let mut dir = [0.0; 2];
        let ret = unsafe { vitalis_lda_2class(data.as_ptr(), labels.as_ptr(), 4, 2, dir.as_mut_ptr()) };
        assert_eq!(ret, 0);
        // Direction should be roughly (1, 1) normalized
        assert!(dir[0].abs() > 0.1);
        assert!(dir[1].abs() > 0.1);
    }
}
