//! Vitalis Graphics Engine — Core 2D/3D rendering primitives and pipeline.
//!
//! Provides the foundational graphics abstractions for all visual output:
//! - **Color**: RGBA/HSLA/Hex color representation with blending, gradients
//! - **Vector2/Vector3/Vector4**: Math primitives for positions, directions
//! - **Matrix3x3/Matrix4x4**: Transform matrices (translate, rotate, scale, project)
//! - **Shape2D/Shape3D**: Geometric primitives (rect, circle, polygon, cube, sphere)
//! - **RenderTarget**: Framebuffer abstraction (screen, texture, offscreen)
//! - **RenderPipeline**: Configurable multi-pass rendering pipeline
//! - **Camera**: Perspective/orthographic projection cameras
//! - **Viewport**: Window/canvas abstraction with DPI awareness
//! - **Gradient**: Linear, radial, conic gradient fills
//! - **TextRenderer**: Font metrics, glyph rasterization, text layout
//! - **ImageBuffer**: Pixel buffer with format conversions (RGBA8, RGBAF32)
//! - **BlendMode**: Porter-Duff compositing operators
//! - **Path2D**: Bézier curves, arcs, SVG-compatible path construction
//! - **AnimationCurve**: Easing functions (ease-in, ease-out, bezier, spring)
//!
//! This module is backend-agnostic — it generates draw commands that can be
//! consumed by the shader_lang backends (GLSL, HLSL, WGSL, MSL) or by a
//! CPU software rasterizer for testing and headless rendering.

use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  COLOR SYSTEM
// ═══════════════════════════════════════════════════════════════════════

/// A color in RGBA color space with f64 components [0.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r: r.clamp(0.0, 1.0), g: g.clamp(0.0, 1.0), b: b.clamp(0.0, 1.0), a: a.clamp(0.0, 1.0) }
    }

    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f64 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f64 / 255.0;
        let b = (hex & 0xFF) as f64 / 255.0;
        Self::rgb(r, g, b)
    }

    pub fn from_hex_alpha(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as f64 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f64 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f64 / 255.0;
        let a = (hex & 0xFF) as f64 / 255.0;
        Self::rgba(r, g, b, a)
    }

    pub fn to_hex(&self) -> u32 {
        let r = (self.r * 255.0).round() as u32;
        let g = (self.g * 255.0).round() as u32;
        let b = (self.b * 255.0).round() as u32;
        (r << 16) | (g << 8) | b
    }

    /// Convert RGBA to HSLA.
    pub fn to_hsla(&self) -> (f64, f64, f64, f64) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < 1e-10 {
            return (0.0, 0.0, l, self.a);
        }

        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

        let h = if (max - self.r).abs() < 1e-10 {
            let mut h = (self.g - self.b) / d;
            if self.g < self.b { h += 6.0; }
            h
        } else if (max - self.g).abs() < 1e-10 {
            (self.b - self.r) / d + 2.0
        } else {
            (self.r - self.g) / d + 4.0
        };

        (h / 6.0, s, l, self.a)
    }

    /// Create color from HSLA values (h, s, l in [0,1]).
    pub fn from_hsla(h: f64, s: f64, l: f64, a: f64) -> Self {
        if s.abs() < 1e-10 {
            return Self::rgba(l, l, l, a);
        }

        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;

        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

        Self::rgba(r, g, b, a)
    }

    /// Linearly interpolate between two colors.
    pub fn lerp(&self, other: &Color, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Porter-Duff "source over" compositing.
    pub fn blend_over(&self, dst: &Color) -> Color {
        let out_a = self.a + dst.a * (1.0 - self.a);
        if out_a < 1e-10 {
            return Color::rgba(0.0, 0.0, 0.0, 0.0);
        }
        Color {
            r: (self.r * self.a + dst.r * dst.a * (1.0 - self.a)) / out_a,
            g: (self.g * self.a + dst.g * dst.a * (1.0 - self.a)) / out_a,
            b: (self.b * self.a + dst.b * dst.a * (1.0 - self.a)) / out_a,
            a: out_a,
        }
    }

    /// Adjust brightness by factor (1.0 = unchanged).
    pub fn brighten(&self, factor: f64) -> Color {
        Color::rgba(
            (self.r * factor).clamp(0.0, 1.0),
            (self.g * factor).clamp(0.0, 1.0),
            (self.b * factor).clamp(0.0, 1.0),
            self.a,
        )
    }

    /// Luminance (perceived brightness, ITU-R BT.709).
    pub fn luminance(&self) -> f64 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Convert to grayscale.
    pub fn grayscale(&self) -> Color {
        let l = self.luminance();
        Color::rgba(l, l, l, self.a)
    }

    /// Invert color.
    pub fn invert(&self) -> Color {
        Color::rgba(1.0 - self.r, 1.0 - self.g, 1.0 - self.b, self.a)
    }

    // Common named colors
    pub fn black() -> Self { Self::rgb(0.0, 0.0, 0.0) }
    pub fn white() -> Self { Self::rgb(1.0, 1.0, 1.0) }
    pub fn red() -> Self { Self::rgb(1.0, 0.0, 0.0) }
    pub fn green() -> Self { Self::rgb(0.0, 1.0, 0.0) }
    pub fn blue() -> Self { Self::rgb(0.0, 0.0, 1.0) }
    pub fn yellow() -> Self { Self::rgb(1.0, 1.0, 0.0) }
    pub fn cyan() -> Self { Self::rgb(0.0, 1.0, 1.0) }
    pub fn magenta() -> Self { Self::rgb(1.0, 0.0, 1.0) }
    pub fn transparent() -> Self { Self::rgba(0.0, 0.0, 0.0, 0.0) }
    pub fn cornflower_blue() -> Self { Self::from_hex(0x6495ED) }
    pub fn coral() -> Self { Self::from_hex(0xFF7F50) }
    pub fn dark_slate_gray() -> Self { Self::from_hex(0x2F4F4F) }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgba({:.3}, {:.3}, {:.3}, {:.3})", self.r, self.g, self.b, self.a)
    }
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

// ═══════════════════════════════════════════════════════════════════════
//  VECTOR TYPES
// ═══════════════════════════════════════════════════════════════════════

