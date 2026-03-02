//! Bioinformatics Module — Vitalis v13.0
//!
//! Comprehensive computational biology algorithms:
//! - DNA/RNA operations (complement, transcription, GC content, codons)
//! - Sequence alignment (Needleman-Wunsch, Smith-Waterman)
//! - Population genetics (Hardy-Weinberg, Lotka-Volterra)
//! - Epidemiology (SIR, SEIR compartmental models)
//! - Enzyme kinetics (Michaelis-Menten)
//! - Phylogenetics (Jukes-Cantor distance, UPGMA)
//! - Motif finding (k-mer frequency, Hamming distance search)
//! - Protein analysis (hydrophobicity, molecular weight)

use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════
// 1. DNA/RNA Operations
// ═══════════════════════════════════════════════════════════════════════

/// Compute GC content of a DNA/RNA sequence.
/// `seq` is a null-terminated string of ACGT/U characters.
/// Returns fraction of G+C bases (0.0 to 1.0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_gc_content(seq: *const u8, len: usize) -> f64 {
    if seq.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };
    let gc = s.iter().filter(|&&b| b == b'G' || b == b'g' || b == b'C' || b == b'c').count();
    gc as f64 / len as f64
}

/// Compute DNA complement in-place. A↔T, C↔G.
/// `seq` is modified in place, `out` receives complement.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_dna_complement(seq: *const u8, out: *mut u8, len: usize) -> i32 {
    if seq.is_null() || out.is_null() || len == 0 { return -1; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };
    for i in 0..len {
        o[i] = match s[i] {
            b'A' | b'a' => b'T', b'T' | b't' => b'A',
            b'C' | b'c' => b'G', b'G' | b'g' => b'C',
            b'U' | b'u' => b'A', // RNA
            other => other,
        };
    }
    0
}

/// Reverse complement of DNA.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_reverse_complement(seq: *const u8, out: *mut u8, len: usize) -> i32 {
    if seq.is_null() || out.is_null() || len == 0 { return -1; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };
    for i in 0..len {
        let base = s[len - 1 - i];
        o[i] = match base {
            b'A' | b'a' => b'T', b'T' | b't' => b'A',
            b'C' | b'c' => b'G', b'G' | b'g' => b'C',
            b'U' | b'u' => b'A',
            other => other,
        };
    }
    0
}

/// DNA → RNA transcription (T→U).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_transcribe(seq: *const u8, out: *mut u8, len: usize) -> i32 {
    if seq.is_null() || out.is_null() || len == 0 { return -1; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, len) };
    for i in 0..len {
        o[i] = match s[i] { b'T' => b'U', b't' => b'u', c => c };
    }
    0
}

