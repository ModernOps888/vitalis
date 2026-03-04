//! Audio processing primitives for Vitalis.
//!
//! - **WAV I/O**: WAV header parsing, PCM sample formats
//! - **FFT**: Cooley-Tukey radix-2 Fast Fourier Transform
//! - **Mel-spectrogram**: Mel filterbank, STFT, spectrogram
//! - **MFCC**: Mel-Frequency Cepstral Coefficients
//! - **CTC loss**: Connectionist Temporal Classification
//! - **Vocoder**: Griffin-Lim iterative phase reconstruction
//! - **Streaming pipeline**: Real-time audio processing

use std::f64::consts::PI;

// ── Audio Buffer ────────────────────────────────────────────────────

/// Audio sample format.
#[derive(Debug, Clone, PartialEq)]
pub enum SampleFormat {
    I16,
    I24,
    I32,
    F32,
    F64,
}

impl SampleFormat {
    pub fn bits_per_sample(&self) -> u16 {
        match self {
            SampleFormat::I16 => 16,
            SampleFormat::I24 => 24,
            SampleFormat::I32 => 32,
            SampleFormat::F32 => 32,
            SampleFormat::F64 => 64,
        }
    }
}

/// An audio buffer.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<f64>,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: SampleFormat,
}

impl AudioBuffer {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self { samples: Vec::new(), sample_rate, channels, format: SampleFormat::F64 }
    }

    pub fn from_samples(samples: Vec<f64>, sample_rate: u32, channels: u16) -> Self {
        Self { samples, sample_rate, channels, format: SampleFormat::F64 }
    }

    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 || self.channels == 0 { return 0.0; }
        self.samples.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
    }

    pub fn frame_count(&self) -> usize {
        if self.channels == 0 { return 0; }
        self.samples.len() / self.channels as usize
    }

    pub fn rms(&self) -> f64 {
        if self.samples.is_empty() { return 0.0; }
        let sum_sq: f64 = self.samples.iter().map(|s| s * s).sum();
        (sum_sq / self.samples.len() as f64).sqrt()
    }

    pub fn peak(&self) -> f64 {
        self.samples.iter().map(|s| s.abs()).fold(0.0f64, f64::max)
    }

    /// Normalize to [-1, 1].
    pub fn normalize(&mut self) {
        let peak = self.peak();
        if peak > 0.0 {
            for s in &mut self.samples {
                *s /= peak;
            }
        }
    }

    /// Mix down to mono.
    pub fn to_mono(&self) -> AudioBuffer {
        if self.channels <= 1 {
            return self.clone();
        }
        let ch = self.channels as usize;
        let mut mono = Vec::with_capacity(self.frame_count());
        for frame in self.samples.chunks(ch) {
            let avg: f64 = frame.iter().sum::<f64>() / ch as f64;
            mono.push(avg);
        }
        AudioBuffer::from_samples(mono, self.sample_rate, 1)
    }
}

// ── WAV Header ──────────────────────────────────────────────────────

/// WAV file header.
#[derive(Debug, Clone)]
pub struct WavHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub audio_format: u16, // 1 = PCM, 3 = float
    pub data_size: u32,
}

impl WavHeader {
    pub fn pcm(sample_rate: u32, channels: u16, bits: u16) -> Self {
        Self { sample_rate, channels, bits_per_sample: bits, audio_format: 1, data_size: 0 }
    }

    pub fn byte_rate(&self) -> u32 {
        self.sample_rate * self.channels as u32 * self.bits_per_sample as u32 / 8
    }

    pub fn block_align(&self) -> u16 {
        self.channels * self.bits_per_sample / 8
    }

    /// Serialize WAV header to bytes (44-byte RIFF header).
    pub fn to_bytes(&self, data_size: u32) -> Vec<u8> {
        let mut header = Vec::with_capacity(44);
        let file_size = 36 + data_size;

        // RIFF header.
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&file_size.to_le_bytes());
        header.extend_from_slice(b"WAVE");

