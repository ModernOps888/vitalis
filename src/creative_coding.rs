//! Vitalis Creative Coding — Processing/p5.js-style generative art and visual programming.
//!
//! Provides a complete creative coding environment inspired by:
//!
//! ## Creative Coding Languages
//! - **Processing** (Java-based): Sketch-based visual programming with setup/draw loop
//! - **p5.js** (JavaScript): Browser-based creative coding on HTML5 canvas
//!
//! ## Features
//! - Sketch lifecycle: `setup()` → `draw()` loop with frame timing
//! - Drawing primitives: shapes, paths, text, images
//! - Color modes: RGB, HSB, HSL  
//! - Transform stack: translate, rotate, scale, push/pop matrix
//! - Math utilities: noise (Perlin), random, map, constrain, lerp
//! - Particle systems with physics simulation
//! - L-system fractal generation
//! - Cellular automata (Game of Life, Wolfram rules)
//! - Attractor systems (Lorenz, strange attractors)
//! - Flow field visualization
//! - Interactive input: mouse position, keyboard state
//! - Export to SVG path data

use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  SKETCH FRAMEWORK
// ═══════════════════════════════════════════════════════════════════════

/// Color mode for the sketch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Rgb,
    Hsb,
    Hsl,
}

/// Sketch configuration and state.
#[derive(Debug, Clone)]
pub struct Sketch {
    pub width: u32,
    pub height: u32,
    pub frame_count: u64,
    pub frame_rate: f64,
    pub color_mode: ColorMode,
    pub background_color: [f64; 4],
    pub fill_color: [f64; 4],
    pub stroke_color: [f64; 4],
    pub stroke_weight: f64,
    pub fill_enabled: bool,
    pub stroke_enabled: bool,
    pub canvas: Vec<DrawPrimitive>,
    pub transform_stack: Vec<Transform2D>,
    pub current_transform: Transform2D,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub mouse_pressed: bool,
    pub key_pressed: Option<char>,
    rng_state: u64,
    noise_perm: [u8; 512],
}

/// A 2D affine transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub a: f64, pub b: f64, pub c: f64,
    pub d: f64, pub e: f64, pub f: f64,
}

impl Transform2D {
    pub fn identity() -> Self { Self { a: 1.0, b: 0.0, c: 0.0, d: 0.0, e: 1.0, f: 0.0 } }

    pub fn translation(tx: f64, ty: f64) -> Self { Self { a: 1.0, b: 0.0, c: tx, d: 0.0, e: 1.0, f: ty } }

    pub fn rotation(angle: f64) -> Self {
        let cos = angle.cos(); let sin = angle.sin();
        Self { a: cos, b: -sin, c: 0.0, d: sin, e: cos, f: 0.0 }
    }

    pub fn scaling(sx: f64, sy: f64) -> Self { Self { a: sx, b: 0.0, c: 0.0, d: 0.0, e: sy, f: 0.0 } }

    pub fn multiply(&self, other: &Transform2D) -> Transform2D {
        Transform2D {
            a: self.a * other.a + self.b * other.d,
            b: self.a * other.b + self.b * other.e,
            c: self.a * other.c + self.b * other.f + self.c,
            d: self.d * other.a + self.e * other.d,
            e: self.d * other.b + self.e * other.e,
            f: self.d * other.c + self.e * other.f + self.f,
        }
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.b * y + self.c, self.d * x + self.e * y + self.f)
    }
}

/// Drawing primitives accumulated during a frame.
#[derive(Debug, Clone)]
pub enum DrawPrimitive {
    Point { x: f64, y: f64, color: [f64; 4] },
    Line { x1: f64, y1: f64, x2: f64, y2: f64, color: [f64; 4], weight: f64 },
    Rect { x: f64, y: f64, w: f64, h: f64, fill: Option<[f64; 4]>, stroke: Option<([f64; 4], f64)> },
    Ellipse { cx: f64, cy: f64, rx: f64, ry: f64, fill: Option<[f64; 4]>, stroke: Option<([f64; 4], f64)> },
    Triangle { x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, fill: Option<[f64; 4]>, stroke: Option<([f64; 4], f64)> },
    Quad { x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64, fill: Option<[f64; 4]>, stroke: Option<([f64; 4], f64)> },
    Arc { cx: f64, cy: f64, rx: f64, ry: f64, start: f64, stop: f64, fill: Option<[f64; 4]> },
    BezierCurve { x1: f64, y1: f64, cx1: f64, cy1: f64, cx2: f64, cy2: f64, x2: f64, y2: f64, color: [f64; 4], weight: f64 },
    Text { text: String, x: f64, y: f64, size: f64, color: [f64; 4] },
    Polygon { vertices: Vec<(f64, f64)>, fill: Option<[f64; 4]>, stroke: Option<([f64; 4], f64)> },
}