/// Count nucleotide frequencies. `counts_out` = [A, C, G, T/U].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_nucleotide_freq(seq: *const u8, len: usize, counts: *mut u64) -> i32 {
    if seq.is_null() || counts.is_null() || len == 0 { return -1; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };
    let c = unsafe { std::slice::from_raw_parts_mut(counts, 4) };
    c[0] = 0; c[1] = 0; c[2] = 0; c[3] = 0;
    for &b in s {
        match b {
            b'A' | b'a' => c[0] += 1,
            b'C' | b'c' => c[1] += 1,
            b'G' | b'g' => c[2] += 1,
            b'T' | b't' | b'U' | b'u' => c[3] += 1,
            _ => {}
        }
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Codon Translation
// ═══════════════════════════════════════════════════════════════════════

fn codon_to_amino(codon: &[u8]) -> u8 {
    if codon.len() < 3 { return b'?'; }
    let c0 = codon[0].to_ascii_uppercase();
    let c1 = codon[1].to_ascii_uppercase();
    let c2 = codon[2].to_ascii_uppercase();
    match (c0, c1, c2) {
        (b'A', b'U' | b'T', b'G') => b'M', // Start / Met
        (b'U', b'U', b'U' | b'C') => b'F', // Phe
        (b'U', b'U', b'A' | b'G') => b'L', // Leu
        (b'C', b'U', _)           => b'L', // Leu
        (b'A', b'U', b'U' | b'C' | b'A') => b'I', // Ile
        (b'G', b'U', _)           => b'V', // Val
        (b'U', b'C', _)           => b'S', // Ser
        (b'C', b'C', _)           => b'P', // Pro
        (b'A', b'C', _)           => b'T', // Thr
        (b'G', b'C', _)           => b'A', // Ala
        (b'U', b'A', b'U' | b'C') => b'Y', // Tyr
        (b'U', b'A', b'A' | b'G') => b'*', // Stop
        (b'C', b'A', b'U' | b'C') => b'H', // His
        (b'C', b'A', b'A' | b'G') => b'Q', // Gln
        (b'A', b'A', b'U' | b'C') => b'N', // Asn
        (b'A', b'A', b'A' | b'G') => b'K', // Lys
        (b'G', b'A', b'U' | b'C') => b'D', // Asp
        (b'G', b'A', b'A' | b'G') => b'E', // Glu
        (b'U', b'G', b'U' | b'C') => b'C', // Cys
        (b'U', b'G', b'A')        => b'*', // Stop
        (b'U', b'G', b'G')        => b'W', // Trp
        (b'C', b'G', _)           => b'R', // Arg
        (b'A', b'G', b'U' | b'C') => b'S', // Ser
        (b'A', b'G', b'A' | b'G') => b'R', // Arg
        (b'G', b'G', _)           => b'G', // Gly
        _ => b'?',
    }
}

/// Translate RNA/DNA sequence to amino acid sequence.
/// Returns number of amino acids written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_translate(seq: *const u8, seq_len: usize, out: *mut u8, out_cap: usize) -> i32 {
    if seq.is_null() || out.is_null() { return -1; }
    let s = unsafe { std::slice::from_raw_parts(seq, seq_len) };
    let o = unsafe { std::slice::from_raw_parts_mut(out, out_cap) };
    let n_codons = seq_len / 3;
    let count = n_codons.min(out_cap);
    for i in 0..count {
        o[i] = codon_to_amino(&s[i*3..i*3+3]);
    }
    count as i32
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Sequence Alignment
// ═══════════════════════════════════════════════════════════════════════

/// Needleman-Wunsch global alignment score.
/// `match_score`, `mismatch_penalty`, `gap_penalty`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_needleman_wunsch(
    seq1: *const u8, len1: usize,
    seq2: *const u8, len2: usize,
    match_score: i32, mismatch_penalty: i32, gap_penalty: i32,
) -> i32 {
    if seq1.is_null() || seq2.is_null() { return i32::MIN; }
    let s1 = unsafe { std::slice::from_raw_parts(seq1, len1) };
    let s2 = unsafe { std::slice::from_raw_parts(seq2, len2) };

    let rows = len1 + 1;
    let cols = len2 + 1;
    let mut dp = vec![0i32; rows * cols];

    for i in 0..rows { dp[i * cols] = i as i32 * gap_penalty; }
    for j in 0..cols { dp[j] = j as i32 * gap_penalty; }

    for i in 1..rows {
        for j in 1..cols {
            let score = if s1[i-1].to_ascii_uppercase() == s2[j-1].to_ascii_uppercase() {
                match_score
            } else {
                mismatch_penalty
            };
            let diag = dp[(i-1) * cols + (j-1)] + score;
            let up   = dp[(i-1) * cols + j] + gap_penalty;
            let left = dp[i * cols + (j-1)] + gap_penalty;
            dp[i * cols + j] = diag.max(up).max(left);
        }
    }
    dp[len1 * cols + len2]
}

/// Smith-Waterman local alignment score.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_smith_waterman(
    seq1: *const u8, len1: usize,
    seq2: *const u8, len2: usize,
    match_score: i32, mismatch_penalty: i32, gap_penalty: i32,
) -> i32 {
    if seq1.is_null() || seq2.is_null() { return 0; }
    let s1 = unsafe { std::slice::from_raw_parts(seq1, len1) };
    let s2 = unsafe { std::slice::from_raw_parts(seq2, len2) };

    let rows = len1 + 1;
    let cols = len2 + 1;
    let mut dp = vec![0i32; rows * cols];
    let mut max_score = 0i32;

    for i in 1..rows {
        for j in 1..cols {
            let score = if s1[i-1].to_ascii_uppercase() == s2[j-1].to_ascii_uppercase() {
                match_score
            } else {
                mismatch_penalty
            };
            let diag = dp[(i-1) * cols + (j-1)] + score;
            let up   = dp[(i-1) * cols + j] + gap_penalty;
            let left = dp[i * cols + (j-1)] + gap_penalty;
            dp[i * cols + j] = 0_i32.max(diag).max(up).max(left);
            max_score = max_score.max(dp[i * cols + j]);
        }
    }
    max_score
}

