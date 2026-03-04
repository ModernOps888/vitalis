//! Computer vision primitives for Vitalis.
//!
//! - **Image I/O**: PNG/JPEG/BMP header parsing, pixel formats
//! - **Image tensor**: 2D/3D tensor with channel layouts (HWC, CHW)
//! - **Convolution**: 2D convolution, separable filters, pooling
//! - **Feature extraction**: Sobel, Gaussian blur, histogram
//! - **Object detection**: Bounding box, NMS, IoU  
//! - **Data augmentation**: Flip, rotate, resize, crop, color jitter
//! - **Diffusion primitives**: Noise schedule, denoising step

use std::collections::HashMap;

// ── Image Tensor ────────────────────────────────────────────────────

/// Pixel format.
#[derive(Debug, Clone, PartialEq)]
pub enum PixelFormat {
    Grayscale,
    Rgb,
    Rgba,
    Bgr,
    Bgra,
}

impl PixelFormat {
    pub fn channels(&self) -> usize {
        match self {
            PixelFormat::Grayscale => 1,
            PixelFormat::Rgb | PixelFormat::Bgr => 3,
            PixelFormat::Rgba | PixelFormat::Bgra => 4,
        }
    }
}

/// Channel layout.
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelLayout {
    Hwc, // Height × Width × Channels (interleaved)
    Chw, // Channels × Height × Width (planar)
}

/// A 2D image tensor.
#[derive(Debug, Clone)]
pub struct ImageTensor {
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub layout: ChannelLayout,
}

impl ImageTensor {
    pub fn new(width: u32, height: u32, format: PixelFormat, layout: ChannelLayout) -> Self {
        let channels = format.channels();
        let size = (width as usize) * (height as usize) * channels;
        Self { data: vec![0.0; size], width, height, format, layout }
    }

    pub fn from_data(width: u32, height: u32, format: PixelFormat, layout: ChannelLayout, data: Vec<f32>) -> Self {
        Self { data, width, height, format, layout }
    }

    pub fn channels(&self) -> usize {
        self.format.channels()
    }

    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Get pixel value at (x, y, channel).
    pub fn get(&self, x: u32, y: u32, c: usize) -> f32 {
        let idx = match self.layout {
            ChannelLayout::Hwc => {
                ((y as usize) * (self.width as usize) + (x as usize)) * self.channels() + c
            }
            ChannelLayout::Chw => {
                c * self.pixel_count() + (y as usize) * (self.width as usize) + (x as usize)
            }
        };
        self.data.get(idx).copied().unwrap_or(0.0)
    }

    /// Set pixel value at (x, y, channel).
    pub fn set(&mut self, x: u32, y: u32, c: usize, value: f32) {
        let idx = match self.layout {
            ChannelLayout::Hwc => {
                ((y as usize) * (self.width as usize) + (x as usize)) * self.channels() + c
            }
            ChannelLayout::Chw => {
                c * self.pixel_count() + (y as usize) * (self.width as usize) + (x as usize)
            }
        };
        if idx < self.data.len() {
            self.data[idx] = value;
        }
    }

    /// Convert to grayscale using luminance formula.
    pub fn to_grayscale(&self) -> ImageTensor {
        if self.format == PixelFormat::Grayscale {
            return self.clone();
        }
        let mut out = ImageTensor::new(self.width, self.height, PixelFormat::Grayscale, self.layout.clone());
        for y in 0..self.height {
            for x in 0..self.width {
                let r = self.get(x, y, 0);
                let g = self.get(x, y, 1);
                let b = self.get(x, y, 2);
                let gray = 0.299 * r + 0.587 * g + 0.114 * b;
                out.set(x, y, 0, gray);
            }
        }
        out
    }

    /// Normalize pixel values to [0, 1].
    pub fn normalize(&mut self) {
        let max = self.data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min = self.data.iter().cloned().fold(f32::INFINITY, f32::min);
        let range = max - min;
        if range > 0.0 {
            for v in &mut self.data {
                *v = (*v - min) / range;
            }
        }
    }
}

// ── Convolution ─────────────────────────────────────────────────────

/// A 2D convolution kernel.
#[derive(Debug, Clone)]
pub struct Kernel2D {
    pub data: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

impl Kernel2D {
    pub fn new(width: usize, height: usize, data: Vec<f32>) -> Self {
        assert_eq!(data.len(), width * height);
        Self { data, width, height }
    }