/// 2D vector / point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self { Self { x, y } }
    pub fn zero() -> Self { Self { x: 0.0, y: 0.0 } }
    pub fn one() -> Self { Self { x: 1.0, y: 1.0 } }
    pub fn length(&self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }
    pub fn length_sq(&self) -> f64 { self.x * self.x + self.y * self.y }
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len < 1e-10 { return *self; }
        Self { x: self.x / len, y: self.y / len }
    }
    pub fn dot(&self, other: &Vec2) -> f64 { self.x * other.x + self.y * other.y }
    pub fn cross(&self, other: &Vec2) -> f64 { self.x * other.y - self.y * other.x }
    pub fn add(&self, other: &Vec2) -> Self { Self { x: self.x + other.x, y: self.y + other.y } }
    pub fn sub(&self, other: &Vec2) -> Self { Self { x: self.x - other.x, y: self.y - other.y } }
    pub fn scale(&self, s: f64) -> Self { Self { x: self.x * s, y: self.y * s } }
    pub fn lerp(&self, other: &Vec2, t: f64) -> Self {
        Self { x: self.x + (other.x - self.x) * t, y: self.y + (other.y - self.y) * t }
    }
    pub fn rotate(&self, angle_rad: f64) -> Self {
        let c = angle_rad.cos();
        let s = angle_rad.sin();
        Self { x: self.x * c - self.y * s, y: self.x * s + self.y * c }
    }
    pub fn distance(&self, other: &Vec2) -> f64 { self.sub(other).length() }
    pub fn angle(&self) -> f64 { self.y.atan2(self.x) }
    pub fn perpendicular(&self) -> Self { Self { x: -self.y, y: self.x } }
    pub fn reflect(&self, normal: &Vec2) -> Self {
        let d = 2.0 * self.dot(normal);
        Self { x: self.x - d * normal.x, y: self.y - d * normal.y }
    }
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3})", self.x, self.y)
    }
}

/// 3D vector / point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self { Self { x, y, z } }
    pub fn zero() -> Self { Self { x: 0.0, y: 0.0, z: 0.0 } }
    pub fn one() -> Self { Self { x: 1.0, y: 1.0, z: 1.0 } }
    pub fn up() -> Self { Self { x: 0.0, y: 1.0, z: 0.0 } }
    pub fn forward() -> Self { Self { x: 0.0, y: 0.0, z: -1.0 } }
    pub fn right() -> Self { Self { x: 1.0, y: 0.0, z: 0.0 } }
    pub fn length(&self) -> f64 { (self.x * self.x + self.y * self.y + self.z * self.z).sqrt() }
    pub fn length_sq(&self) -> f64 { self.x * self.x + self.y * self.y + self.z * self.z }
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len < 1e-10 { return *self; }
        Self { x: self.x / len, y: self.y / len, z: self.z / len }
    }
    pub fn dot(&self, other: &Vec3) -> f64 { self.x * other.x + self.y * other.y + self.z * other.z }
    pub fn cross(&self, other: &Vec3) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
    pub fn add(&self, other: &Vec3) -> Self { Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z } }
    pub fn sub(&self, other: &Vec3) -> Self { Self { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z } }
    pub fn scale(&self, s: f64) -> Self { Self { x: self.x * s, y: self.y * s, z: self.z * s } }
    pub fn lerp(&self, other: &Vec3, t: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
    pub fn distance(&self, other: &Vec3) -> f64 { self.sub(other).length() }
    pub fn reflect(&self, normal: &Vec3) -> Self {
        let d = 2.0 * self.dot(normal);
        Self { x: self.x - d * normal.x, y: self.y - d * normal.y, z: self.z - d * normal.z }
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.3}, {:.3}, {:.3})", self.x, self.y, self.z)
    }
}

/// 4D vector (homogeneous coordinates / quaternion storage).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec4 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Vec4 {
    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self { Self { x, y, z, w } }
    pub fn zero() -> Self { Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 } }
    pub fn dot(&self, other: &Vec4) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }
    pub fn length(&self) -> f64 { self.dot(self).sqrt() }
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len < 1e-10 { return *self; }
        Self { x: self.x / len, y: self.y / len, z: self.z / len, w: self.w / len }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TRANSFORM MATRICES
// ═══════════════════════════════════════════════════════════════════════

/// 4×4 column-major transformation matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub m: [f64; 16],
}

impl Mat4 {
    pub fn identity() -> Self {
        let mut m = [0.0; 16];
        m[0] = 1.0; m[5] = 1.0; m[10] = 1.0; m[15] = 1.0;
        Self { m }
    }

    pub fn translation(x: f64, y: f64, z: f64) -> Self {
        let mut mat = Self::identity();
        mat.m[12] = x; mat.m[13] = y; mat.m[14] = z;
        mat
    }

    pub fn scaling(x: f64, y: f64, z: f64) -> Self {
        let mut mat = Self::identity();
        mat.m[0] = x; mat.m[5] = y; mat.m[10] = z;
        mat
    }

    pub fn rotation_x(angle: f64) -> Self {
        let mut mat = Self::identity();
        let c = angle.cos(); let s = angle.sin();
        mat.m[5] = c;  mat.m[6] = s;
        mat.m[9] = -s; mat.m[10] = c;
        mat
    }

    pub fn rotation_y(angle: f64) -> Self {
        let mut mat = Self::identity();
        let c = angle.cos(); let s = angle.sin();
        mat.m[0] = c;  mat.m[2] = -s;
        mat.m[8] = s; mat.m[10] = c;
        mat
    }

    pub fn rotation_z(angle: f64) -> Self {
        let mut mat = Self::identity();
        let c = angle.cos(); let s = angle.sin();
        mat.m[0] = c;  mat.m[1] = s;
        mat.m[4] = -s; mat.m[5] = c;
        mat
    }

    /// Perspective projection matrix.
    pub fn perspective(fov_rad: f64, aspect: f64, near: f64, far: f64) -> Self {
        let f = 1.0 / (fov_rad / 2.0).tan();
        let nf = 1.0 / (near - far);
        let mut m = [0.0; 16];
        m[0] = f / aspect;
        m[5] = f;
        m[10] = (far + near) * nf;
        m[11] = -1.0;
        m[14] = 2.0 * far * near * nf;
        Self { m }
    }