/// Hamming distance between two equal-length sequences.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_hamming_distance(
    seq1: *const u8, seq2: *const u8, len: usize,
) -> i32 {
    if seq1.is_null() || seq2.is_null() { return -1; }
    let s1 = unsafe { std::slice::from_raw_parts(seq1, len) };
    let s2 = unsafe { std::slice::from_raw_parts(seq2, len) };
    s1.iter().zip(s2.iter()).filter(|(a, b)| a.to_ascii_uppercase() != b.to_ascii_uppercase()).count() as i32
}

/// Edit distance (Levenshtein) between two sequences.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_edit_distance(
    seq1: *const u8, len1: usize, seq2: *const u8, len2: usize,
) -> i32 {
    if seq1.is_null() || seq2.is_null() { return -1; }
    let s1 = unsafe { std::slice::from_raw_parts(seq1, len1) };
    let s2 = unsafe { std::slice::from_raw_parts(seq2, len2) };

    let mut dp = vec![0u32; (len1 + 1) * (len2 + 1)];
    let cols = len2 + 1;
    for i in 0..=len1 { dp[i * cols] = i as u32; }
    for j in 0..=len2 { dp[j] = j as u32; }
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1[i-1].to_ascii_uppercase() == s2[j-1].to_ascii_uppercase() { 0 } else { 1 };
            dp[i * cols + j] = (dp[(i-1)*cols + j] + 1)
                .min(dp[i*cols + (j-1)] + 1)
                .min(dp[(i-1)*cols + (j-1)] + cost);
        }
    }
    dp[len1 * cols + len2] as i32
}

// ═══════════════════════════════════════════════════════════════════════
// 4. K-mer Analysis
// ═══════════════════════════════════════════════════════════════════════

/// Count k-mers in a sequence. Returns number of distinct k-mers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_kmer_count(seq: *const u8, len: usize, k: usize) -> u64 {
    if seq.is_null() || k == 0 || k > len { return 0; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };
    let mut set = std::collections::HashSet::new();
    for i in 0..=(len - k) {
        set.insert(&s[i..i+k]);
    }
    set.len() as u64
}

/// Linguistic complexity: ratio of observed k-mers to possible k-mers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_linguistic_complexity(seq: *const u8, len: usize, k: usize) -> f64 {
    if seq.is_null() || k == 0 || k > len { return 0.0; }
    let observed = unsafe { vitalis_bio_kmer_count(seq, len, k) };
    let max_possible = 4u64.pow(k as u32).min((len - k + 1) as u64);
    if max_possible == 0 { return 0.0; }
    observed as f64 / max_possible as f64
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Population Genetics
// ═══════════════════════════════════════════════════════════════════════

/// Hardy-Weinberg equilibrium: given allele frequency p, returns genotype
/// frequencies (p², 2pq, q²) where q = 1-p. `freqs_out` = [3].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_hardy_weinberg(p: f64, freqs_out: *mut f64) -> i32 {
    if freqs_out.is_null() || p < 0.0 || p > 1.0 { return -1; }
    let out = unsafe { std::slice::from_raw_parts_mut(freqs_out, 3) };
    let q = 1.0 - p;
    out[0] = p * p;       // AA
    out[1] = 2.0 * p * q; // Aa
    out[2] = q * q;       // aa
    0
}