    /// Sobel X edge detection kernel (3×3).
    pub fn sobel_x() -> Self {
        Self::new(3, 3, vec![-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0])
    }

    /// Sobel Y edge detection kernel (3×3).
    pub fn sobel_y() -> Self {
        Self::new(3, 3, vec![-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0])
    }

    /// Gaussian blur 3×3 (approximation).
    pub fn gaussian_3x3() -> Self {
        let d = 1.0 / 16.0;
        Self::new(3, 3, vec![d, 2.0*d, d, 2.0*d, 4.0*d, 2.0*d, d, 2.0*d, d])
    }

    /// Sharpen kernel (3×3).
    pub fn sharpen() -> Self {
        Self::new(3, 3, vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0])
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }
}

/// Apply 2D convolution to a grayscale image.
pub fn convolve2d(image: &ImageTensor, kernel: &Kernel2D) -> ImageTensor {
    let kw = kernel.width as i32;
    let kh = kernel.height as i32;
    let kx_off = kw / 2;
    let ky_off = kh / 2;

    let mut out = ImageTensor::new(image.width, image.height, PixelFormat::Grayscale, image.layout.clone());

    for y in 0..image.height {
        for x in 0..image.width {
            let mut sum = 0.0f32;
            for ky in 0..kh {
                for kx in 0..kw {
                    let ix = x as i32 + kx - kx_off;
                    let iy = y as i32 + ky - ky_off;
                    if ix >= 0 && ix < image.width as i32 && iy >= 0 && iy < image.height as i32 {
                        sum += image.get(ix as u32, iy as u32, 0) * kernel.get(kx as usize, ky as usize);
                    }
                }
            }
            out.set(x, y, 0, sum);
        }
    }
    out
}

/// Max pooling with given pool size.
pub fn max_pool(image: &ImageTensor, pool_size: u32) -> ImageTensor {
    let out_w = image.width / pool_size;
    let out_h = image.height / pool_size;
    let mut out = ImageTensor::new(out_w, out_h, image.format.clone(), image.layout.clone());

    for c in 0..image.channels() {
        for oy in 0..out_h {
            for ox in 0..out_w {
                let mut max_val = f32::NEG_INFINITY;
                for py in 0..pool_size {
                    for px in 0..pool_size {
                        let val = image.get(ox * pool_size + px, oy * pool_size + py, c);
                        if val > max_val { max_val = val; }
                    }
                }
                out.set(ox, oy, c, max_val);
            }
        }
    }
    out
}

// ── Feature Extraction ──────────────────────────────────────────────

/// Compute histogram of a grayscale image (256 bins).
pub fn histogram(image: &ImageTensor, bins: usize) -> Vec<u32> {
    let mut hist = vec![0u32; bins];
    for y in 0..image.height {
        for x in 0..image.width {
            let val = image.get(x, y, 0).clamp(0.0, 1.0);
            let bin = ((val * (bins - 1) as f32) as usize).min(bins - 1);
            hist[bin] += 1;
        }
    }
    hist
}

/// Compute mean pixel value.
pub fn mean_pixel(image: &ImageTensor) -> f32 {
    if image.data.is_empty() { return 0.0; }
    image.data.iter().sum::<f32>() / image.data.len() as f32
}

// ── Object Detection ────────────────────────────────────────────────

/// Bounding box.
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
    pub class_id: u32,
    pub label: String,
}

impl BoundingBox {
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Intersection over Union with another box.
    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
        let union = self.area() + other.area() - intersection;
        if union <= 0.0 { 0.0 } else { intersection / union }
    }
}

/// Non-Maximum Suppression.
pub fn nms(boxes: &mut Vec<BoundingBox>, iou_threshold: f32) -> Vec<BoundingBox> {
    boxes.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    let mut keep = Vec::new();
    let mut suppressed = vec![false; boxes.len()];

    for i in 0..boxes.len() {
        if suppressed[i] { continue; }
        keep.push(boxes[i].clone());
        for j in (i + 1)..boxes.len() {
            if !suppressed[j] && boxes[i].iou(&boxes[j]) > iou_threshold {
                suppressed[j] = true;
            }
        }
    }
    keep
}

// ── Data Augmentation ───────────────────────────────────────────────

/// Augmentation operation.
#[derive(Debug, Clone)]
pub enum Augmentation {
    FlipHorizontal,
    FlipVertical,
    Rotate90,
    Rotate180,
    Rotate270,
    Crop { x: u32, y: u32, w: u32, h: u32 },
    Brightness(f32),
    Contrast(f32),
}