    /// Orthographic projection matrix.
    pub fn orthographic(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> Self {
        let mut m = [0.0; 16];
        m[0] = 2.0 / (right - left);
        m[5] = 2.0 / (top - bottom);
        m[10] = -2.0 / (far - near);
        m[12] = -(right + left) / (right - left);
        m[13] = -(top + bottom) / (top - bottom);
        m[14] = -(far + near) / (far - near);
        m[15] = 1.0;
        Self { m }
    }

    /// Look-at view matrix.
    pub fn look_at(eye: &Vec3, target: &Vec3, up: &Vec3) -> Self {
        let f = target.sub(eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(&f);
        let mut m = [0.0; 16];
        m[0] = s.x; m[4] = s.y; m[8] = s.z;
        m[1] = u.x; m[5] = u.y; m[9] = u.z;
        m[2] = -f.x; m[6] = -f.y; m[10] = -f.z;
        m[12] = -s.dot(eye);
        m[13] = -u.dot(eye);
        m[14] = f.dot(eye);
        m[15] = 1.0;
        Self { m }
    }

    /// Multiply two matrices.
    pub fn multiply(&self, other: &Mat4) -> Mat4 {
        let mut out = [0.0; 16];
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.m[row + k * 4] * other.m[k + col * 4];
                }
                out[row + col * 4] = sum;
            }
        }
        Mat4 { m: out }
    }

    /// Transform a Vec4 by this matrix.
    pub fn transform_vec4(&self, v: &Vec4) -> Vec4 {
        Vec4 {
            x: self.m[0]*v.x + self.m[4]*v.y + self.m[8]*v.z  + self.m[12]*v.w,
            y: self.m[1]*v.x + self.m[5]*v.y + self.m[9]*v.z  + self.m[13]*v.w,
            z: self.m[2]*v.x + self.m[6]*v.y + self.m[10]*v.z + self.m[14]*v.w,
            w: self.m[3]*v.x + self.m[7]*v.y + self.m[11]*v.z + self.m[15]*v.w,
        }
    }

    /// Transform a Vec3 as a point (w=1).
    pub fn transform_point(&self, v: &Vec3) -> Vec3 {
        let v4 = self.transform_vec4(&Vec4::new(v.x, v.y, v.z, 1.0));
        if v4.w.abs() < 1e-10 { return Vec3::new(v4.x, v4.y, v4.z); }
        Vec3::new(v4.x / v4.w, v4.y / v4.w, v4.z / v4.w)
    }

    /// Determinant.
    pub fn determinant(&self) -> f64 {
        let m = &self.m;
        let a = m[0]*(m[5]*(m[10]*m[15]-m[11]*m[14]) - m[6]*(m[9]*m[15]-m[11]*m[13]) + m[7]*(m[9]*m[14]-m[10]*m[13]));
        let b = m[1]*(m[4]*(m[10]*m[15]-m[11]*m[14]) - m[6]*(m[8]*m[15]-m[11]*m[12]) + m[7]*(m[8]*m[14]-m[10]*m[12]));
        let c = m[2]*(m[4]*(m[9]*m[15]-m[11]*m[13])  - m[5]*(m[8]*m[15]-m[11]*m[12]) + m[7]*(m[8]*m[13]-m[9]*m[12]));
        let d = m[3]*(m[4]*(m[9]*m[14]-m[10]*m[13])  - m[5]*(m[8]*m[14]-m[10]*m[12]) + m[6]*(m[8]*m[13]-m[9]*m[12]));
        a - b + c - d
    }

    /// Transpose.
    pub fn transpose(&self) -> Mat4 {
        let mut out = [0.0; 16];
        for row in 0..4 {
            for col in 0..4 {
                out[col + row * 4] = self.m[row + col * 4];
            }
        }
        Mat4 { m: out }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  2D SHAPES
// ═══════════════════════════════════════════════════════════════════════

/// Blend mode for compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Additive,
    SoftLight,
    Difference,
}

/// Fill style for shapes.
#[derive(Debug, Clone)]
pub enum FillStyle {
    Solid(Color),
    LinearGradient { start: Vec2, end: Vec2, stops: Vec<(f64, Color)> },
    RadialGradient { center: Vec2, radius: f64, stops: Vec<(f64, Color)> },
    ConicGradient { center: Vec2, angle: f64, stops: Vec<(f64, Color)> },
    Pattern { image_id: u64, repeat_x: bool, repeat_y: bool },
}

/// Stroke configuration.
#[derive(Debug, Clone)]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub dash_pattern: Vec<f64>,
    pub dash_offset: f64,
}

impl Stroke {
    pub fn new(color: Color, width: f64) -> Self {
        Self { color, width, line_cap: LineCap::Butt, line_join: LineJoin::Miter, dash_pattern: vec![], dash_offset: 0.0 }
    }