/// Lotka-Volterra predator-prey simulation.
/// `alpha` = prey growth, `beta` = predation, `delta` = predator growth,
/// `gamma` = predator death. Simulates `steps` with `dt`.
/// `prey_out` and `pred_out` are [steps+1].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_lotka_volterra(
    prey0: f64, pred0: f64,
    alpha: f64, beta: f64, delta: f64, gamma: f64,
    dt: f64, steps: usize,
    prey_out: *mut f64, pred_out: *mut f64,
) -> i32 {
    if prey_out.is_null() || pred_out.is_null() { return -1; }
    let po = unsafe { std::slice::from_raw_parts_mut(prey_out, steps + 1) };
    let pd = unsafe { std::slice::from_raw_parts_mut(pred_out, steps + 1) };

    let mut x = prey0;
    let mut y = pred0;
    po[0] = x;
    pd[0] = y;

    for i in 0..steps {
        let dx = (alpha * x - beta * x * y) * dt;
        let dy = (delta * x * y - gamma * y) * dt;
        x += dx;
        y += dy;
        x = x.max(0.0);
        y = y.max(0.0);
        po[i + 1] = x;
        pd[i + 1] = y;
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════
// 6. Epidemiology (Compartmental Models)
// ═══════════════════════════════════════════════════════════════════════

/// SIR model simulation: Susceptible → Infected → Recovered.
/// `beta` = infection rate, `gamma_rate` = recovery rate.
/// `sir_out` = [steps+1][3] (S, I, R per step).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_sir_model(
    s0: f64, i0: f64, r0: f64,
    beta: f64, gamma_rate: f64,
    dt: f64, steps: usize,
    sir_out: *mut f64,
) -> i32 {
    if sir_out.is_null() { return -1; }
    let out = unsafe { std::slice::from_raw_parts_mut(sir_out, (steps + 1) * 3) };
    let n = s0 + i0 + r0;
    let mut s = s0;
    let mut i = i0;
    let mut r = r0;

    out[0] = s; out[1] = i; out[2] = r;
    for step in 0..steps {
        let new_infected = beta * s * i / n * dt;
        let new_recovered = gamma_rate * i * dt;
        s -= new_infected;
        i += new_infected - new_recovered;
        r += new_recovered;
        s = s.max(0.0);
        i = i.max(0.0);
        r = r.max(0.0);
        let base = (step + 1) * 3;
        out[base] = s; out[base + 1] = i; out[base + 2] = r;
    }
    0
}

/// SEIR model: Susceptible → Exposed → Infected → Recovered.
/// `sigma` = incubation rate (1/latent period).
/// `seir_out` = [steps+1][4] (S, E, I, R per step).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_seir_model(
    s0: f64, e0: f64, i0: f64, r0: f64,
    beta: f64, sigma: f64, gamma_rate: f64,
    dt: f64, steps: usize,
    seir_out: *mut f64,
) -> i32 {
    if seir_out.is_null() { return -1; }
    let out = unsafe { std::slice::from_raw_parts_mut(seir_out, (steps + 1) * 4) };
    let n = s0 + e0 + i0 + r0;
    let (mut s, mut e, mut i, mut r) = (s0, e0, i0, r0);

    out[0] = s; out[1] = e; out[2] = i; out[3] = r;
    for step in 0..steps {
        let new_exposed = beta * s * i / n * dt;
        let new_infected = sigma * e * dt;
        let new_recovered = gamma_rate * i * dt;
        s -= new_exposed;
        e += new_exposed - new_infected;
        i += new_infected - new_recovered;
        r += new_recovered;
        s = s.max(0.0); e = e.max(0.0); i = i.max(0.0); r = r.max(0.0);
        let base = (step + 1) * 4;
        out[base] = s; out[base+1] = e; out[base+2] = i; out[base+3] = r;
    }
    0
}

/// Basic reproduction number R₀ = β / γ.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_r0(beta: f64, gamma_rate: f64) -> f64 {
    if gamma_rate <= 0.0 { return f64::INFINITY; }
    beta / gamma_rate
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Enzyme Kinetics
// ═══════════════════════════════════════════════════════════════════════

/// Michaelis-Menten enzyme kinetics: v = Vmax * [S] / (Km + [S]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_michaelis_menten(substrate: f64, vmax: f64, km: f64) -> f64 {
    if km + substrate <= 0.0 { return 0.0; }
    vmax * substrate / (km + substrate)
}

/// Competitive inhibition: v = Vmax * [S] / (Km*(1 + [I]/Ki) + [S]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_competitive_inhibition(
    substrate: f64, inhibitor: f64, vmax: f64, km: f64, ki: f64,
) -> f64 {
    if ki <= 0.0 { return 0.0; }
    let km_app = km * (1.0 + inhibitor / ki);
    vmax * substrate / (km_app + substrate)
}

