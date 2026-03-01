//! Signal Processing Module — DSP algorithms for Vitalis
//!
//! Provides native-speed implementations of fundamental signal processing
//! operations exposed via C FFI for Python interop.
//!
//! # Algorithms:
//! - FFT (Cooley-Tukey radix-2 DIT)
//! - Convolution (linear, 1D)
//! - FIR / IIR filters
//! - Windowing functions (Hann, Hamming, Blackman, Kaiser)
//! - Resampling / interpolation
//! - Autocorrelation
//! - Spectral analysis (power spectrum, spectral centroid)
//! - Zero-crossing rate
//! - RMS energy

use std::f64::consts::PI;

// ─── FFT (Cooley-Tukey radix-2 DIT) ──────────────────────────────────

fn fft_recursive(real: &mut [f64], imag: &mut [f64], invert: bool) {
    let n = real.len();
    if n <= 1 {
        return;
    }

    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
    }

    // Butterfly operations
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle = 2.0 * PI / len as f64 * if invert { -1.0 } else { 1.0 };
        let wn_r = angle.cos();
        let wn_i = angle.sin();

        let mut i = 0;
        while i < n {
            let mut w_r = 1.0;
            let mut w_i = 0.0;
            for k in 0..half {
                let u_r = real[i + k];
                let u_i = imag[i + k];
                let v_r = real[i + k + half] * w_r - imag[i + k + half] * w_i;
                let v_i = real[i + k + half] * w_i + imag[i + k + half] * w_r;
                real[i + k] = u_r + v_r;
                imag[i + k] = u_i + v_i;
                real[i + k + half] = u_r - v_r;
                imag[i + k + half] = u_i - v_i;
                let new_w_r = w_r * wn_r - w_i * wn_i;
                let new_w_i = w_r * wn_i + w_i * wn_r;
                w_r = new_w_r;
                w_i = new_w_i;
            }
            i += len;
        }
        len <<= 1;
    }

    if invert {
        let inv_n = 1.0 / n as f64;
        for i in 0..n {
            real[i] *= inv_n;
            imag[i] *= inv_n;
        }
    }
}

/// Compute FFT in-place. Arrays must be power-of-2 length.
/// Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_fft(
    real: *mut f64,
    imag: *mut f64,
    n: usize,
) -> i32 {
    if real.is_null() || imag.is_null() || n == 0 || (n & (n - 1)) != 0 {
        return -1;
    }
    let r = unsafe { std::slice::from_raw_parts_mut(real, n) };
    let i = unsafe { std::slice::from_raw_parts_mut(imag, n) };
    fft_recursive(r, i, false);
    0
}

/// Compute inverse FFT in-place.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_ifft(
    real: *mut f64,
    imag: *mut f64,
    n: usize,
) -> i32 {
    if real.is_null() || imag.is_null() || n == 0 || (n & (n - 1)) != 0 {
        return -1;
    }
    let r = unsafe { std::slice::from_raw_parts_mut(real, n) };
    let i = unsafe { std::slice::from_raw_parts_mut(imag, n) };
    fft_recursive(r, i, true);
    0
}

/// Compute power spectrum: |X[k]|^2 for each frequency bin.
/// Output must have space for n values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_power_spectrum(
    real: *const f64,
    imag: *const f64,
    output: *mut f64,
    n: usize,
) -> i32 {
    if real.is_null() || imag.is_null() || output.is_null() || n == 0 {
        return -1;
    }
    let r = unsafe { std::slice::from_raw_parts(real, n) };
    let i = unsafe { std::slice::from_raw_parts(imag, n) };
    let out = unsafe { std::slice::from_raw_parts_mut(output, n) };
    for k in 0..n {
        out[k] = r[k] * r[k] + i[k] * i[k];
    }
    0
}

// ─── Convolution ──────────────────────────────────────────────────────

/// 1D linear convolution: output[i] = sum_k(signal[i-k] * kernel[k])
/// Output length must be signal_len + kernel_len - 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_convolve(
    signal: *const f64,
    signal_len: usize,
    kernel: *const f64,
    kernel_len: usize,
    output: *mut f64,
) -> i32 {
    if signal.is_null() || kernel.is_null() || output.is_null()
        || signal_len == 0 || kernel_len == 0
    {
        return -1;
    }
    let sig = unsafe { std::slice::from_raw_parts(signal, signal_len) };
    let ker = unsafe { std::slice::from_raw_parts(kernel, kernel_len) };
    let out_len = signal_len + kernel_len - 1;
    let out = unsafe { std::slice::from_raw_parts_mut(output, out_len) };

    for i in 0..out_len {
        out[i] = 0.0;
        let k_start = if i >= signal_len { i - signal_len + 1 } else { 0 };
        let k_end = if i < kernel_len { i + 1 } else { kernel_len };
        for k in k_start..k_end {
            out[i] += sig[i - k] * ker[k];
        }
    }
    0
}