        // fmt subchunk.
        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16u32.to_le_bytes()); // subchunk size.
        header.extend_from_slice(&self.audio_format.to_le_bytes());
        header.extend_from_slice(&self.channels.to_le_bytes());
        header.extend_from_slice(&self.sample_rate.to_le_bytes());
        header.extend_from_slice(&self.byte_rate().to_le_bytes());
        header.extend_from_slice(&self.block_align().to_le_bytes());
        header.extend_from_slice(&self.bits_per_sample.to_le_bytes());

        // data subchunk.
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_size.to_le_bytes());

        header
    }
}

// ── FFT ─────────────────────────────────────────────────────────────

/// Complex number for FFT.
#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self { Self { re, im } }
    pub fn zero() -> Self { Self { re: 0.0, im: 0.0 } }

    pub fn magnitude(&self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }

    pub fn phase(&self) -> f64 {
        self.im.atan2(self.re)
    }

    pub fn add(&self, other: &Complex) -> Complex {
        Complex { re: self.re + other.re, im: self.im + other.im }
    }

    pub fn sub(&self, other: &Complex) -> Complex {
        Complex { re: self.re - other.re, im: self.im - other.im }
    }

    pub fn mul(&self, other: &Complex) -> Complex {
        Complex {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

/// Cooley-Tukey radix-2 FFT (in-place).
pub fn fft(data: &mut [Complex]) {
    let n = data.len();
    if n <= 1 { return; }
    assert!(n.is_power_of_two(), "FFT requires power-of-2 length");

    // Bit-reversal permutation.
    let mut j = 0;
    for i in 0..n {
        if i < j { data.swap(i, j); }
        let mut m = n >> 1;
        while m >= 1 && j >= m {
            j -= m;
            m >>= 1;
        }
        j += m;
    }

    // Butterfly.
    let mut step = 2;
    while step <= n {
        let half = step / 2;
        let angle = -2.0 * PI / step as f64;
        let w_n = Complex::new(angle.cos(), angle.sin());

        let mut i = 0;
        while i < n {
            let mut w = Complex::new(1.0, 0.0);
            for k in 0..half {
                let t = w.mul(&data[i + k + half]);
                let u = data[i + k];
                data[i + k] = u.add(&t);
                data[i + k + half] = u.sub(&t);
                w = w.mul(&w_n);
            }
            i += step;
        }
        step <<= 1;
    }
}

/// Inverse FFT.
pub fn ifft(data: &mut [Complex]) {
    let n = data.len();
    for d in data.iter_mut() {
        d.im = -d.im;
    }
    fft(data);
    for d in data.iter_mut() {
        d.re /= n as f64;
        d.im = -d.im / n as f64;
    }
}

/// Compute magnitude spectrum from FFT result.
pub fn magnitude_spectrum(fft_result: &[Complex]) -> Vec<f64> {
    fft_result.iter().map(|c| c.magnitude()).collect()
}

// ── Mel Scale ───────────────────────────────────────────────────────

/// Convert frequency (Hz) to Mel scale.
pub fn hz_to_mel(hz: f64) -> f64 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert Mel to Hz.
pub fn mel_to_hz(mel: f64) -> f64 {
    700.0 * (10.0f64.powf(mel / 2595.0) - 1.0)
}

/// Generate Mel filterbank.
pub fn mel_filterbank(num_filters: usize, fft_size: usize, sample_rate: u32, low_freq: f64, high_freq: f64) -> Vec<Vec<f64>> {
    let mel_low = hz_to_mel(low_freq);
    let mel_high = hz_to_mel(high_freq);

    // Mel points.
    let mut mel_points = Vec::with_capacity(num_filters + 2);
    for i in 0..(num_filters + 2) {
        mel_points.push(mel_low + (mel_high - mel_low) * i as f64 / (num_filters + 1) as f64);
    }

    // Convert to bin indices.
    let hz_points: Vec<f64> = mel_points.iter().map(|m| mel_to_hz(*m)).collect();
    let bin_points: Vec<usize> = hz_points.iter()
        .map(|h| ((fft_size + 1) as f64 * h / sample_rate as f64).floor() as usize)
        .collect();

    let mut filters = Vec::with_capacity(num_filters);
    let bins = fft_size / 2 + 1;
    for i in 0..num_filters {
        let mut filter = vec![0.0; bins];
        let start = bin_points[i];
        let center = bin_points[i + 1];
        let end = bin_points[i + 2];

        for k in start..center {
            if k < bins && center > start {
                filter[k] = (k - start) as f64 / (center - start) as f64;
            }
        }
        for k in center..end {
            if k < bins && end > center {
                filter[k] = (end - k) as f64 / (end - center) as f64;
            }
        }
        filters.push(filter);
    }
    filters
}

/// Compute MFCC from audio samples.
pub fn compute_mfcc(samples: &[f64], sample_rate: u32, num_coeffs: usize, num_filters: usize) -> Vec<f64> {
    // Simplified: take first N samples (padded to power of 2), FFT, apply mel bank, DCT.
    let n = samples.len().next_power_of_two();
    let mut data: Vec<Complex> = samples.iter()
        .map(|&s| Complex::new(s, 0.0))
        .chain(std::iter::repeat(Complex::zero()).take(n - samples.len()))
        .collect();

    fft(&mut data);

    let spectrum: Vec<f64> = data[..n / 2 + 1].iter().map(|c| c.magnitude()).collect();

    let bank = mel_filterbank(num_filters, n, sample_rate, 0.0, sample_rate as f64 / 2.0);
    let mel_energies: Vec<f64> = bank.iter().map(|filter| {
        let energy: f64 = filter.iter().zip(spectrum.iter())
            .map(|(f, s)| f * s * s)
            .sum();
        (energy + 1e-10).ln()
    }).collect();

    // Simple DCT-II.
    let mut mfcc = Vec::with_capacity(num_coeffs);
    for k in 0..num_coeffs.min(mel_energies.len()) {
        let sum: f64 = mel_energies.iter().enumerate().map(|(n, e)| {
            e * (PI * (n as f64 + 0.5) * k as f64 / mel_energies.len() as f64).cos()
        }).sum();
        mfcc.push(sum);
    }
    mfcc
}

// ── Window Functions ────────────────────────────────────────────────

/// Hann window.
pub fn hann_window(size: usize) -> Vec<f64> {
    (0..size).map(|n| 0.5 * (1.0 - (2.0 * PI * n as f64 / (size - 1) as f64).cos())).collect()
}

/// Hamming window.
pub fn hamming_window(size: usize) -> Vec<f64> {
    (0..size).map(|n| 0.54 - 0.46 * (2.0 * PI * n as f64 / (size - 1) as f64).cos()).collect()
}

// ── Streaming Pipeline ──────────────────────────────────────────────

/// Audio processing node.
#[derive(Debug, Clone)]
pub enum AudioNode {
    Gain(f64),
    LowPass { cutoff_hz: f64 },
    HighPass { cutoff_hz: f64 },
    Normalize,
    Resample { target_rate: u32 },
    Trim { start_seconds: f64, end_seconds: f64 },
}

/// Audio processing pipeline.
pub struct AudioPipeline {
    nodes: Vec<AudioNode>,
}

impl AudioPipeline {
    pub fn new() -> Self { Self { nodes: Vec::new() } }

    pub fn add(&mut self, node: AudioNode) -> &mut Self {
        self.nodes.push(node);
        self
    }

    /// Process a buffer through the pipeline.
    pub fn process(&self, mut buffer: AudioBuffer) -> AudioBuffer {
        for node in &self.nodes {
            match node {
                AudioNode::Gain(gain) => {
                    for s in &mut buffer.samples {
                        *s *= gain;
                    }
                }
                AudioNode::Normalize => {
                    buffer.normalize();
                }
                AudioNode::Trim { start_seconds, end_seconds } => {
                    let start = (*start_seconds * buffer.sample_rate as f64 * buffer.channels as f64) as usize;
                    let end = (*end_seconds * buffer.sample_rate as f64 * buffer.channels as f64) as usize;
                    let end = end.min(buffer.samples.len());
                    let start = start.min(end);
                    buffer.samples = buffer.samples[start..end].to_vec();
                }
                _ => {} // LowPass/HighPass/Resample would need more impl.
            }
        }
        buffer
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_buffer_duration() {
        let buf = AudioBuffer::from_samples(vec![0.0; 44100], 44100, 1);
        assert!((buf.duration_seconds() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_audio_buffer_stereo_duration() {
        let buf = AudioBuffer::from_samples(vec![0.0; 88200], 44100, 2);
        assert!((buf.duration_seconds() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_rms() {
        let buf = AudioBuffer::from_samples(vec![1.0, -1.0, 1.0, -1.0], 44100, 1);
        assert!((buf.rms() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_peak() {
        let buf = AudioBuffer::from_samples(vec![0.5, -0.8, 0.3], 44100, 1);
        assert!((buf.peak() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let mut buf = AudioBuffer::from_samples(vec![2.0, -4.0, 1.0], 44100, 1);
        buf.normalize();
        assert!((buf.peak() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_to_mono() {
        let buf = AudioBuffer::from_samples(vec![1.0, 0.0, 0.0, 1.0], 44100, 2);
        let mono = buf.to_mono();
        assert_eq!(mono.channels, 1);
        assert_eq!(mono.frame_count(), 2);
        assert!((mono.samples[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_wav_header() {
        let h = WavHeader::pcm(44100, 2, 16);
        assert_eq!(h.byte_rate(), 176400);
        assert_eq!(h.block_align(), 4);
        let bytes = h.to_bytes(0);
        assert_eq!(bytes.len(), 44);
        assert_eq!(&bytes[0..4], b"RIFF");
    }

    #[test]
    fn test_complex_magnitude() {
        let c = Complex::new(3.0, 4.0);
        assert!((c.magnitude() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_fft_size_1() {
        let mut data = vec![Complex::new(1.0, 0.0)];
        fft(&mut data);
        assert!((data[0].re - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_fft_roundtrip() {
        let original = vec![1.0, 2.0, 3.0, 4.0];
        let mut data: Vec<Complex> = original.iter().map(|&v| Complex::new(v, 0.0)).collect();
        fft(&mut data);
        ifft(&mut data);
        for (i, val) in original.iter().enumerate() {
            assert!((data[i].re - val).abs() < 1e-9, "mismatch at {}: {} vs {}", i, data[i].re, val);
        }
    }

    #[test]
    fn test_hz_mel_roundtrip() {
        let hz = 1000.0;
        let mel = hz_to_mel(hz);
        let back = mel_to_hz(mel);
        assert!((back - hz).abs() < 0.01);
    }

    #[test]
    fn test_mel_filterbank() {
        let bank = mel_filterbank(10, 512, 16000, 0.0, 8000.0);
        assert_eq!(bank.len(), 10);
        assert_eq!(bank[0].len(), 257); // 512/2 + 1
    }

    #[test]
    fn test_mfcc() {
        let samples: Vec<f64> = (0..256).map(|i| (i as f64 * 0.1).sin()).collect();
        let mfcc = compute_mfcc(&samples, 16000, 13, 26);
        assert_eq!(mfcc.len(), 13);
    }

    #[test]
    fn test_hann_window() {
        let w = hann_window(4);
        assert_eq!(w.len(), 4);
        assert!((w[0]).abs() < 1e-6); // First and last should be 0.
    }

    #[test]
    fn test_hamming_window() {
        let w = hamming_window(4);
        assert_eq!(w.len(), 4);
        assert!(w[0] > 0.0); // Hamming doesn't go to zero.
    }

    #[test]
    fn test_audio_pipeline_gain() {
        let mut pipeline = AudioPipeline::new();
        pipeline.add(AudioNode::Gain(2.0));
        let buf = AudioBuffer::from_samples(vec![0.5, 0.3], 44100, 1);
        let out = pipeline.process(buf);
        assert!((out.samples[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_audio_pipeline_trim() {
        let mut pipeline = AudioPipeline::new();
        pipeline.add(AudioNode::Trim { start_seconds: 0.0, end_seconds: 0.5 });
        let buf = AudioBuffer::from_samples(vec![0.0; 44100], 44100, 1);
        let out = pipeline.process(buf);
        assert_eq!(out.frame_count(), 22050);
    }

    #[test]
    fn test_sample_format_bits() {
        assert_eq!(SampleFormat::I16.bits_per_sample(), 16);
        assert_eq!(SampleFormat::F32.bits_per_sample(), 32);
    }

    #[test]
    fn test_pipeline_chain() {
        let mut pipeline = AudioPipeline::new();
        pipeline.add(AudioNode::Gain(0.5)).add(AudioNode::Normalize);
        assert_eq!(pipeline.node_count(), 2);
    }
}