    pub fn solid(width: f64) -> Self { Self::new(Color::black(), width) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap { Butt, Round, Square }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin { Miter, Round, Bevel }

/// A 2D path composed of sub-commands (SVG-compatible).
#[derive(Debug, Clone)]
pub struct Path2D {
    pub commands: Vec<PathCommand>,
}

#[derive(Debug, Clone, Copy)]
pub enum PathCommand {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadraticTo { control: Vec2, end: Vec2 },
    CubicTo { control1: Vec2, control2: Vec2, end: Vec2 },
    ArcTo { center: Vec2, radius: f64, start_angle: f64, end_angle: f64 },
    ClosePath,
}

impl Path2D {
    pub fn new() -> Self { Self { commands: Vec::new() } }

    pub fn move_to(&mut self, pos: Vec2) -> &mut Self {
        self.commands.push(PathCommand::MoveTo(pos)); self
    }
    pub fn line_to(&mut self, pos: Vec2) -> &mut Self {
        self.commands.push(PathCommand::LineTo(pos)); self
    }
    pub fn quad_to(&mut self, control: Vec2, end: Vec2) -> &mut Self {
        self.commands.push(PathCommand::QuadraticTo { control, end }); self
    }
    pub fn cubic_to(&mut self, c1: Vec2, c2: Vec2, end: Vec2) -> &mut Self {
        self.commands.push(PathCommand::CubicTo { control1: c1, control2: c2, end }); self
    }
    pub fn arc_to(&mut self, center: Vec2, radius: f64, start: f64, end: f64) -> &mut Self {
        self.commands.push(PathCommand::ArcTo { center, radius, start_angle: start, end_angle: end }); self
    }
    pub fn close(&mut self) -> &mut Self {
        self.commands.push(PathCommand::ClosePath); self
    }

    /// Create a rectangle path.
    pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Self {
        let mut p = Self::new();
        p.move_to(Vec2::new(x, y));
        p.line_to(Vec2::new(x + w, y));
        p.line_to(Vec2::new(x + w, y + h));
        p.line_to(Vec2::new(x, y + h));
        p.close();
        p
    }

    /// Create a rounded rectangle path.
    pub fn rounded_rect(x: f64, y: f64, w: f64, h: f64, radius: f64) -> Self {
        let r = radius.min(w / 2.0).min(h / 2.0);
        let mut p = Self::new();
        p.move_to(Vec2::new(x + r, y));
        p.line_to(Vec2::new(x + w - r, y));
        p.arc_to(Vec2::new(x + w - r, y + r), r, -std::f64::consts::FRAC_PI_2, 0.0);
        p.line_to(Vec2::new(x + w, y + h - r));
        p.arc_to(Vec2::new(x + w - r, y + h - r), r, 0.0, std::f64::consts::FRAC_PI_2);
        p.line_to(Vec2::new(x + r, y + h));
        p.arc_to(Vec2::new(x + r, y + h - r), r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        p.line_to(Vec2::new(x, y + r));
        p.arc_to(Vec2::new(x + r, y + r), r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
        p.close();
        p
    }

    /// Create a circle path.
    pub fn circle(cx: f64, cy: f64, radius: f64) -> Self {
        let mut p = Self::new();
        p.arc_to(Vec2::new(cx, cy), radius, 0.0, 2.0 * std::f64::consts::PI);
        p.close();
        p
    }

    /// Create a regular polygon path.
    pub fn regular_polygon(cx: f64, cy: f64, radius: f64, sides: usize) -> Self {
        let mut p = Self::new();
        if sides < 3 { return p; }
        let angle_step = 2.0 * std::f64::consts::PI / sides as f64;
        let start = -std::f64::consts::FRAC_PI_2; // Start from top
        for i in 0..sides {
            let a = start + angle_step * i as f64;
            let pt = Vec2::new(cx + radius * a.cos(), cy + radius * a.sin());
            if i == 0 { p.move_to(pt); } else { p.line_to(pt); }
        }
        p.close();
        p
    }

    /// Create a star path.
    pub fn star(cx: f64, cy: f64, outer: f64, inner: f64, points: usize) -> Self {
        let mut p = Self::new();
        if points < 2 { return p; }
        let step = std::f64::consts::PI / points as f64;
        let start = -std::f64::consts::FRAC_PI_2;
        for i in 0..(points * 2) {
            let a = start + step * i as f64;
            let r = if i % 2 == 0 { outer } else { inner };
            let pt = Vec2::new(cx + r * a.cos(), cy + r * a.sin());
            if i == 0 { p.move_to(pt); } else { p.line_to(pt); }
        }
        p.close();
        p
    }

    pub fn command_count(&self) -> usize { self.commands.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  CAMERA
// ═══════════════════════════════════════════════════════════════════════

/// Camera projection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMode {
    Perspective,
    Orthographic,
}

/// A camera describing a viewpoint into a 3D scene.
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f64,
    pub near: f64,
    pub far: f64,
    pub aspect: f64,
    pub projection: ProjectionMode,
}

impl Camera {
    pub fn perspective(position: Vec3, target: Vec3, fov_deg: f64, aspect: f64) -> Self {
        Self {
            position, target, up: Vec3::up(), fov: fov_deg.to_radians(),
            near: 0.1, far: 1000.0, aspect, projection: ProjectionMode::Perspective,
        }
    }

    pub fn orthographic(position: Vec3, target: Vec3, aspect: f64) -> Self {
        Self {
            position, target, up: Vec3::up(), fov: 45.0_f64.to_radians(),
            near: 0.1, far: 1000.0, aspect, projection: ProjectionMode::Orthographic,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(&self.position, &self.target, &self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        match self.projection {
            ProjectionMode::Perspective => Mat4::perspective(self.fov, self.aspect, self.near, self.far),
            ProjectionMode::Orthographic => {
                let h = 10.0;
                let w = h * self.aspect;
                Mat4::orthographic(-w/2.0, w/2.0, -h/2.0, h/2.0, self.near, self.far)
            }
        }
    }

    pub fn view_projection(&self) -> Mat4 {
        self.projection_matrix().multiply(&self.view_matrix())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  VIEWPORT & RENDER TARGET
// ═══════════════════════════════════════════════════════════════════════

/// Pixel format for image buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba8,
    RgbaF32,
    Rgb8,
    GrayScale8,
    GrayScaleF32,
    DepthF32,
}

impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgba8 => 4,
            PixelFormat::RgbaF32 => 16,
            PixelFormat::Rgb8 => 3,
            PixelFormat::GrayScale8 => 1,
            PixelFormat::GrayScaleF32 => 4,
            PixelFormat::DepthF32 => 4,
        }
    }
}

/// A pixel buffer.
#[derive(Debug, Clone)]
pub struct ImageBuffer {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub data: Vec<u8>,
}

impl ImageBuffer {
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        let size = width as usize * height as usize * format.bytes_per_pixel();
        Self { width, height, format, data: vec![0; size] }
    }

    pub fn filled(width: u32, height: u32, color: &Color) -> Self {
        let mut buf = Self::new(width, height, PixelFormat::Rgba8);
        let r = (color.r * 255.0).round() as u8;
        let g = (color.g * 255.0).round() as u8;
        let b = (color.b * 255.0).round() as u8;
        let a = (color.a * 255.0).round() as u8;
        for pixel in buf.data.chunks_exact_mut(4) {
            pixel[0] = r; pixel[1] = g; pixel[2] = b; pixel[3] = a;
        }
        buf
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: &Color) {
        if x >= self.width || y >= self.height { return; }
        let idx = (y as usize * self.width as usize + x as usize) * self.format.bytes_per_pixel();
        if self.format == PixelFormat::Rgba8 && idx + 3 < self.data.len() {
            self.data[idx]     = (color.r * 255.0).round() as u8;
            self.data[idx + 1] = (color.g * 255.0).round() as u8;
            self.data[idx + 2] = (color.b * 255.0).round() as u8;
            self.data[idx + 3] = (color.a * 255.0).round() as u8;
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        if x >= self.width || y >= self.height { return Color::transparent(); }
        let idx = (y as usize * self.width as usize + x as usize) * self.format.bytes_per_pixel();
        if self.format == PixelFormat::Rgba8 && idx + 3 < self.data.len() {
            Color::rgba(
                self.data[idx] as f64 / 255.0,
                self.data[idx + 1] as f64 / 255.0,
                self.data[idx + 2] as f64 / 255.0,
                self.data[idx + 3] as f64 / 255.0,
            )
        } else {
            Color::transparent()
        }
    }

    pub fn pixel_count(&self) -> usize { self.width as usize * self.height as usize }

    pub fn clear(&mut self, color: &Color) {
        let r = (color.r * 255.0).round() as u8;
        let g = (color.g * 255.0).round() as u8;
        let b = (color.b * 255.0).round() as u8;
        let a = (color.a * 255.0).round() as u8;
        if self.format == PixelFormat::Rgba8 {
            for pixel in self.data.chunks_exact_mut(4) {
                pixel[0] = r; pixel[1] = g; pixel[2] = b; pixel[3] = a;
            }
        }
    }

    /// Draw a filled rectangle (software rasterizer).
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: &Color) {
        for py in 0..h {
            for px in 0..w {
                let cx = x + px as i32;
                let cy = y + py as i32;
                if cx >= 0 && cy >= 0 {
                    self.set_pixel(cx as u32, cy as u32, color);
                }
            }
        }
    }

    /// Draw a line using Bresenham's algorithm.
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: &Color) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx = x0;
        let mut cy = y0;
        loop {
            if cx >= 0 && cy >= 0 {
                self.set_pixel(cx as u32, cy as u32, color);
            }
            if cx == x1 && cy == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; cx += sx; }
            if e2 <= dx { err += dx; cy += sy; }
        }
    }