/// Cross-correlation of two signals.
/// Output length = 2 * max(a_len, b_len) - 1 (centered).
/// Returns the lag of maximum correlation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_cross_correlate(
    a: *const f64,
    a_len: usize,
    b: *const f64,
    b_len: usize,
    output: *mut f64,
    output_len: usize,
) -> i64 {
    if a.is_null() || b.is_null() || output.is_null() || a_len == 0 || b_len == 0 {
        return 0;
    }
    let sa = unsafe { std::slice::from_raw_parts(a, a_len) };
    let sb = unsafe { std::slice::from_raw_parts(b, b_len) };
    let out = unsafe { std::slice::from_raw_parts_mut(output, output_len) };
    let max_lag = a_len.max(b_len) as i64;
    let mut best_lag: i64 = 0;
    let mut best_val = f64::NEG_INFINITY;

    for (idx, lag) in (-max_lag + 1..max_lag).enumerate() {
        if idx >= output_len {
            break;
        }
        let mut sum = 0.0;
        for i in 0..a_len {
            let j = i as i64 + lag;
            if j >= 0 && (j as usize) < b_len {
                sum += sa[i] * sb[j as usize];
            }
        }
        out[idx] = sum;
        if sum > best_val {
            best_val = sum;
            best_lag = lag;
        }
    }
    best_lag
}

// ─── Windowing Functions ──────────────────────────────────────────────

/// Apply Hann window in-place.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_window_hann(data: *mut f64, n: usize) {
    if data.is_null() || n == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts_mut(data, n) };
    for i in 0..n {
        let w = 0.5 * (1.0 - (2.0 * PI * i as f64 / (n - 1) as f64).cos());
        d[i] *= w;
    }
}

/// Apply Hamming window in-place.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_window_hamming(data: *mut f64, n: usize) {
    if data.is_null() || n == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts_mut(data, n) };
    for i in 0..n {
        let w = 0.54 - 0.46 * (2.0 * PI * i as f64 / (n - 1) as f64).cos();
        d[i] *= w;
    }
}

/// Apply Blackman window in-place.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_window_blackman(data: *mut f64, n: usize) {
    if data.is_null() || n == 0 { return; }
    let d = unsafe { std::slice::from_raw_parts_mut(data, n) };
    for i in 0..n {
        let k = 2.0 * PI * i as f64 / (n - 1) as f64;
        let w = 0.42 - 0.5 * k.cos() + 0.08 * (2.0 * k).cos();
        d[i] *= w;
    }
}

// ─── FIR Filter ───────────────────────────────────────────────────────

/// Apply an FIR filter to input signal.
/// output must have space for input_len values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_fir_filter(
    input: *const f64,
    input_len: usize,
    coeffs: *const f64,
    num_taps: usize,
    output: *mut f64,
) -> i32 {
    if input.is_null() || coeffs.is_null() || output.is_null()
        || input_len == 0 || num_taps == 0
    {
        return -1;
    }
    let inp = unsafe { std::slice::from_raw_parts(input, input_len) };
    let taps = unsafe { std::slice::from_raw_parts(coeffs, num_taps) };
    let out = unsafe { std::slice::from_raw_parts_mut(output, input_len) };

    for i in 0..input_len {
        let mut sum = 0.0;
        for j in 0..num_taps {
            if i >= j {
                sum += taps[j] * inp[i - j];
            }
        }
        out[i] = sum;
    }
    0
}

/// Apply a second-order IIR (biquad) filter.
/// Coefficients: b0, b1, b2, a1, a2 (a0 = 1.0 assumed).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_iir_biquad(
    input: *const f64,
    output: *mut f64,
    n: usize,
    b0: f64, b1: f64, b2: f64,
    a1: f64, a2: f64,
) -> i32 {
    if input.is_null() || output.is_null() || n == 0 {
        return -1;
    }
    let inp = unsafe { std::slice::from_raw_parts(input, n) };
    let out = unsafe { std::slice::from_raw_parts_mut(output, n) };
    let mut x1 = 0.0;
    let mut x2 = 0.0;
    let mut y1 = 0.0;
    let mut y2 = 0.0;

    for i in 0..n {
        let x0 = inp[i];
        let y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        out[i] = y0;
        x2 = x1;
        x1 = x0;
        y2 = y1;
        y1 = y0;
    }
    0
}