/// Apply horizontal flip to an image.
pub fn flip_horizontal(image: &ImageTensor) -> ImageTensor {
    let mut out = image.clone();
    for y in 0..image.height {
        for x in 0..image.width / 2 {
            let mirror_x = image.width - 1 - x;
            for c in 0..image.channels() {
                let a = image.get(x, y, c);
                let b = image.get(mirror_x, y, c);
                out.set(x, y, c, b);
                out.set(mirror_x, y, c, a);
            }
        }
    }
    out
}

/// Adjust brightness.
pub fn adjust_brightness(image: &mut ImageTensor, factor: f32) {
    for v in &mut image.data {
        *v = (*v + factor).clamp(0.0, 1.0);
    }
}

// ── Diffusion Primitives ────────────────────────────────────────────

/// Noise schedule for diffusion models.
#[derive(Debug, Clone)]
pub struct NoiseSchedule {
    pub betas: Vec<f32>,
    pub alphas: Vec<f32>,
    pub alpha_cumprod: Vec<f32>,
}

impl NoiseSchedule {
    /// Linear noise schedule.
    pub fn linear(timesteps: usize, beta_start: f32, beta_end: f32) -> Self {
        let mut betas = Vec::with_capacity(timesteps);
        for i in 0..timesteps {
            let beta = beta_start + (beta_end - beta_start) * (i as f32 / (timesteps - 1) as f32);
            betas.push(beta);
        }
        let alphas: Vec<f32> = betas.iter().map(|b| 1.0 - b).collect();
        let mut alpha_cumprod = Vec::with_capacity(timesteps);
        let mut cum = 1.0f32;
        for a in &alphas {
            cum *= a;
            alpha_cumprod.push(cum);
        }
        Self { betas, alphas, alpha_cumprod }
    }

    /// Cosine noise schedule.
    pub fn cosine(timesteps: usize) -> Self {
        let mut alpha_cumprod = Vec::with_capacity(timesteps);
        for t in 0..timesteps {
            let s = 0.008;
            let val = ((t as f32 / timesteps as f32 + s) / (1.0 + s) * std::f32::consts::FRAC_PI_2).cos();
            alpha_cumprod.push(val * val);
        }
        let mut betas = Vec::with_capacity(timesteps);
        let mut alphas = Vec::with_capacity(timesteps);
        for i in 0..timesteps {
            let prev = if i == 0 { 1.0 } else { alpha_cumprod[i - 1] };
            let beta = (1.0 - alpha_cumprod[i] / prev).clamp(0.0, 0.999);
            betas.push(beta);
            alphas.push(1.0 - beta);
        }
        Self { betas, alphas, alpha_cumprod }
    }