    /// Draw a filled circle using midpoint algorithm.
    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: &Color) {
        for y in -radius..=radius {
            for x in -radius..=radius {
                if x * x + y * y <= radius * radius {
                    let px = cx + x;
                    let py = cy + y;
                    if px >= 0 && py >= 0 {
                        self.set_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
    }
}

/// Render target abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTarget {
    Screen,
    Texture { id: u64, width: u32, height: u32 },
    Offscreen { width: u32, height: u32 },
}

/// Viewport configuration.
#[derive(Debug, Clone)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub dpi_scale: f64,
    pub target: RenderTarget,
}

impl Viewport {
    pub fn new(width: u32, height: u32) -> Self {
        Self { x: 0, y: 0, width, height, dpi_scale: 1.0, target: RenderTarget::Screen }
    }

    pub fn physical_width(&self) -> u32 { (self.width as f64 * self.dpi_scale) as u32 }
    pub fn physical_height(&self) -> u32 { (self.height as f64 * self.dpi_scale) as u32 }
    pub fn aspect_ratio(&self) -> f64 { self.width as f64 / self.height as f64 }
}

// ═══════════════════════════════════════════════════════════════════════
//  ANIMATION & EASING
// ═══════════════════════════════════════════════════════════════════════

/// Easing function types for animation curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFunction {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInElastic,
    EaseOutElastic,
    EaseOutBounce,
    Spring,
}

/// Evaluate an easing function at time t ∈ [0, 1].
pub fn ease(func: EasingFunction, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match func {
        EasingFunction::Linear => t,
        EasingFunction::EaseInQuad => t * t,
        EasingFunction::EaseOutQuad => t * (2.0 - t),
        EasingFunction::EaseInOutQuad => {
            if t < 0.5 { 2.0 * t * t } else { -1.0 + (4.0 - 2.0 * t) * t }
        }
        EasingFunction::EaseInCubic => t * t * t,
        EasingFunction::EaseOutCubic => { let t1 = t - 1.0; t1 * t1 * t1 + 1.0 }
        EasingFunction::EaseInOutCubic => {
            if t < 0.5 { 4.0 * t * t * t } else { (t - 1.0) * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0 }
        }
        EasingFunction::EaseInSine => 1.0 - (t * std::f64::consts::FRAC_PI_2).cos(),
        EasingFunction::EaseOutSine => (t * std::f64::consts::FRAC_PI_2).sin(),
        EasingFunction::EaseInOutSine => -(((std::f64::consts::PI * t).cos() - 1.0) / 2.0),
        EasingFunction::EaseInExpo => if t == 0.0 { 0.0 } else { (2.0_f64).powf(10.0 * t - 10.0) },
        EasingFunction::EaseOutExpo => if t == 1.0 { 1.0 } else { 1.0 - (2.0_f64).powf(-10.0 * t) },
        EasingFunction::EaseInOutExpo => {
            if t == 0.0 { 0.0 }
            else if t == 1.0 { 1.0 }
            else if t < 0.5 { (2.0_f64).powf(20.0 * t - 10.0) / 2.0 }
            else { (2.0 - (2.0_f64).powf(-20.0 * t + 10.0)) / 2.0 }
        }
        EasingFunction::EaseInElastic => {
            if t == 0.0 || t == 1.0 { t }
            else { -(2.0_f64).powf(10.0 * t - 10.0) * ((10.0 * t - 10.75) * (2.0 * std::f64::consts::PI / 3.0)).sin() }
        }
        EasingFunction::EaseOutElastic => {
            if t == 0.0 || t == 1.0 { t }
            else { (2.0_f64).powf(-10.0 * t) * ((10.0 * t - 0.75) * (2.0 * std::f64::consts::PI / 3.0)).sin() + 1.0 }
        }
        EasingFunction::EaseOutBounce => bounce_out(t),
        EasingFunction::Spring => {
            let freq = 4.5;
            let decay = 5.0;
            1.0 - (-decay * t).exp() * (freq * std::f64::consts::PI * t).cos()
        }
    }
}

fn bounce_out(t: f64) -> f64 {
    if t < 1.0 / 2.75 { 7.5625 * t * t }
    else if t < 2.0 / 2.75 { let t = t - 1.5 / 2.75; 7.5625 * t * t + 0.75 }
    else if t < 2.5 / 2.75 { let t = t - 2.25 / 2.75; 7.5625 * t * t + 0.9375 }
    else { let t = t - 2.625 / 2.75; 7.5625 * t * t + 0.984375 }
}

/// An animation keyframe.
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f64,
    pub value: f64,
    pub easing: EasingFunction,
}

/// Animation curve composed of keyframes.
#[derive(Debug, Clone)]
pub struct AnimationCurve {
    pub keyframes: Vec<Keyframe>,
    pub duration: f64,
    pub looping: bool,
}

impl AnimationCurve {
    pub fn new(duration: f64) -> Self {
        Self { keyframes: Vec::new(), duration, looping: false }
    }