/// Hill equation: v = Vmax * [S]^n / (K^n + [S]^n).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_hill_equation(
    substrate: f64, vmax: f64, k: f64, n: f64,
) -> f64 {
    let sn = substrate.powf(n);
    let kn = k.powf(n);
    if kn + sn <= 0.0 { return 0.0; }
    vmax * sn / (kn + sn)
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Phylogenetics
// ═══════════════════════════════════════════════════════════════════════

/// Jukes-Cantor distance: corrects observed proportion of differences.
/// `p` = fraction of differing sites. Returns evolutionary distance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_jukes_cantor(p: f64) -> f64 {
    if p < 0.0 || p >= 0.75 { return f64::INFINITY; }
    -0.75 * (1.0 - 4.0 * p / 3.0).ln()
}

/// Kimura 2-parameter distance.
/// `p_transitions` = fraction of transition differences.
/// `q_transversions` = fraction of transversion differences.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_kimura_distance(p_transitions: f64, q_transversions: f64) -> f64 {
    let a = 1.0 - 2.0 * p_transitions - q_transversions;
    let b = 1.0 - 2.0 * q_transversions;
    if a <= 0.0 || b <= 0.0 { return f64::INFINITY; }
    -0.5 * a.ln() - 0.25 * b.ln()
}

// ═══════════════════════════════════════════════════════════════════════
// 9. Protein Analysis
// ═══════════════════════════════════════════════════════════════════════

/// Approximate molecular weight of a protein (amino acid sequence).
/// Returns weight in Daltons.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_protein_mw(seq: *const u8, len: usize) -> f64 {
    if seq.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };

    let weight = |aa: u8| -> f64 {
        match aa.to_ascii_uppercase() {
            b'G' => 57.02,  b'A' => 71.04,  b'V' => 99.07,
            b'L' => 113.08, b'I' => 113.08, b'P' => 97.05,
            b'F' => 147.07, b'W' => 186.08, b'M' => 131.04,
            b'S' => 87.03,  b'T' => 101.05, b'C' => 103.01,
            b'Y' => 163.06, b'H' => 137.06, b'D' => 115.03,
            b'E' => 129.04, b'N' => 114.04, b'Q' => 128.06,
            b'K' => 128.09, b'R' => 156.10,
            _ => 110.0, // average
        }
    };

    let mut mw = 18.02; // water
    for &aa in s { mw += weight(aa); }
    mw
}

/// GRAVY (Grand Average of Hydropathy) score.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_gravy(seq: *const u8, len: usize) -> f64 {
    if seq.is_null() || len == 0 { return 0.0; }
    let s = unsafe { std::slice::from_raw_parts(seq, len) };

    let hydro = |aa: u8| -> f64 {
        match aa.to_ascii_uppercase() {
            b'I' => 4.5,  b'V' => 4.2,  b'L' => 3.8,
            b'F' => 2.8,  b'C' => 2.5,  b'M' => 1.9,
            b'A' => 1.8,  b'G' => -0.4, b'T' => -0.7,
            b'S' => -0.8, b'W' => -0.9, b'Y' => -1.3,
            b'P' => -1.6, b'H' => -3.2, b'D' => -3.5,
            b'E' => -3.5, b'N' => -3.5, b'Q' => -3.5,
            b'K' => -3.9, b'R' => -4.5,
            _ => 0.0,
        }
    };

    let total: f64 = s.iter().map(|&aa| hydro(aa)).sum();
    total / len as f64
}

// ═══════════════════════════════════════════════════════════════════════
// 10. Population Dynamics
// ═══════════════════════════════════════════════════════════════════════

/// Logistic growth: P(t) = K / (1 + ((K - P0)/P0) * e^{-rt}).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_logistic_growth(p0: f64, r: f64, k: f64, t: f64) -> f64 {
    if p0 <= 0.0 || k <= 0.0 { return 0.0; }
    k / (1.0 + ((k - p0) / p0) * (-r * t).exp())
}

/// Wright-Fisher drift simulation: evolve allele frequency over generations.
/// Returns final allele frequency after `generations` steps with `pop_size` diploids.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_wright_fisher(
    initial_freq: f64, pop_size: usize, generations: usize, seed: u64,
) -> f64 {
    let mut rng = seed;
    let mut freq = initial_freq;

    for _ in 0..generations {
        let n = 2 * pop_size;
        let mut count = 0u32;
        for _ in 0..n {
            if prng_next(&mut rng) < freq { count += 1; }
        }
        freq = count as f64 / n as f64;
        if freq <= 0.0 || freq >= 1.0 { break; }
    }
    freq
}