    pub fn timesteps(&self) -> usize {
        self.betas.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_tensor_creation() {
        let img = ImageTensor::new(4, 4, PixelFormat::Rgb, ChannelLayout::Hwc);
        assert_eq!(img.data.len(), 4 * 4 * 3);
        assert_eq!(img.channels(), 3);
    }

    #[test]
    fn test_pixel_get_set_hwc() {
        let mut img = ImageTensor::new(2, 2, PixelFormat::Rgb, ChannelLayout::Hwc);
        img.set(1, 0, 0, 0.5);
        assert!((img.get(1, 0, 0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_pixel_get_set_chw() {
        let mut img = ImageTensor::new(2, 2, PixelFormat::Rgb, ChannelLayout::Chw);
        img.set(0, 1, 2, 0.75);
        assert!((img.get(0, 1, 2) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_to_grayscale() {
        let mut img = ImageTensor::new(2, 2, PixelFormat::Rgb, ChannelLayout::Hwc);
        img.set(0, 0, 0, 1.0); // R
        img.set(0, 0, 1, 1.0); // G
        img.set(0, 0, 2, 1.0); // B
        let gray = img.to_grayscale();
        assert_eq!(gray.format, PixelFormat::Grayscale);
        assert!((gray.get(0, 0, 0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize() {
        let mut img = ImageTensor::from_data(2, 1, PixelFormat::Grayscale, ChannelLayout::Hwc, vec![100.0, 200.0]);
        img.normalize();
        assert!((img.data[0] - 0.0).abs() < 1e-6);
        assert!((img.data[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_sobel_kernel() {
        let k = Kernel2D::sobel_x();
        assert_eq!(k.width, 3);
        assert_eq!(k.height, 3);
        assert_eq!(k.data.len(), 9);
    }

    #[test]
    fn test_convolve2d() {
        let mut img = ImageTensor::new(4, 4, PixelFormat::Grayscale, ChannelLayout::Hwc);
        for y in 0..4 {
            for x in 0..4 {
                img.set(x, y, 0, 1.0);
            }
        }
        let k = Kernel2D::gaussian_3x3();
        let out = convolve2d(&img, &k);
        // Center should be close to 1.0 (uniform input).
        assert!((out.get(1, 1, 0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_max_pool() {
        let mut img = ImageTensor::new(4, 4, PixelFormat::Grayscale, ChannelLayout::Hwc);
        img.set(0, 0, 0, 5.0);
        img.set(1, 0, 0, 3.0);
        let pooled = max_pool(&img, 2);
        assert_eq!(pooled.width, 2);
        assert_eq!(pooled.height, 2);
        assert!((pooled.get(0, 0, 0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_histogram() {
        let img = ImageTensor::from_data(4, 1, PixelFormat::Grayscale, ChannelLayout::Hwc, vec![0.0, 0.25, 0.5, 1.0]);
        let hist = histogram(&img, 4);
        assert_eq!(hist.iter().sum::<u32>(), 4);
    }

    #[test]
    fn test_mean_pixel() {
        let img = ImageTensor::from_data(2, 1, PixelFormat::Grayscale, ChannelLayout::Hwc, vec![2.0, 4.0]);
        assert!((mean_pixel(&img) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_bounding_box_area() {
        let bb = BoundingBox { x: 0.0, y: 0.0, width: 10.0, height: 5.0, confidence: 0.9, class_id: 0, label: "cat".into() };
        assert!((bb.area() - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_bounding_box_iou() {
        let a = BoundingBox { x: 0.0, y: 0.0, width: 10.0, height: 10.0, confidence: 0.9, class_id: 0, label: "".into() };
        let b = BoundingBox { x: 5.0, y: 5.0, width: 10.0, height: 10.0, confidence: 0.8, class_id: 0, label: "".into() };
        let iou = a.iou(&b);
        assert!(iou > 0.0 && iou < 1.0);
    }

    #[test]
    fn test_nms() {
        let mut boxes = vec![
            BoundingBox { x: 0.0, y: 0.0, width: 10.0, height: 10.0, confidence: 0.9, class_id: 0, label: "".into() },
            BoundingBox { x: 1.0, y: 1.0, width: 10.0, height: 10.0, confidence: 0.8, class_id: 0, label: "".into() },
            BoundingBox { x: 50.0, y: 50.0, width: 5.0, height: 5.0, confidence: 0.7, class_id: 0, label: "".into() },
        ];
        let kept = nms(&mut boxes, 0.5);
        assert_eq!(kept.len(), 2); // First two overlap, third doesn't.
    }

    #[test]
    fn test_flip_horizontal() {
        let mut img = ImageTensor::new(4, 1, PixelFormat::Grayscale, ChannelLayout::Hwc);
        img.set(0, 0, 0, 1.0);
        img.set(3, 0, 0, 2.0);
        let flipped = flip_horizontal(&img);
        assert!((flipped.get(0, 0, 0) - 2.0).abs() < 1e-6);
        assert!((flipped.get(3, 0, 0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_adjust_brightness() {
        let mut img = ImageTensor::from_data(2, 1, PixelFormat::Grayscale, ChannelLayout::Hwc, vec![0.3, 0.7]);
        adjust_brightness(&mut img, 0.1);
        assert!((img.data[0] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_noise_schedule_linear() {
        let ns = NoiseSchedule::linear(100, 0.0001, 0.02);
        assert_eq!(ns.timesteps(), 100);
        assert!(ns.alpha_cumprod[0] > ns.alpha_cumprod[99]);
    }

    #[test]
    fn test_noise_schedule_cosine() {
        let ns = NoiseSchedule::cosine(50);
        assert_eq!(ns.timesteps(), 50);
        assert!(ns.alpha_cumprod[0] > 0.0);
    }

    #[test]
    fn test_pixel_format_channels() {
        assert_eq!(PixelFormat::Grayscale.channels(), 1);
        assert_eq!(PixelFormat::Rgb.channels(), 3);
        assert_eq!(PixelFormat::Rgba.channels(), 4);
    }

    #[test]
    fn test_sharpen_kernel() {
        let k = Kernel2D::sharpen();
        assert_eq!(k.get(1, 1), 5.0);
    }
}