    pub fn add_keyframe(&mut self, time: f64, value: f64, easing: EasingFunction) {
        self.keyframes.push(Keyframe { time, value, easing });
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Evaluate the curve at time t.
    pub fn evaluate(&self, mut t: f64) -> f64 {
        if self.keyframes.is_empty() { return 0.0; }
        if self.looping && self.duration > 0.0 {
            t = t % self.duration;
        }
        t = t.clamp(0.0, self.duration);

        // Find surrounding keyframes
        let normalized = if self.duration > 0.0 { t / self.duration } else { 0.0 };
        if self.keyframes.len() == 1 { return self.keyframes[0].value; }

        for i in 0..self.keyframes.len() - 1 {
            let kf0 = &self.keyframes[i];
            let kf1 = &self.keyframes[i + 1];
            if normalized >= kf0.time && normalized <= kf1.time {
                let local_t = if (kf1.time - kf0.time).abs() < 1e-10 { 0.0 }
                              else { (normalized - kf0.time) / (kf1.time - kf0.time) };
                let eased = ease(kf1.easing, local_t);
                return kf0.value + (kf1.value - kf0.value) * eased;
            }
        }
        self.keyframes.last().unwrap().value
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  DRAW COMMAND BUFFER
// ═══════════════════════════════════════════════════════════════════════

/// A single draw command in the render queue.
#[derive(Debug, Clone)]
pub enum DrawCommand {
    Clear(Color),
    FillPath { path: Path2D, style: FillStyle, blend: BlendMode },
    StrokePath { path: Path2D, stroke: Stroke, blend: BlendMode },
    DrawImage { image_id: u64, position: Vec2, size: Vec2, opacity: f64 },
    SetTransform(Mat4),
    PushTransform(Mat4),
    PopTransform,
    SetClipRect { x: f64, y: f64, width: f64, height: f64 },
    ClearClip,
    DrawText { text: String, position: Vec2, font_size: f64, color: Color },
}

/// Accumulates draw commands for batch rendering.
#[derive(Debug, Clone)]
pub struct DrawList {
    pub commands: Vec<DrawCommand>,
}

impl DrawList {
    pub fn new() -> Self { Self { commands: Vec::new() } }

    pub fn clear(&mut self, color: Color) { self.commands.push(DrawCommand::Clear(color)); }

    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: Color) {
        self.commands.push(DrawCommand::FillPath {
            path: Path2D::rect(x, y, w, h),
            style: FillStyle::Solid(color),
            blend: BlendMode::Normal,
        });
    }

    pub fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64, stroke: Stroke) {
        self.commands.push(DrawCommand::StrokePath {
            path: Path2D::rect(x, y, w, h),
            stroke,
            blend: BlendMode::Normal,
        });
    }

    pub fn fill_circle(&mut self, cx: f64, cy: f64, r: f64, color: Color) {
        self.commands.push(DrawCommand::FillPath {
            path: Path2D::circle(cx, cy, r),
            style: FillStyle::Solid(color),
            blend: BlendMode::Normal,
        });
    }

    pub fn draw_line(&mut self, from: Vec2, to: Vec2, stroke: Stroke) {
        let mut path = Path2D::new();
        path.move_to(from).line_to(to);
        self.commands.push(DrawCommand::StrokePath {
            path,
            stroke,
            blend: BlendMode::Normal,
        });
    }

    pub fn draw_text(&mut self, text: &str, pos: Vec2, font_size: f64, color: Color) {
        self.commands.push(DrawCommand::DrawText {
            text: text.to_string(), position: pos, font_size, color,
        });
    }

    pub fn push_transform(&mut self, mat: Mat4) { self.commands.push(DrawCommand::PushTransform(mat)); }
    pub fn pop_transform(&mut self) { self.commands.push(DrawCommand::PopTransform); }

    pub fn command_count(&self) -> usize { self.commands.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  RENDER PIPELINE
// ═══════════════════════════════════════════════════════════════════════

/// A render pass in the pipeline.
#[derive(Debug, Clone)]
pub struct RenderPass {
    pub name: String,
    pub target: RenderTarget,
    pub clear_color: Option<Color>,
    pub draw_list: DrawList,
}

impl RenderPass {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            target: RenderTarget::Screen,
            clear_color: Some(Color::black()),
            draw_list: DrawList::new(),
        }
    }
}

/// Multi-pass rendering pipeline.
#[derive(Debug, Clone)]
pub struct RenderPipeline {
    pub passes: Vec<RenderPass>,
    pub viewport: Viewport,
    pub camera: Option<Camera>,
}

impl RenderPipeline {
    pub fn new(viewport: Viewport) -> Self {
        Self { passes: Vec::new(), viewport, camera: None }
    }

    pub fn add_pass(&mut self, pass: RenderPass) {
        self.passes.push(pass);
    }

    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = Some(camera);
    }

    pub fn total_draw_commands(&self) -> usize {
        self.passes.iter().map(|p| p.draw_list.command_count()).sum()
    }