fn prng_next(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

/// Shannon diversity index for species abundances.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_shannon_diversity(abundances: *const f64, n: usize) -> f64 {
    if abundances.is_null() || n == 0 { return 0.0; }
    let a = unsafe { std::slice::from_raw_parts(abundances, n) };
    let total: f64 = a.iter().sum();
    if total <= 0.0 { return 0.0; }
    let mut h = 0.0;
    for &x in a {
        if x > 0.0 {
            let p = x / total;
            h -= p * p.ln();
        }
    }
    h
}

/// Simpson's diversity index: D = 1 - Σ(pᵢ²).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_bio_simpson_diversity(abundances: *const f64, n: usize) -> f64 {
    if abundances.is_null() || n == 0 { return 0.0; }
    let a = unsafe { std::slice::from_raw_parts(abundances, n) };
    let total: f64 = a.iter().sum();
    if total <= 0.0 { return 0.0; }
    let mut sum_sq = 0.0;
    for &x in a {
        let p = x / total;
        sum_sq += p * p;
    }
    1.0 - sum_sq
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_content() {
        let seq = b"ATGCGCTA";
        let gc = unsafe { vitalis_bio_gc_content(seq.as_ptr(), seq.len()) };
        assert!((gc - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_dna_complement() {
        let seq = b"ATCG";
        let mut out = [0u8; 4];
        unsafe { vitalis_bio_dna_complement(seq.as_ptr(), out.as_mut_ptr(), 4) };
        assert_eq!(&out, b"TAGC");
    }

    #[test]
    fn test_reverse_complement() {
        let seq = b"AACG";
        let mut out = [0u8; 4];
        unsafe { vitalis_bio_reverse_complement(seq.as_ptr(), out.as_mut_ptr(), 4) };
        assert_eq!(&out, b"CGTT");
    }

    #[test]
    fn test_transcribe() {
        let seq = b"ATCG";
        let mut out = [0u8; 4];
        unsafe { vitalis_bio_transcribe(seq.as_ptr(), out.as_mut_ptr(), 4) };
        assert_eq!(&out, b"AUCG");
    }

    #[test]
    fn test_translate() {
        let seq = b"AUGGCU"; // Met-Ala
        let mut out = [0u8; 2];
        let n = unsafe { vitalis_bio_translate(seq.as_ptr(), seq.len(), out.as_mut_ptr(), 2) };
        assert_eq!(n, 2);
        assert_eq!(out[0], b'M');
        assert_eq!(out[1], b'A');
    }

    #[test]
    fn test_needleman_wunsch() {
        let s1 = b"AGTACG";
        let s2 = b"AGTACG"; // identical sequences
        let score = unsafe {
            vitalis_bio_needleman_wunsch(s1.as_ptr(), s1.len(), s2.as_ptr(), s2.len(), 1, -1, -2)
        };
        assert_eq!(score, 6); // perfect match
    }

    #[test]
    fn test_smith_waterman() {
        let s1 = b"AGTACGCA";
        let s2 = b"TATGC";
        let score = unsafe {
            vitalis_bio_smith_waterman(s1.as_ptr(), s1.len(), s2.as_ptr(), s2.len(), 2, -1, -1)
        };
        assert!(score > 0);
    }

    #[test]
    fn test_hamming() {
        let s1 = b"ACGT";
        let s2 = b"ACTT";
        let d = unsafe { vitalis_bio_hamming_distance(s1.as_ptr(), s2.as_ptr(), 4) };
        assert_eq!(d, 1);
    }

    #[test]
    fn test_hardy_weinberg() {
        let mut freqs = [0.0; 3];
        unsafe { vitalis_bio_hardy_weinberg(0.6, freqs.as_mut_ptr()) };
        assert!((freqs[0] - 0.36).abs() < 0.001); // p²
        assert!((freqs[1] - 0.48).abs() < 0.001); // 2pq
        assert!((freqs[2] - 0.16).abs() < 0.001); // q²
    }

    #[test]
    fn test_lotka_volterra() {
        let steps = 100;
        let mut prey = vec![0.0; steps + 1];
        let mut pred = vec![0.0; steps + 1];
        let r = unsafe {
            vitalis_bio_lotka_volterra(
                100.0, 20.0, 1.0, 0.01, 0.01, 1.0, 0.01, steps,
                prey.as_mut_ptr(), pred.as_mut_ptr(),
            )
        };
        assert_eq!(r, 0);
        assert!(prey[0] > 0.0);
        assert!(pred[0] > 0.0);
    }

    #[test]
    fn test_sir_model() {
        let steps = 100;
        let mut sir = vec![0.0; (steps + 1) * 3];
        let r = unsafe {
            vitalis_bio_sir_model(990.0, 10.0, 0.0, 0.3, 0.1, 0.1, steps, sir.as_mut_ptr())
        };
        assert_eq!(r, 0);
        // Total should be conserved
        let total = sir[steps * 3] + sir[steps * 3 + 1] + sir[steps * 3 + 2];
        assert!((total - 1000.0).abs() < 1.0);
    }

    #[test]
    fn test_seir_model() {
        let steps = 100;
        let mut seir = vec![0.0; (steps + 1) * 4];
        let r = unsafe {
            vitalis_bio_seir_model(990.0, 0.0, 10.0, 0.0, 0.3, 0.2, 0.1, 0.1, steps, seir.as_mut_ptr())
        };
        assert_eq!(r, 0);
    }

    #[test]
    fn test_r0() {
        let r0 = unsafe { vitalis_bio_r0(0.3, 0.1) };
        assert!((r0 - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_michaelis_menten() {
        let v = unsafe { vitalis_bio_michaelis_menten(10.0, 100.0, 5.0) };
        // v = 100 * 10 / (5 + 10) = 66.67
        assert!((v - 66.667).abs() < 0.1);
    }

    #[test]
    fn test_hill_equation() {
        let v = unsafe { vitalis_bio_hill_equation(10.0, 100.0, 10.0, 2.0) };
        assert!((v - 50.0).abs() < 0.1); // S^2 / (K^2 + S^2) when S=K → 0.5
    }

    #[test]
    fn test_jukes_cantor() {
        let d = unsafe { vitalis_bio_jukes_cantor(0.1) };
        assert!(d > 0.0 && d < 1.0);
    }

    #[test]
    fn test_protein_mw() {
        let seq = b"MVKL"; // 4 amino acids
        let mw = unsafe { vitalis_bio_protein_mw(seq.as_ptr(), seq.len()) };
        assert!(mw > 400.0 && mw < 600.0);
    }

    #[test]
    fn test_gravy() {
        let seq = b"IVLF"; // hydrophobic
        let g = unsafe { vitalis_bio_gravy(seq.as_ptr(), seq.len()) };
        assert!(g > 0.0); // should be positive (hydrophobic)
    }

    #[test]
    fn test_logistic_growth() {
        let p = unsafe { vitalis_bio_logistic_growth(10.0, 0.5, 1000.0, 20.0) };
        assert!(p > 10.0 && p < 1000.0);
    }

    #[test]
    fn test_shannon_diversity() {
        let abundances = [25.0, 25.0, 25.0, 25.0]; // max diversity
        let h = unsafe { vitalis_bio_shannon_diversity(abundances.as_ptr(), 4) };
        assert!((h - 4.0_f64.ln()).abs() < 0.01);
    }

    #[test]
    fn test_simpson_diversity() {
        let abundances = [25.0, 25.0, 25.0, 25.0];
        let d = unsafe { vitalis_bio_simpson_diversity(abundances.as_ptr(), 4) };
        assert!((d - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_edit_distance() {
        let s1 = b"ACGT";
        let s2 = b"AGT";
        let d = unsafe { vitalis_bio_edit_distance(s1.as_ptr(), s1.len(), s2.as_ptr(), s2.len()) };
        assert_eq!(d, 1);
    }

    #[test]
    fn test_kmer_count() {
        let seq = b"ACGT";
        let k = unsafe { vitalis_bio_kmer_count(seq.as_ptr(), seq.len(), 2) };
        assert_eq!(k, 3); // AC, CG, GT
    }
}