impl Sketch {
    pub fn new(width: u32, height: u32) -> Self {
        let mut perm = [0u8; 512];
        for i in 0..256 { perm[i] = i as u8; }
        // Simple shuffle using LCG
        let mut seed: u64 = 42;
        for i in (1..256).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (seed >> 33) as usize % (i + 1);
            perm.swap(i, j);
        }
        for i in 0..256 { perm[i + 256] = perm[i]; }

        Self {
            width, height,
            frame_count: 0, frame_rate: 60.0,
            color_mode: ColorMode::Rgb,
            background_color: [1.0, 1.0, 1.0, 1.0],
            fill_color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 1.0],
            stroke_weight: 1.0,
            fill_enabled: true,
            stroke_enabled: true,
            canvas: Vec::new(),
            transform_stack: Vec::new(),
            current_transform: Transform2D::identity(),
            mouse_x: 0.0, mouse_y: 0.0,
            mouse_pressed: false,
            key_pressed: None,
            rng_state: 42,
            noise_perm: perm,
        }
    }

    // ── Drawing State ────────────────────────────────────────────────

    pub fn background(&mut self, r: f64, g: f64, b: f64) {
        self.background_color = [r, g, b, 1.0];
        self.canvas.clear();
    }

    pub fn fill(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.fill_color = [r, g, b, a];
        self.fill_enabled = true;
    }

    pub fn no_fill(&mut self) { self.fill_enabled = false; }

    pub fn stroke(&mut self, r: f64, g: f64, b: f64, a: f64) {
        self.stroke_color = [r, g, b, a];
        self.stroke_enabled = true;
    }

    pub fn no_stroke(&mut self) { self.stroke_enabled = false; }

    pub fn set_stroke_weight(&mut self, weight: f64) { self.stroke_weight = weight; }

    // ── Transform Stack ──────────────────────────────────────────────

    pub fn push_matrix(&mut self) { self.transform_stack.push(self.current_transform); }
    pub fn pop_matrix(&mut self) {
        if let Some(t) = self.transform_stack.pop() { self.current_transform = t; }
    }
    pub fn translate(&mut self, tx: f64, ty: f64) {
        self.current_transform = self.current_transform.multiply(&Transform2D::translation(tx, ty));
    }
    pub fn rotate(&mut self, angle: f64) {
        self.current_transform = self.current_transform.multiply(&Transform2D::rotation(angle));
    }
    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.current_transform = self.current_transform.multiply(&Transform2D::scaling(sx, sy));
    }
    pub fn reset_matrix(&mut self) { self.current_transform = Transform2D::identity(); }

    // ── Drawing Primitives ───────────────────────────────────────────

    pub fn point(&mut self, x: f64, y: f64) {
        let (x, y) = self.current_transform.apply(x, y);
        self.canvas.push(DrawPrimitive::Point { x, y, color: self.stroke_color });
    }

    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        let (x1, y1) = self.current_transform.apply(x1, y1);
        let (x2, y2) = self.current_transform.apply(x2, y2);
        self.canvas.push(DrawPrimitive::Line { x1, y1, x2, y2, color: self.stroke_color, weight: self.stroke_weight });
    }

    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let (x, y) = self.current_transform.apply(x, y);
        self.canvas.push(DrawPrimitive::Rect {
            x, y, w, h,
            fill: if self.fill_enabled { Some(self.fill_color) } else { None },
            stroke: if self.stroke_enabled { Some((self.stroke_color, self.stroke_weight)) } else { None },
        });
    }

    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        let (cx, cy) = self.current_transform.apply(cx, cy);
        self.canvas.push(DrawPrimitive::Ellipse {
            cx, cy, rx, ry,
            fill: if self.fill_enabled { Some(self.fill_color) } else { None },
            stroke: if self.stroke_enabled { Some((self.stroke_color, self.stroke_weight)) } else { None },
        });
    }

    pub fn circle(&mut self, cx: f64, cy: f64, r: f64) { self.ellipse(cx, cy, r, r); }

    pub fn triangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        let (x1, y1) = self.current_transform.apply(x1, y1);
        let (x2, y2) = self.current_transform.apply(x2, y2);
        let (x3, y3) = self.current_transform.apply(x3, y3);
        self.canvas.push(DrawPrimitive::Triangle {
            x1, y1, x2, y2, x3, y3,
            fill: if self.fill_enabled { Some(self.fill_color) } else { None },
            stroke: if self.stroke_enabled { Some((self.stroke_color, self.stroke_weight)) } else { None },
        });
    }

    pub fn text(&mut self, text: &str, x: f64, y: f64, size: f64) {
        let (x, y) = self.current_transform.apply(x, y);
        self.canvas.push(DrawPrimitive::Text { text: text.into(), x, y, size, color: self.fill_color });
    }

    pub fn polygon(&mut self, vertices: &[(f64, f64)]) {
        let transformed: Vec<(f64, f64)> = vertices.iter().map(|&(x, y)| self.current_transform.apply(x, y)).collect();
        self.canvas.push(DrawPrimitive::Polygon {
            vertices: transformed,
            fill: if self.fill_enabled { Some(self.fill_color) } else { None },
            stroke: if self.stroke_enabled { Some((self.stroke_color, self.stroke_weight)) } else { None },
        });
    }

    pub fn bezier(&mut self, x1: f64, y1: f64, cx1: f64, cy1: f64, cx2: f64, cy2: f64, x2: f64, y2: f64) {
        self.canvas.push(DrawPrimitive::BezierCurve {
            x1, y1, cx1, cy1, cx2, cy2, x2, y2,
            color: self.stroke_color, weight: self.stroke_weight,
        });
    }

    // ── Frame Management ─────────────────────────────────────────────

    pub fn begin_frame(&mut self) {
        self.canvas.clear();
        self.frame_count += 1;
    }

    pub fn end_frame(&self) -> &[DrawPrimitive] { &self.canvas }

    pub fn primitive_count(&self) -> usize { self.canvas.len() }

    // ── Math Utilities ───────────────────────────────────────────────

    /// Map a value from one range to another.
    pub fn map_value(value: f64, start1: f64, stop1: f64, start2: f64, stop2: f64) -> f64 {
        start2 + (stop2 - start2) * ((value - start1) / (stop1 - start1))
    }

    /// Constrain a value to a range.
    pub fn constrain(value: f64, low: f64, high: f64) -> f64 { value.clamp(low, high) }

    /// Linear interpolation.
    pub fn lerp_value(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }

    /// Distance between two points.
    pub fn dist(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
    }

    /// Pseudo-random number [0, 1).
    pub fn random(&mut self) -> f64 {
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng_state >> 33) as f64) / (u32::MAX as f64)
    }

    /// Random float in [low, high).
    pub fn random_range(&mut self, low: f64, high: f64) -> f64 {
        low + self.random() * (high - low)
    }

    /// 1D Perlin noise.
    pub fn noise(&self, x: f64) -> f64 {
        let xi = x.floor() as i32;
        let xf = x - x.floor();
        let u = fade(xf);
        let a = self.noise_perm[(xi & 255) as usize] as f64 / 255.0;
        let b = self.noise_perm[((xi + 1) & 255) as usize] as f64 / 255.0;
        a + u * (b - a)
    }

    /// 2D Perlin noise.
    pub fn noise2d(&self, x: f64, y: f64) -> f64 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - x.floor();
        let yf = y - y.floor();
        let u = fade(xf);
        let v = fade(yf);
        let aa = self.noise_perm[(self.noise_perm[(xi & 255) as usize] as i32 + (yi & 255)) as usize & 511] as f64 / 255.0;
        let ab = self.noise_perm[(self.noise_perm[(xi & 255) as usize] as i32 + ((yi + 1) & 255)) as usize & 511] as f64 / 255.0;
        let ba = self.noise_perm[(self.noise_perm[((xi + 1) & 255) as usize] as i32 + (yi & 255)) as usize & 511] as f64 / 255.0;
        let bb = self.noise_perm[(self.noise_perm[((xi + 1) & 255) as usize] as i32 + ((yi + 1) & 255)) as usize & 511] as f64 / 255.0;
        let x1 = Self::lerp_value(aa, ba, u);
        let x2 = Self::lerp_value(ab, bb, u);
        Self::lerp_value(x1, x2, v)
    }

    /// Export canvas to SVG path data.
    pub fn to_svg(&self) -> String {
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
            self.width, self.height, self.width, self.height
        );
        svg.push_str(&format!(
            "  <rect width=\"{}\" height=\"{}\" fill=\"rgb({},{},{})\" />\n",
            self.width, self.height,
            (self.background_color[0] * 255.0) as u8,
            (self.background_color[1] * 255.0) as u8,
            (self.background_color[2] * 255.0) as u8
        ));
        for prim in &self.canvas {
            match prim {
                DrawPrimitive::Line { x1, y1, x2, y2, color, weight } => {
                    svg.push_str(&format!(
                        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"rgb({},{},{})\" stroke-width=\"{}\" />\n",
                        x1, y1, x2, y2, (color[0]*255.0) as u8, (color[1]*255.0) as u8, (color[2]*255.0) as u8, weight
                    ));
                }
                DrawPrimitive::Rect { x, y, w, h, fill, .. } => {
                    let fill_str = fill.map(|c| format!("rgb({},{},{})", (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8))
                                       .unwrap_or_else(|| "none".into());
                    svg.push_str(&format!("  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" />\n", x, y, w, h, fill_str));
                }
                DrawPrimitive::Ellipse { cx, cy, rx, ry, fill, .. } => {
                    let fill_str = fill.map(|c| format!("rgb({},{},{})", (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8))
                                       .unwrap_or_else(|| "none".into());
                    svg.push_str(&format!("  <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" />\n", cx, cy, rx, ry, fill_str));
                }
                DrawPrimitive::Text { text, x, y, size, color } => {
                    svg.push_str(&format!(
                        "  <text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"rgb({},{},{})\">{}</text>\n",
                        x, y, size, (color[0]*255.0) as u8, (color[1]*255.0) as u8, (color[2]*255.0) as u8, text
                    ));
                }
                _ => {}
            }
        }
        svg.push_str("</svg>");
        svg
    }
}

fn fade(t: f64) -> f64 { t * t * t * (t * (t * 6.0 - 15.0) + 10.0) }

// ═══════════════════════════════════════════════════════════════════════
//  PARTICLE SYSTEM
// ═══════════════════════════════════════════════════════════════════════

/// A single particle.
#[derive(Debug, Clone)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub ax: f64,
    pub ay: f64,
    pub mass: f64,
    pub life: f64,
    pub max_life: f64,
    pub color: [f64; 4],
    pub size: f64,
}

impl Particle {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x, y, vx: 0.0, vy: 0.0, ax: 0.0, ay: 0.0,
            mass: 1.0, life: 1.0, max_life: 1.0,
            color: [1.0, 1.0, 1.0, 1.0], size: 4.0,
        }
    }

    pub fn update(&mut self, dt: f64) {
        self.vx += self.ax * dt;
        self.vy += self.ay * dt;
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.life -= dt / self.max_life;
        self.ax = 0.0;
        self.ay = 0.0;
    }

    pub fn apply_force(&mut self, fx: f64, fy: f64) {
        self.ax += fx / self.mass;
        self.ay += fy / self.mass;
    }

    pub fn is_alive(&self) -> bool { self.life > 0.0 }
    pub fn age_ratio(&self) -> f64 { 1.0 - self.life }
}

/// Particle emission shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitterShape { Point, Circle, Rectangle, Line }

/// Particle system emitter.
#[derive(Debug, Clone)]
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub emitter_x: f64,
    pub emitter_y: f64,
    pub emitter_shape: EmitterShape,
    pub emission_rate: f64,
    pub max_particles: usize,
    pub gravity: f64,
    pub wind: f64,
    pub particle_life: f64,
    pub particle_size: f64,
    pub spread: f64,
    pub speed: f64,
    emit_accumulator: f64,
}