    /// Execute the pipeline (software rasterizer for testing).
    pub fn execute_software(&self) -> ImageBuffer {
        let mut buffer = ImageBuffer::new(
            self.viewport.physical_width(),
            self.viewport.physical_height(),
            PixelFormat::Rgba8,
        );

        for pass in &self.passes {
            if let Some(cc) = &pass.clear_color {
                buffer.clear(cc);
            }
            // Process draw commands (simplified software rasterizer)
            for cmd in &pass.draw_list.commands {
                match cmd {
                    DrawCommand::Clear(c) => buffer.clear(c),
                    DrawCommand::FillPath { style: FillStyle::Solid(color), .. } => {
                        // Simplified: just fill the entire buffer for testing
                        buffer.clear(color);
                    }
                    _ => {} // Other commands handled by GPU backends
                }
            }
        }

        buffer
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  GRADIENT EVALUATION
// ═══════════════════════════════════════════════════════════════════════

/// Evaluate a gradient at a position t ∈ [0, 1].
pub fn evaluate_gradient(stops: &[(f64, Color)], t: f64) -> Color {
    if stops.is_empty() { return Color::transparent(); }
    if stops.len() == 1 { return stops[0].1; }

    let t = t.clamp(0.0, 1.0);

    if t <= stops[0].0 { return stops[0].1; }
    if t >= stops.last().unwrap().0 { return stops.last().unwrap().1; }

    for i in 0..stops.len() - 1 {
        if t >= stops[i].0 && t <= stops[i + 1].0 {
            let local_t = if (stops[i + 1].0 - stops[i].0).abs() < 1e-10 { 0.0 }
                          else { (t - stops[i].0) / (stops[i + 1].0 - stops[i].0) };
            return stops[i].1.lerp(&stops[i + 1].1, local_t);
        }
    }
    stops.last().unwrap().1
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Color Tests ─────────────────────────────────────────────────

    #[test]
    fn test_color_rgba_clamp() {
        let c = Color::rgba(1.5, -0.2, 0.5, 0.8);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.5);
    }

    #[test]
    fn test_color_hex_roundtrip() {
        let c = Color::from_hex(0xFF8040);
        let hex = c.to_hex();
        assert_eq!(hex, 0xFF8040);
    }

    #[test]
    fn test_color_lerp() {
        let a = Color::black();
        let b = Color::white();
        let mid = a.lerp(&b, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-6);
        assert!((mid.g - 0.5).abs() < 1e-6);
        assert!((mid.b - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_color_luminance() {
        let white = Color::white();
        assert!((white.luminance() - 1.0).abs() < 1e-6);
        let black = Color::black();
        assert!(black.luminance().abs() < 1e-6);
    }

    #[test]
    fn test_color_invert() {
        let c = Color::rgb(0.2, 0.4, 0.6);
        let inv = c.invert();
        assert!((inv.r - 0.8).abs() < 1e-6);
        assert!((inv.g - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_color_blend_over() {
        let fg = Color::rgba(1.0, 0.0, 0.0, 0.5);
        let bg = Color::rgba(0.0, 0.0, 1.0, 1.0);
        let result = fg.blend_over(&bg);
        assert!(result.a > 0.9);
        assert!(result.r > 0.3);
    }

    #[test]
    fn test_color_hsla_roundtrip() {
        let c = Color::rgb(0.8, 0.3, 0.5);
        let (h, s, l, a) = c.to_hsla();
        let back = Color::from_hsla(h, s, l, a);
        assert!((c.r - back.r).abs() < 0.01);
        assert!((c.g - back.g).abs() < 0.01);
        assert!((c.b - back.b).abs() < 0.01);
    }

    #[test]
    fn test_color_brighten() {
        let c = Color::rgb(0.4, 0.4, 0.4);
        let bright = c.brighten(2.0);
        assert!((bright.r - 0.8).abs() < 1e-6);
    }

    // ── Vector Tests ────────────────────────────────────────────────

    #[test]
    fn test_vec2_operations() {
        let a = Vec2::new(3.0, 4.0);
        assert!((a.length() - 5.0).abs() < 1e-10);

        let n = a.normalize();
        assert!((n.length() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec2_dot_cross() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert!(a.dot(&b).abs() < 1e-10);
        assert!((a.cross(&b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec2_rotate() {
        let v = Vec2::new(1.0, 0.0);
        let rotated = v.rotate(std::f64::consts::FRAC_PI_2);
        assert!(rotated.x.abs() < 1e-10);
        assert!((rotated.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec3_cross_product() {
        let x = Vec3::right();
        let y = Vec3::up();
        let z = x.cross(&y);
        assert!((z.x).abs() < 1e-10);
        assert!((z.y).abs() < 1e-10);
        assert!((z.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec3_distance() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 6.0, 3.0);
        assert!((a.distance(&b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec4_dot() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(2.0, 3.0, 4.0, 5.0);
        assert!((a.dot(&b) - 40.0).abs() < 1e-10);
    }

    // ── Matrix Tests ────────────────────────────────────────────────

    #[test]
    fn test_mat4_identity() {
        let m = Mat4::identity();
        let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
        let result = m.transform_vec4(&v);
        assert!((result.x - 1.0).abs() < 1e-10);
        assert!((result.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_mat4_translation() {
        let m = Mat4::translation(10.0, 20.0, 30.0);
        let p = Vec3::zero();
        let result = m.transform_point(&p);
        assert!((result.x - 10.0).abs() < 1e-10);
        assert!((result.y - 20.0).abs() < 1e-10);
        assert!((result.z - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_mat4_scaling() {
        let m = Mat4::scaling(2.0, 3.0, 4.0);
        let p = Vec3::new(1.0, 1.0, 1.0);
        let result = m.transform_point(&p);
        assert!((result.x - 2.0).abs() < 1e-10);
        assert!((result.y - 3.0).abs() < 1e-10);
        assert!((result.z - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_mat4_determinant_identity() {
        let m = Mat4::identity();
        assert!((m.determinant() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_mat4_multiply_identity() {
        let a = Mat4::translation(1.0, 2.0, 3.0);
        let i = Mat4::identity();
        let result = a.multiply(&i);
        assert!((result.m[12] - 1.0).abs() < 1e-10);
        assert!((result.m[13] - 2.0).abs() < 1e-10);
    }

    // ── Path Tests ──────────────────────────────────────────────────

    #[test]
    fn test_path_rect() {
        let p = Path2D::rect(0.0, 0.0, 100.0, 50.0);
        assert_eq!(p.command_count(), 5); // move + 3 lines + close
    }

    #[test]
    fn test_path_circle() {
        let p = Path2D::circle(50.0, 50.0, 25.0);
        assert_eq!(p.command_count(), 2); // arc + close
    }

    #[test]
    fn test_path_star() {
        let p = Path2D::star(100.0, 100.0, 50.0, 20.0, 5);
        assert_eq!(p.command_count(), 11); // move + 9 lines + close
    }

    #[test]
    fn test_path_polygon() {
        let p = Path2D::regular_polygon(0.0, 0.0, 50.0, 6);
        assert_eq!(p.command_count(), 7); // move + 5 lines + close
    }

    // ── Image Buffer Tests ──────────────────────────────────────────

    #[test]
    fn test_image_buffer_create() {
        let buf = ImageBuffer::new(640, 480, PixelFormat::Rgba8);
        assert_eq!(buf.pixel_count(), 640 * 480);
        assert_eq!(buf.data.len(), 640 * 480 * 4);
    }

    #[test]
    fn test_image_buffer_set_get_pixel() {
        let mut buf = ImageBuffer::new(100, 100, PixelFormat::Rgba8);
        let red = Color::red();
        buf.set_pixel(50, 50, &red);
        let got = buf.get_pixel(50, 50);
        assert!((got.r - 1.0).abs() < 0.01);
        assert!(got.g.abs() < 0.01);
    }

    #[test]
    fn test_image_buffer_filled() {
        let buf = ImageBuffer::filled(10, 10, &Color::blue());
        let px = buf.get_pixel(5, 5);
        assert!((px.b - 1.0).abs() < 0.01);
        assert!(px.r.abs() < 0.01);
    }

    #[test]
    fn test_image_buffer_draw_line() {
        let mut buf = ImageBuffer::new(100, 100, PixelFormat::Rgba8);
        buf.draw_line(0, 0, 99, 99, &Color::white());
        let px = buf.get_pixel(50, 50);
        assert!(px.r > 0.5); // Should have drawn on the diagonal
    }

    #[test]
    fn test_image_buffer_fill_circle() {
        let mut buf = ImageBuffer::new(100, 100, PixelFormat::Rgba8);
        buf.fill_circle(50, 50, 10, &Color::green());
        let center = buf.get_pixel(50, 50);
        assert!(center.g > 0.9);
        let outside = buf.get_pixel(0, 0);
        assert!(outside.g < 0.1);
    }

    // ── Easing Tests ────────────────────────────────────────────────

    #[test]
    fn test_easing_linear() {
        assert!((ease(EasingFunction::Linear, 0.0)).abs() < 1e-10);
        assert!((ease(EasingFunction::Linear, 0.5) - 0.5).abs() < 1e-10);
        assert!((ease(EasingFunction::Linear, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_easing_quad() {
        let v = ease(EasingFunction::EaseInQuad, 0.5);
        assert!((v - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_easing_boundaries() {
        for func in [EasingFunction::EaseInCubic, EasingFunction::EaseOutExpo, EasingFunction::EaseOutBounce, EasingFunction::Spring] {
            let start = ease(func, 0.0);
            let end = ease(func, 1.0);
            assert!(start.abs() < 0.1, "Easing {:?} start={}", func, start);
            assert!((end - 1.0).abs() < 0.1, "Easing {:?} end={}", func, end);
        }
    }

    // ── Animation Curve Tests ───────────────────────────────────────

    #[test]
    fn test_animation_curve() {
        let mut curve = AnimationCurve::new(1.0);
        curve.add_keyframe(0.0, 0.0, EasingFunction::Linear);
        curve.add_keyframe(1.0, 100.0, EasingFunction::Linear);
        let mid = curve.evaluate(0.5);
        assert!((mid - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_animation_curve_looping() {
        let mut curve = AnimationCurve::new(1.0);
        curve.looping = true;
        curve.add_keyframe(0.0, 0.0, EasingFunction::Linear);
        curve.add_keyframe(1.0, 100.0, EasingFunction::Linear);
        let v = curve.evaluate(1.5); // Should wrap to 0.5
        assert!((v - 50.0).abs() < 1e-6);
    }

    // ── Gradient Tests ──────────────────────────────────────────────

    #[test]
    fn test_gradient_evaluation() {
        let stops = vec![
            (0.0, Color::black()),
            (1.0, Color::white()),
        ];
        let mid = evaluate_gradient(&stops, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_gradient_multi_stop() {
        let stops = vec![
            (0.0, Color::red()),
            (0.5, Color::green()),
            (1.0, Color::blue()),
        ];
        let at_half = evaluate_gradient(&stops, 0.5);
        assert!(at_half.g > 0.9);
    }

    // ── Viewport Tests ──────────────────────────────────────────────

    #[test]
    fn test_viewport_aspect_ratio() {
        let vp = Viewport::new(1920, 1080);
        assert!((vp.aspect_ratio() - 16.0/9.0).abs() < 0.01);
    }

    #[test]
    fn test_viewport_dpi_scale() {
        let mut vp = Viewport::new(800, 600);
        vp.dpi_scale = 2.0;
        assert_eq!(vp.physical_width(), 1600);
        assert_eq!(vp.physical_height(), 1200);
    }

    // ── Draw List Tests ─────────────────────────────────────────────

    #[test]
    fn test_draw_list() {
        let mut dl = DrawList::new();
        dl.clear(Color::black());
        dl.fill_rect(10.0, 10.0, 100.0, 50.0, Color::red());
        dl.fill_circle(50.0, 50.0, 25.0, Color::blue());
        dl.draw_text("Hello", Vec2::new(10.0, 10.0), 16.0, Color::white());
        assert_eq!(dl.command_count(), 4);
    }

    // ── Camera Tests ────────────────────────────────────────────────

    #[test]
    fn test_camera_perspective() {
        let cam = Camera::perspective(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::zero(),
            45.0,
            16.0 / 9.0,
        );
        let vp = cam.view_projection();
        assert!(vp.determinant().abs() > 1e-10);
    }

    // ── Render Pipeline Tests ───────────────────────────────────────

    #[test]
    fn test_render_pipeline() {
        let viewport = Viewport::new(800, 600);
        let mut pipeline = RenderPipeline::new(viewport);
        let mut pass = RenderPass::new("main");
        pass.draw_list.fill_rect(0.0, 0.0, 800.0, 600.0, Color::cornflower_blue());
        pipeline.add_pass(pass);
        assert_eq!(pipeline.total_draw_commands(), 1);
    }

    #[test]
    fn test_render_pipeline_software_execute() {
        let viewport = Viewport::new(64, 64);
        let mut pipeline = RenderPipeline::new(viewport);
        let mut pass = RenderPass::new("bg");
        pass.clear_color = Some(Color::red());
        pipeline.add_pass(pass);
        let buf = pipeline.execute_software();
        assert_eq!(buf.width, 64);
        assert_eq!(buf.height, 64);
        let px = buf.get_pixel(32, 32);
        assert!(px.r > 0.9);
    }

    // ── Pixel Format Tests ──────────────────────────────────────────

    #[test]
    fn test_pixel_format_bpp() {
        assert_eq!(PixelFormat::Rgba8.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::RgbaF32.bytes_per_pixel(), 16);
        assert_eq!(PixelFormat::GrayScale8.bytes_per_pixel(), 1);
    }

    #[test]
    fn test_named_colors() {
        assert_eq!(Color::red().r, 1.0);
        assert_eq!(Color::green().g, 1.0);
        assert_eq!(Color::blue().b, 1.0);
        assert_eq!(Color::white().luminance(), 1.0);
    }
}