// ─── Signal Analysis ──────────────────────────────────────────────────

/// Compute zero-crossing rate of a signal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_zero_crossing_rate(
    data: *const f64,
    n: usize,
) -> f64 {
    if data.is_null() || n < 2 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let mut crossings = 0usize;
    for i in 1..n {
        if (d[i] >= 0.0) != (d[i - 1] >= 0.0) {
            crossings += 1;
        }
    }
    crossings as f64 / (n - 1) as f64
}

/// Compute RMS energy of a signal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_rms_energy(
    data: *const f64,
    n: usize,
) -> f64 {
    if data.is_null() || n == 0 { return 0.0; }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let sum_sq: f64 = d.iter().map(|x| x * x).sum();
    (sum_sq / n as f64).sqrt()
}

/// Compute spectral centroid from magnitude spectrum.
/// freq_bins[i] is the frequency of bin i, magnitudes[i] is the magnitude.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_spectral_centroid(
    freq_bins: *const f64,
    magnitudes: *const f64,
    n: usize,
) -> f64 {
    if freq_bins.is_null() || magnitudes.is_null() || n == 0 { return 0.0; }
    let f = unsafe { std::slice::from_raw_parts(freq_bins, n) };
    let m = unsafe { std::slice::from_raw_parts(magnitudes, n) };
    let sum_fm: f64 = f.iter().zip(m.iter()).map(|(fi, mi)| fi * mi).sum();
    let sum_m: f64 = m.iter().sum();
    if sum_m.abs() < 1e-15 { 0.0 } else { sum_fm / sum_m }
}

/// Compute autocorrelation of a signal.
/// Output must have space for max_lag values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_autocorrelation(
    data: *const f64,
    n: usize,
    output: *mut f64,
    max_lag: usize,
) -> i32 {
    if data.is_null() || output.is_null() || n == 0 || max_lag == 0 {
        return -1;
    }
    let d = unsafe { std::slice::from_raw_parts(data, n) };
    let out = unsafe { std::slice::from_raw_parts_mut(output, max_lag) };
    let mean: f64 = d.iter().sum::<f64>() / n as f64;

    for lag in 0..max_lag {
        let mut sum = 0.0;
        for i in 0..n - lag {
            sum += (d[i] - mean) * (d[i + lag] - mean);
        }
        out[lag] = sum / n as f64;
    }
    0
}

/// Linear interpolation / resampling.
/// Resamples `input` of length `in_len` to `output` of length `out_len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vitalis_resample_linear(
    input: *const f64,
    in_len: usize,
    output: *mut f64,
    out_len: usize,
) -> i32 {
    if input.is_null() || output.is_null() || in_len == 0 || out_len == 0 {
        return -1;
    }
    let inp = unsafe { std::slice::from_raw_parts(input, in_len) };
    let out = unsafe { std::slice::from_raw_parts_mut(output, out_len) };

    let ratio = (in_len - 1) as f64 / (out_len - 1).max(1) as f64;
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = pos - idx as f64;
        if idx + 1 < in_len {
            out[i] = inp[idx] * (1.0 - frac) + inp[idx + 1] * frac;
        } else {
            out[i] = inp[in_len - 1];
        }
    }
    0
}