impl ParticleSystem {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            particles: Vec::new(),
            emitter_x: x, emitter_y: y,
            emitter_shape: EmitterShape::Point,
            emission_rate: 10.0,
            max_particles: 1000,
            gravity: 98.0,
            wind: 0.0,
            particle_life: 2.0,
            particle_size: 4.0,
            spread: std::f64::consts::PI / 4.0,
            speed: 100.0,
            emit_accumulator: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64) {
        // Emit new particles
        self.emit_accumulator += self.emission_rate * dt;
        while self.emit_accumulator >= 1.0 && self.particles.len() < self.max_particles {
            self.emit_accumulator -= 1.0;
            let mut p = Particle::new(self.emitter_x, self.emitter_y);
            p.max_life = self.particle_life;
            p.life = 1.0;
            p.size = self.particle_size;
            let angle = -std::f64::consts::FRAC_PI_2 + (self.emit_accumulator - 0.5) * self.spread;
            p.vx = self.speed * angle.cos();
            p.vy = self.speed * angle.sin();
            self.particles.push(p);
        }

        // Update physics
        for p in &mut self.particles {
            p.apply_force(self.wind, self.gravity * p.mass);
            p.update(dt);
        }

        // Remove dead particles
        self.particles.retain(|p| p.is_alive());
    }

    pub fn alive_count(&self) -> usize { self.particles.len() }

    pub fn draw(&self, sketch: &mut Sketch) {
        for p in &self.particles {
            let alpha = p.life.clamp(0.0, 1.0);
            sketch.fill(p.color[0], p.color[1], p.color[2], alpha);
            sketch.no_stroke();
            sketch.circle(p.x, p.y, p.size * p.life);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  L-SYSTEM (Fractal Generation)
// ═══════════════════════════════════════════════════════════════════════

/// An L-System rule.
#[derive(Debug, Clone)]
pub struct LSystemRule {
    pub symbol: char,
    pub replacement: String,
}

/// Lindenmayer system for fractal generation.
#[derive(Debug, Clone)]
pub struct LSystem {
    pub axiom: String,
    pub rules: Vec<LSystemRule>,
    pub angle: f64,
    pub length: f64,
    pub generations: usize,
}

impl LSystem {
    pub fn new(axiom: &str, angle: f64, length: f64) -> Self {
        Self { axiom: axiom.into(), rules: Vec::new(), angle, length, generations: 0 }
    }

    pub fn add_rule(&mut self, symbol: char, replacement: &str) {
        self.rules.push(LSystemRule { symbol, replacement: replacement.into() });
    }

    /// Generate the string after n iterations.
    pub fn generate(&self, iterations: usize) -> String {
        let mut current = self.axiom.clone();
        for _ in 0..iterations {
            let mut next = String::new();
            for ch in current.chars() {
                let mut found = false;
                for rule in &self.rules {
                    if rule.symbol == ch {
                        next.push_str(&rule.replacement);
                        found = true;
                        break;
                    }
                }
                if !found { next.push(ch); }
            }
            current = next;
        }
        current
    }

    /// Compute turtle graphics points from L-system string.
    pub fn to_points(&self, iterations: usize, start_x: f64, start_y: f64) -> Vec<(f64, f64)> {
        let instructions = self.generate(iterations);
        let mut x = start_x;
        let mut y = start_y;
        let mut heading = -std::f64::consts::FRAC_PI_2;
        let mut stack: Vec<(f64, f64, f64)> = Vec::new();
        let mut points = vec![(x, y)];
        let length = self.length / (2.0_f64).powi(iterations as i32);

        for ch in instructions.chars() {
            match ch {
                'F' | 'G' => {
                    x += length * heading.cos();
                    y += length * heading.sin();
                    points.push((x, y));
                }
                '+' => heading += self.angle,
                '-' => heading -= self.angle,
                '[' => stack.push((x, y, heading)),
                ']' => {
                    if let Some((sx, sy, sh)) = stack.pop() {
                        x = sx; y = sy; heading = sh;
                        points.push((x, y));
                    }
                }
                _ => {}
            }
        }
        points
    }

    /// Pre-built: Koch snowflake.
    pub fn koch_snowflake() -> Self {
        let mut ls = LSystem::new("F--F--F", std::f64::consts::PI / 3.0, 100.0);
        ls.add_rule('F', "F+F--F+F");
        ls
    }

    /// Pre-built: Sierpinski triangle.
    pub fn sierpinski() -> Self {
        let mut ls = LSystem::new("F-G-G", 2.0 * std::f64::consts::PI / 3.0, 100.0);
        ls.add_rule('F', "F-G+F+G-F");
        ls.add_rule('G', "GG");
        ls
    }

    /// Pre-built: Fractal plant.
    pub fn fractal_plant() -> Self {
        let mut ls = LSystem::new("X", 25.0_f64.to_radians(), 100.0);
        ls.add_rule('X', "F+[[X]-X]-F[-FX]+X");
        ls.add_rule('F', "FF");
        ls
    }

    /// Pre-built: Dragon curve.
    pub fn dragon_curve() -> Self {
        let mut ls = LSystem::new("FX", std::f64::consts::FRAC_PI_2, 100.0);
        ls.add_rule('X', "X+YF+");
        ls.add_rule('Y', "-FX-Y");
        ls
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CELLULAR AUTOMATA
// ═══════════════════════════════════════════════════════════════════════

/// Conway's Game of Life.
#[derive(Debug, Clone)]
pub struct GameOfLife {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<bool>,
    pub generation_count: u64,
}

impl GameOfLife {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, cells: vec![false; width * height], generation_count: 0 }
    }

    pub fn set_cell(&mut self, x: usize, y: usize, alive: bool) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = alive;
        }
    }

    pub fn get_cell(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height { self.cells[y * self.width + x] } else { false }
    }

    fn count_neighbors(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        for dy in [-1i32, 0, 1] {
            for dx in [-1i32, 0, 1] {
                if dx == 0 && dy == 0 { continue; }
                let nx = (x as i32 + dx).rem_euclid(self.width as i32) as usize;
                let ny = (y as i32 + dy).rem_euclid(self.height as i32) as usize;
                if self.cells[ny * self.width + nx] { count += 1; }
            }
        }
        count
    }

    pub fn step(&mut self) {
        let mut next = vec![false; self.width * self.height];
        for y in 0..self.height {
            for x in 0..self.width {
                let neighbors = self.count_neighbors(x, y);
                let alive = self.cells[y * self.width + x];
                next[y * self.width + x] = matches!((alive, neighbors), (true, 2) | (true, 3) | (false, 3));
            }
        }
        self.cells = next;
        self.generation_count += 1;
    }

    pub fn alive_count(&self) -> usize { self.cells.iter().filter(|&&c| c).count() }

    /// Seed with a glider at position.
    pub fn add_glider(&mut self, x: usize, y: usize) {
        let pattern = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
        for (dx, dy) in pattern { self.set_cell(x + dx, y + dy, true); }
    }

    /// Seed random cells.
    pub fn randomize(&mut self, density: f64) {
        let mut rng: u64 = 12345;
        for cell in &mut self.cells {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let val = ((rng >> 33) as f64) / (u32::MAX as f64);
            *cell = val < density;
        }
    }
}

/// 1D elementary cellular automaton (Wolfram rules).
#[derive(Debug, Clone)]
pub struct ElementaryCA {
    pub width: usize,
    pub rule: u8,
    pub cells: Vec<bool>,
    pub history: Vec<Vec<bool>>,
}

impl ElementaryCA {
    pub fn new(width: usize, rule: u8) -> Self {
        let mut cells = vec![false; width];
        cells[width / 2] = true; // Single cell in center
        Self { width, rule, cells: cells.clone(), history: vec![cells] }
    }

    pub fn step(&mut self) {
        let mut next = vec![false; self.width];
        for i in 0..self.width {
            let left = if i > 0 { self.cells[i - 1] } else { false };
            let center = self.cells[i];
            let right = if i < self.width - 1 { self.cells[i + 1] } else { false };
            let index = (left as u8) << 2 | (center as u8) << 1 | (right as u8);
            next[i] = (self.rule >> index) & 1 == 1;
        }
        self.cells = next.clone();
        self.history.push(next);
    }

    pub fn run(&mut self, steps: usize) {
        for _ in 0..steps { self.step(); }
    }

    pub fn generation_count(&self) -> usize { self.history.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  ATTRACTORS
// ═══════════════════════════════════════════════════════════════════════

/// Lorenz attractor state.
#[derive(Debug, Clone)]
pub struct LorenzAttractor {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub sigma: f64,
    pub rho: f64,
    pub beta: f64,
    pub trail: Vec<(f64, f64, f64)>,
    pub max_trail: usize,
}

impl LorenzAttractor {
    pub fn new() -> Self {
        Self {
            x: 0.1, y: 0.0, z: 0.0,
            sigma: 10.0, rho: 28.0, beta: 8.0 / 3.0,
            trail: Vec::new(), max_trail: 5000,
        }
    }

    pub fn step(&mut self, dt: f64) {
        let dx = self.sigma * (self.y - self.x);
        let dy = self.x * (self.rho - self.z) - self.y;
        let dz = self.x * self.y - self.beta * self.z;
        self.x += dx * dt;
        self.y += dy * dt;
        self.z += dz * dt;
        self.trail.push((self.x, self.y, self.z));
        if self.trail.len() > self.max_trail { self.trail.remove(0); }
    }

    pub fn trail_length(&self) -> usize { self.trail.len() }
}

/// Flow field for visualization.
#[derive(Debug, Clone)]
pub struct FlowField {
    pub width: usize,
    pub height: usize,
    pub resolution: f64,
    pub angles: Vec<f64>,
}

impl FlowField {
    pub fn new(width: usize, height: usize, resolution: f64) -> Self {
        let cols = (width as f64 / resolution).ceil() as usize;
        let rows = (height as f64 / resolution).ceil() as usize;
        Self {
            width, height, resolution,
            angles: vec![0.0; cols * rows],
        }
    }

    /// Generate flow field from Perlin noise.
    pub fn generate_from_noise(&mut self, sketch: &Sketch, z_offset: f64) {
        let cols = (self.width as f64 / self.resolution).ceil() as usize;
        let rows = (self.height as f64 / self.resolution).ceil() as usize;
        for y in 0..rows {
            for x in 0..cols {
                let noise_val = sketch.noise2d(x as f64 * 0.1 + z_offset, y as f64 * 0.1);
                if y * cols + x < self.angles.len() {
                    self.angles[y * cols + x] = noise_val * std::f64::consts::TAU;
                }
            }
        }
    }

    /// Get angle at a continuous position.
    pub fn angle_at(&self, x: f64, y: f64) -> f64 {
        let cols = (self.width as f64 / self.resolution).ceil() as usize;
        let col = ((x / self.resolution).floor() as usize).min(cols.saturating_sub(1));
        let row = ((y / self.resolution).floor() as usize).min(self.angles.len() / cols.max(1));
        let idx = row * cols + col;
        if idx < self.angles.len() { self.angles[idx] } else { 0.0 }
    }

    pub fn cell_count(&self) -> usize { self.angles.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sketch Tests ────────────────────────────────────────────────

    #[test]
    fn test_sketch_creation() {
        let s = Sketch::new(800, 600);
        assert_eq!(s.width, 800);
        assert_eq!(s.height, 600);
        assert_eq!(s.frame_count, 0);
    }

    #[test]
    fn test_sketch_drawing() {
        let mut s = Sketch::new(400, 400);
        s.background(0.0, 0.0, 0.0);
        s.fill(1.0, 0.0, 0.0, 1.0);
        s.rect(10.0, 10.0, 50.0, 50.0);
        s.circle(200.0, 200.0, 30.0);
        s.line(0.0, 0.0, 400.0, 400.0);
        assert_eq!(s.primitive_count(), 3);
    }

    #[test]
    fn test_sketch_transform() {
        let mut s = Sketch::new(400, 400);
        s.push_matrix();
        s.translate(100.0, 100.0);
        s.point(0.0, 0.0);
        s.pop_matrix();
        assert_eq!(s.primitive_count(), 1);
        if let DrawPrimitive::Point { x, y, .. } = &s.canvas[0] {
            assert!((x - 100.0).abs() < 1e-6);
            assert!((y - 100.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_sketch_map_value() {
        assert!((Sketch::map_value(5.0, 0.0, 10.0, 0.0, 100.0) - 50.0).abs() < 1e-6);
        assert!((Sketch::map_value(0.0, 0.0, 10.0, 100.0, 200.0) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_sketch_constrain() {
        assert_eq!(Sketch::constrain(150.0, 0.0, 100.0), 100.0);
        assert_eq!(Sketch::constrain(-10.0, 0.0, 100.0), 0.0);
        assert_eq!(Sketch::constrain(50.0, 0.0, 100.0), 50.0);
    }

    #[test]
    fn test_sketch_random() {
        let mut s = Sketch::new(100, 100);
        let r = s.random();
        assert!(r >= 0.0 && r < 1.0);
        let r2 = s.random_range(10.0, 20.0);
        assert!(r2 >= 10.0 && r2 < 20.0);
    }

    #[test]
    fn test_sketch_noise() {
        let s = Sketch::new(100, 100);
        let n1 = s.noise(0.0);
        let n2 = s.noise(0.5);
        assert!(n1 >= 0.0 && n1 <= 1.0);
        assert!(n2 >= 0.0 && n2 <= 1.0);
    }

    #[test]
    fn test_sketch_noise2d() {
        let s = Sketch::new(100, 100);
        let n = s.noise2d(1.5, 2.5);
        assert!(n >= 0.0 && n <= 1.0);
    }

    #[test]
    fn test_sketch_svg_export() {
        let mut s = Sketch::new(200, 200);
        s.fill(1.0, 0.0, 0.0, 1.0);
        s.rect(10.0, 10.0, 50.0, 30.0);
        let svg = s.to_svg();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_sketch_polygon() {
        let mut s = Sketch::new(200, 200);
        s.polygon(&[(50.0, 10.0), (90.0, 90.0), (10.0, 90.0)]);
        assert_eq!(s.primitive_count(), 1);
    }

    // ── Transform Tests ─────────────────────────────────────────────

    #[test]
    fn test_transform_identity() {
        let t = Transform2D::identity();
        let (x, y) = t.apply(5.0, 10.0);
        assert!((x - 5.0).abs() < 1e-10);
        assert!((y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_translation() {
        let t = Transform2D::translation(10.0, 20.0);
        let (x, y) = t.apply(5.0, 5.0);
        assert!((x - 15.0).abs() < 1e-10);
        assert!((y - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_transform_rotation() {
        let t = Transform2D::rotation(std::f64::consts::FRAC_PI_2);
        let (x, y) = t.apply(1.0, 0.0);
        assert!(x.abs() < 1e-10);
        assert!((y - 1.0).abs() < 1e-10);
    }

    // ── Particle System Tests ───────────────────────────────────────

    #[test]
    fn test_particle() {
        let mut p = Particle::new(100.0, 100.0);
        p.vx = 10.0;
        p.vy = -5.0;
        p.update(0.1);
        assert!(p.x > 100.0);
        assert!(p.y < 100.0);
        assert!(p.is_alive());
    }

    #[test]
    fn test_particle_system() {
        let mut ps = ParticleSystem::new(200.0, 200.0);
        ps.emission_rate = 100.0;
        ps.update(0.5);
        assert!(ps.alive_count() > 0);
    }

    // ── L-System Tests ──────────────────────────────────────────────

    #[test]
    fn test_lsystem_generate() {
        let mut ls = LSystem::new("A", 0.0, 10.0);
        ls.add_rule('A', "AB");
        ls.add_rule('B', "A");
        let g0 = ls.generate(0);
        assert_eq!(g0, "A");
        let g1 = ls.generate(1);
        assert_eq!(g1, "AB");
        let g2 = ls.generate(2);
        assert_eq!(g2, "ABA");
    }

    #[test]
    fn test_lsystem_koch() {
        let ls = LSystem::koch_snowflake();
        let gen1 = ls.generate(1);
        assert!(gen1.contains("F+F--F+F"));
    }

    #[test]
    fn test_lsystem_points() {
        let ls = LSystem::koch_snowflake();
        let points = ls.to_points(1, 0.0, 0.0);
        assert!(points.len() > 1);
    }

    #[test]
    fn test_lsystem_dragon() {
        let ls = LSystem::dragon_curve();
        let gen2 = ls.generate(2);
        assert!(gen2.len() > 2);
    }

    #[test]
    fn test_lsystem_plant() {
        let ls = LSystem::fractal_plant();
        let gen1 = ls.generate(1);
        assert!(gen1.contains('['));
        assert!(gen1.contains(']'));
    }

    // ── Game of Life Tests ──────────────────────────────────────────

    #[test]
    fn test_game_of_life_creation() {
        let gol = GameOfLife::new(50, 50);
        assert_eq!(gol.alive_count(), 0);
    }

    #[test]
    fn test_game_of_life_glider() {
        let mut gol = GameOfLife::new(20, 20);
        gol.add_glider(5, 5);
        let initial_alive = gol.alive_count();
        assert_eq!(initial_alive, 5);
        gol.step();
        // Glider should still be 5 cells after one step
        assert_eq!(gol.alive_count(), 5);
    }

    #[test]
    fn test_game_of_life_blinker() {
        let mut gol = GameOfLife::new(10, 10);
        // Blinker pattern
        gol.set_cell(4, 5, true);
        gol.set_cell(5, 5, true);
        gol.set_cell(6, 5, true);
        gol.step();
        assert!(gol.get_cell(5, 4));
        assert!(gol.get_cell(5, 5));
        assert!(gol.get_cell(5, 6));
    }

    #[test]
    fn test_game_of_life_randomize() {
        let mut gol = GameOfLife::new(50, 50);
        gol.randomize(0.3);
        let alive = gol.alive_count();
        assert!(alive > 0 && alive < 2500);
    }

    // ── Elementary CA Tests ─────────────────────────────────────────

    #[test]
    fn test_elementary_ca_rule30() {
        let mut ca = ElementaryCA::new(21, 30);
        ca.step();
        assert_eq!(ca.generation_count(), 2);
    }

    #[test]
    fn test_elementary_ca_rule110() {
        let mut ca = ElementaryCA::new(31, 110);
        ca.run(10);
        assert_eq!(ca.generation_count(), 11); // Initial + 10 steps
    }

    // ── Lorenz Attractor Tests ──────────────────────────────────────

    #[test]
    fn test_lorenz_attractor() {
        let mut lorenz = LorenzAttractor::new();
        for _ in 0..100 {
            lorenz.step(0.01);
        }
        assert_eq!(lorenz.trail_length(), 100);
        assert!(lorenz.x.abs() > 1e-6 || lorenz.y.abs() > 1e-6);
    }

    // ── Flow Field Tests ────────────────────────────────────────────

    #[test]
    fn test_flow_field() {
        let ff = FlowField::new(400, 400, 20.0);
        assert!(ff.cell_count() > 0);
        let angle = ff.angle_at(100.0, 100.0);
        assert!(!angle.is_nan());
    }

    #[test]
    fn test_flow_field_noise_generation() {
        let sketch = Sketch::new(200, 200);
        let mut ff = FlowField::new(200, 200, 20.0);
        ff.generate_from_noise(&sketch, 0.0);
        let a1 = ff.angle_at(10.0, 10.0);
        let a2 = ff.angle_at(50.0, 50.0);
        // Different positions should generally give different angles
        assert!(!a1.is_nan());
        assert!(!a2.is_nan());
    }

    // ── Distance Test ───────────────────────────────────────────────

    #[test]
    fn test_dist() {
        assert!((Sketch::dist(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < 1e-10);
    }
}