// ────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_basic() {
        let mut real = vec![1.0, 0.0, 0.0, 0.0];
        let mut imag = vec![0.0; 4];
        let rc = unsafe { vitalis_fft(real.as_mut_ptr(), imag.as_mut_ptr(), 4) };
        assert_eq!(rc, 0);
        // DC component should be 1.0
        assert!((real[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_fft_inverse_roundtrip() {
        let orig = vec![1.0, 2.0, 3.0, 4.0];
        let mut real = orig.clone();
        let mut imag = vec![0.0; 4];
        unsafe { vitalis_fft(real.as_mut_ptr(), imag.as_mut_ptr(), 4); }
        unsafe { vitalis_ifft(real.as_mut_ptr(), imag.as_mut_ptr(), 4); }
        for (a, b) in real.iter().zip(orig.iter()) {
            assert!((a - b).abs() < 1e-10);
        }
    }

    #[test]
    fn test_fft_non_power_of_2() {
        let mut real = vec![1.0, 2.0, 3.0];
        let mut imag = vec![0.0; 3];
        let rc = unsafe { vitalis_fft(real.as_mut_ptr(), imag.as_mut_ptr(), 3) };
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_power_spectrum() {
        let real = vec![3.0, 4.0];
        let imag = vec![0.0, 0.0];
        let mut out = vec![0.0; 2];
        unsafe { vitalis_power_spectrum(real.as_ptr(), imag.as_ptr(), out.as_mut_ptr(), 2); }
        assert!((out[0] - 9.0).abs() < 1e-10);
        assert!((out[1] - 16.0).abs() < 1e-10);
    }

    #[test]
    fn test_convolve() {
        let signal = vec![1.0, 2.0, 3.0];
        let kernel = vec![1.0, 0.5];
        let mut out = vec![0.0; 4]; // 3+2-1
        unsafe { vitalis_convolve(signal.as_ptr(), 3, kernel.as_ptr(), 2, out.as_mut_ptr()); }
        assert!((out[0] - 1.0).abs() < 1e-10);
        assert!((out[1] - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_window_hann() {
        let mut data = vec![1.0; 4];
        unsafe { vitalis_window_hann(data.as_mut_ptr(), 4); }
        assert!((data[0]).abs() < 1e-10); // Hann starts at 0
        assert!(data[1] > 0.0);
        assert!((data[3]).abs() < 1e-10); // Hann ends at 0
    }

    #[test]
    fn test_fir_filter() {
        let input = vec![1.0, 0.0, 0.0, 0.0];
        let coeffs = vec![0.5, 0.3, 0.2];
        let mut output = vec![0.0; 4];
        unsafe { vitalis_fir_filter(input.as_ptr(), 4, coeffs.as_ptr(), 3, output.as_mut_ptr()); }
        assert!((output[0] - 0.5).abs() < 1e-10);
        assert!((output[1] - 0.3).abs() < 1e-10);
        assert!((output[2] - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_iir_biquad() {
        let input = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        let mut output = vec![0.0; 5];
        // Simple passthrough: b0=1, others=0
        unsafe { vitalis_iir_biquad(input.as_ptr(), output.as_mut_ptr(), 5, 1.0, 0.0, 0.0, 0.0, 0.0); }
        assert!((output[0] - 1.0).abs() < 1e-10);
        assert!((output[1]).abs() < 1e-10);
    }

    #[test]
    fn test_zero_crossing_rate() {
        let data = vec![1.0, -1.0, 1.0, -1.0];
        let zcr = unsafe { vitalis_zero_crossing_rate(data.as_ptr(), 4) };
        assert!((zcr - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rms_energy() {
        let data = vec![3.0, 4.0];
        let rms = unsafe { vitalis_rms_energy(data.as_ptr(), 2) };
        // sqrt((9+16)/2) = sqrt(12.5) ≈ 3.5355
        assert!((rms - (12.5_f64).sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_spectral_centroid() {
        let freqs = vec![100.0, 200.0, 300.0];
        let mags = vec![1.0, 2.0, 1.0];
        let sc = unsafe { vitalis_spectral_centroid(freqs.as_ptr(), mags.as_ptr(), 3) };
        // (100+400+300)/4 = 200
        assert!((sc - 200.0).abs() < 1e-10);
    }

    #[test]
    fn test_autocorrelation() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let mut out = vec![0.0; 3];
        unsafe { vitalis_autocorrelation(data.as_ptr(), 4, out.as_mut_ptr(), 3); }
        // lag=0 should be the variance
        assert!(out[0] > 0.0);
    }

    #[test]
    fn test_resample_linear() {
        let input = vec![0.0, 10.0];
        let mut output = vec![0.0; 5];
        unsafe { vitalis_resample_linear(input.as_ptr(), 2, output.as_mut_ptr(), 5); }
        assert!((output[0]).abs() < 1e-10);
        assert!((output[2] - 5.0).abs() < 1e-10);
        assert!((output[4] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_cross_correlate() {
        let a = vec![0.0, 1.0, 0.0, 0.0];
        let b = vec![0.0, 0.0, 1.0, 0.0];
        let mut out = vec![0.0; 7];
        let lag = unsafe { vitalis_cross_correlate(a.as_ptr(), 4, b.as_ptr(), 4, out.as_mut_ptr(), 7) };
        assert!(lag != 0); // should detect offset
    }
}
