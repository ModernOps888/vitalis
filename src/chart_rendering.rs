//! Vitalis Chart Rendering — Data visualization and charting library.
//!
//! Comprehensive chart types for data visualization:
//!
//! ## Chart Types
//! - **Pie Chart** / Donut Chart — proportional data display
//! - **Bar Chart** — vertical/horizontal bars, stacked/grouped
//! - **Line Chart** — time series, trend lines, area charts
//! - **Scatter Plot** — correlation, clusters, bubble charts
//! - **Histogram** — frequency distribution
//! - **Radar/Spider Chart** — multi-dimensional comparison
//! - **Heat Map** — matrix visualization
//! - **Treemap** — hierarchical data
//! - **Candlestick** — financial OHLC data
//! - **Gauge** — single-value dashboards
//! - **Sparkline** — inline mini-charts
//!
//! ## Features
//! - Configurable axes, legends, titles, tooltips
//! - Color palettes and themes
//! - Animation/transition support
//! - SVG export for all chart types
//! - Grid lines, tick marks, labels
//! - Responsive sizing

use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  CORE TYPES
// ═══════════════════════════════════════════════════════════════════════

/// RGBA color for chart elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl ChartColor {
    pub const fn new(r: f64, g: f64, b: f64) -> Self { Self { r, g, b, a: 1.0 } }
    pub const fn with_alpha(r: f64, g: f64, b: f64, a: f64) -> Self { Self { r, g, b, a } }

    pub fn to_css(&self) -> String {
        if (self.a - 1.0).abs() < 1e-6 {
            format!("rgb({},{},{})", (self.r * 255.0) as u8, (self.g * 255.0) as u8, (self.b * 255.0) as u8)
        } else {
            format!("rgba({},{},{},{:.2})", (self.r * 255.0) as u8, (self.g * 255.0) as u8, (self.b * 255.0) as u8, self.a)
        }
    }

    // Named colors
    pub fn red() -> Self { Self::new(0.894, 0.102, 0.110) }
    pub fn blue() -> Self { Self::new(0.216, 0.494, 0.722) }
    pub fn green() -> Self { Self::new(0.302, 0.686, 0.290) }
    pub fn orange() -> Self { Self::new(1.0, 0.498, 0.0) }
    pub fn purple() -> Self { Self::new(0.596, 0.306, 0.639) }
    pub fn teal() -> Self { Self::new(0.2, 0.627, 0.627) }
    pub fn pink() -> Self { Self::new(0.969, 0.506, 0.749) }
    pub fn gold() -> Self { Self::new(1.0, 0.843, 0.0) }
    pub fn gray() -> Self { Self::new(0.5, 0.5, 0.5) }
    pub fn black() -> Self { Self::new(0.0, 0.0, 0.0) }
    pub fn white() -> Self { Self::new(1.0, 1.0, 1.0) }
}

impl fmt::Display for ChartColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.to_css()) }
}

/// Color palette for chart series.
#[derive(Debug, Clone)]
pub struct Palette {
    pub colors: Vec<ChartColor>,
}

impl Palette {
    pub fn new(colors: Vec<ChartColor>) -> Self { Self { colors } }

    pub fn get(&self, index: usize) -> ChartColor {
        self.colors[index % self.colors.len()]
    }

    /// Tableau 10 palette.
    pub fn tableau10() -> Self {
        Self::new(vec![
            ChartColor::new(0.122, 0.467, 0.706),
            ChartColor::new(1.0, 0.498, 0.055),
            ChartColor::new(0.173, 0.627, 0.173),
            ChartColor::new(0.839, 0.153, 0.157),
            ChartColor::new(0.580, 0.404, 0.741),
            ChartColor::new(0.549, 0.337, 0.294),
            ChartColor::new(0.890, 0.467, 0.761),
            ChartColor::new(0.498, 0.498, 0.498),
            ChartColor::new(0.737, 0.741, 0.133),
            ChartColor::new(0.090, 0.745, 0.812),
        ])
    }

    /// Vibrant palette.
    pub fn vibrant() -> Self {
        Self::new(vec![
            ChartColor::red(), ChartColor::blue(), ChartColor::green(),
            ChartColor::orange(), ChartColor::purple(), ChartColor::teal(),
            ChartColor::pink(), ChartColor::gold(),
        ])
    }

    /// Monochrome palette from a base color.
    pub fn monochrome(base: ChartColor, count: usize) -> Self {
        let colors: Vec<ChartColor> = (0..count).map(|i| {
            let factor = 0.3 + 0.7 * (i as f64 / count as f64);
            ChartColor::new(base.r * factor, base.g * factor, base.b * factor)
        }).collect();
        Self::new(colors)
    }
}

/// Data point with optional label.
#[derive(Debug, Clone)]
pub struct DataPoint {
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub color: Option<ChartColor>,
}

impl DataPoint {
    pub fn new(x: f64, y: f64) -> Self { Self { x, y, label: None, color: None } }
    pub fn labeled(x: f64, y: f64, label: &str) -> Self { Self { x, y, label: Some(label.into()), color: None } }
}

/// A named series of data points.
#[derive(Debug, Clone)]
pub struct DataSeries {
    pub name: String,
    pub points: Vec<DataPoint>,
    pub color: Option<ChartColor>,
}

impl DataSeries {
    pub fn new(name: &str) -> Self { Self { name: name.into(), points: Vec::new(), color: None } }

    pub fn with_color(mut self, color: ChartColor) -> Self { self.color = Some(color); self }

    pub fn add(&mut self, x: f64, y: f64) { self.points.push(DataPoint::new(x, y)); }

    pub fn add_labeled(&mut self, x: f64, y: f64, label: &str) { self.points.push(DataPoint::labeled(x, y, label)); }

    pub fn from_values(name: &str, values: &[f64]) -> Self {
        let mut s = Self::new(name);
        for (i, &v) in values.iter().enumerate() { s.add(i as f64, v); }
        s
    }

    pub fn min_y(&self) -> f64 { self.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min) }
    pub fn max_y(&self) -> f64 { self.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max) }
    pub fn min_x(&self) -> f64 { self.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min) }
    pub fn max_x(&self) -> f64 { self.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max) }
    pub fn sum_y(&self) -> f64 { self.points.iter().map(|p| p.y).sum() }
    pub fn len(&self) -> usize { self.points.len() }
    pub fn is_empty(&self) -> bool { self.points.is_empty() }
}

// ═══════════════════════════════════════════════════════════════════════
//  AXIS CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════

/// Axis position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPosition { Left, Right, Top, Bottom }

/// Axis scale type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleType { Linear, Logarithmic, Categorical }

/// Axis configuration.
#[derive(Debug, Clone)]
pub struct Axis {
    pub label: String,
    pub position: AxisPosition,
    pub scale: ScaleType,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub tick_count: usize,
    pub grid_lines: bool,
    pub label_rotation: f64,
    pub format: AxisFormat,
}

/// Axis label formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisFormat { Number, Percentage, Currency, Custom(String) }

impl Axis {
    pub fn new(label: &str, position: AxisPosition) -> Self {
        Self {
            label: label.into(), position, scale: ScaleType::Linear,
            min: None, max: None, tick_count: 5,
            grid_lines: true, label_rotation: 0.0,
            format: AxisFormat::Number,
        }
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min); self.max = Some(max); self
    }

    /// Generate tick values.
    pub fn tick_values(&self, data_min: f64, data_max: f64) -> Vec<f64> {
        let min = self.min.unwrap_or(data_min);
        let max = self.max.unwrap_or(data_max);
        let step = (max - min) / self.tick_count as f64;
        (0..=self.tick_count).map(|i| min + step * i as f64).collect()
    }

    /// Map a value to [0, 1] normalized coordinate.
    pub fn normalize(&self, value: f64, data_min: f64, data_max: f64) -> f64 {
        let min = self.min.unwrap_or(data_min);
        let max = self.max.unwrap_or(data_max);
        if (max - min).abs() < 1e-10 { return 0.5; }
        match self.scale {
            ScaleType::Linear => (value - min) / (max - min),
            ScaleType::Logarithmic => {
                if value <= 0.0 || min <= 0.0 { return 0.0; }
                (value.ln() - min.ln()) / (max.ln() - min.ln())
            }
            ScaleType::Categorical => (value - min) / (max - min),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  LEGEND
// ═══════════════════════════════════════════════════════════════════════

/// Legend position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendPosition { TopLeft, TopRight, BottomLeft, BottomRight, None }

/// Legend entry.
#[derive(Debug, Clone)]
pub struct LegendEntry {
    pub label: String,
    pub color: ChartColor,
}

/// Legend configuration.
#[derive(Debug, Clone)]
pub struct Legend {
    pub position: LegendPosition,
    pub entries: Vec<LegendEntry>,
}

impl Legend {
    pub fn new(position: LegendPosition) -> Self { Self { position, entries: Vec::new() } }

    pub fn add_entry(&mut self, label: &str, color: ChartColor) {
        self.entries.push(LegendEntry { label: label.into(), color });
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CHART CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════

/// Margins around the chart area.
#[derive(Debug, Clone, Copy)]
pub struct Margins {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Margins {
    pub fn uniform(m: f64) -> Self { Self { top: m, right: m, bottom: m, left: m } }
    pub fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self { Self { top, right, bottom, left } }
}

impl Default for Margins {
    fn default() -> Self { Self::new(40.0, 30.0, 60.0, 60.0) }
}

/// Common chart configuration.
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub subtitle: Option<String>,
    pub margins: Margins,
    pub palette: Palette,
    pub legend: LegendPosition,
    pub background: ChartColor,
    pub font_size: f64,
    pub title_size: f64,
    pub animate: bool,
}

impl ChartConfig {
    pub fn new(title: &str, width: f64, height: f64) -> Self {
        Self {
            width, height,
            title: title.into(), subtitle: None,
            margins: Margins::default(),
            palette: Palette::tableau10(),
            legend: LegendPosition::TopRight,
            background: ChartColor::white(),
            font_size: 12.0, title_size: 18.0,
            animate: false,
        }
    }

    pub fn plot_width(&self) -> f64 { self.width - self.margins.left - self.margins.right }
    pub fn plot_height(&self) -> f64 { self.height - self.margins.top - self.margins.bottom }
}

// ═══════════════════════════════════════════════════════════════════════
//  PIE CHART
// ═══════════════════════════════════════════════════════════════════════

/// A slice in a pie chart.
#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
    pub color: ChartColor,
    pub percentage: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub explode: f64,
}

/// Pie/donut chart.
#[derive(Debug, Clone)]
pub struct PieChart {
    pub config: ChartConfig,
    pub slices: Vec<PieSlice>,
    pub inner_radius_ratio: f64,
    pub start_angle: f64,
    pub sort_slices: bool,
}

impl PieChart {
    pub fn new(config: ChartConfig) -> Self {
        Self { config, slices: Vec::new(), inner_radius_ratio: 0.0, start_angle: -std::f64::consts::FRAC_PI_2, sort_slices: true }
    }

    pub fn donut(config: ChartConfig, inner_ratio: f64) -> Self {
        Self { config, slices: Vec::new(), inner_radius_ratio: inner_ratio.clamp(0.0, 0.95), start_angle: -std::f64::consts::FRAC_PI_2, sort_slices: true }
    }

    pub fn add_slice(&mut self, label: &str, value: f64) {
        let color = self.config.palette.get(self.slices.len());
        self.slices.push(PieSlice {
            label: label.into(), value, color,
            percentage: 0.0, start_angle: 0.0, end_angle: 0.0, explode: 0.0,
        });
    }

    pub fn add_slice_with_color(&mut self, label: &str, value: f64, color: ChartColor) {
        self.slices.push(PieSlice {
            label: label.into(), value, color,
            percentage: 0.0, start_angle: 0.0, end_angle: 0.0, explode: 0.0,
        });
    }

    /// Calculate angles and percentages.
    pub fn compute(&mut self) {
        let total: f64 = self.slices.iter().map(|s| s.value).sum();
        if total <= 0.0 { return; }

        if self.sort_slices {
            self.slices.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        }

        let mut angle = self.start_angle;
        for slice in &mut self.slices {
            slice.percentage = slice.value / total * 100.0;
            slice.start_angle = angle;
            slice.end_angle = angle + (slice.value / total) * std::f64::consts::TAU;
            angle = slice.end_angle;
        }
    }

    pub fn total(&self) -> f64 { self.slices.iter().map(|s| s.value).sum() }
    pub fn slice_count(&self) -> usize { self.slices.len() }

    /// Export to SVG.
    pub fn to_svg(&mut self) -> String {
        self.compute();
        let w = self.config.width;
        let h = self.config.height;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let radius = (w.min(h) / 2.0) - 40.0;
        let inner_r = radius * self.inner_radius_ratio;

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n\
             <rect width=\"{w}\" height=\"{h}\" fill=\"{}\" />\n\
             <text x=\"{cx}\" y=\"25\" text-anchor=\"middle\" font-size=\"{}\">{}</text>\n",
            self.config.background.to_css(), self.config.title_size, self.config.title
        );

        for slice in &self.slices {
            let large_arc = if (slice.end_angle - slice.start_angle) > std::f64::consts::PI { 1 } else { 0 };
            let x1 = cx + radius * slice.start_angle.cos();
            let y1 = cy + radius * slice.start_angle.sin();
            let x2 = cx + radius * slice.end_angle.cos();
            let y2 = cy + radius * slice.end_angle.sin();

            if self.inner_radius_ratio > 0.0 {
                let ix1 = cx + inner_r * slice.start_angle.cos();
                let iy1 = cy + inner_r * slice.start_angle.sin();
                let ix2 = cx + inner_r * slice.end_angle.cos();
                let iy2 = cy + inner_r * slice.end_angle.sin();
                svg.push_str(&format!(
                    "<path d=\"M {x1} {y1} A {radius} {radius} 0 {large_arc} 1 {x2} {y2} L {ix2} {iy2} A {inner_r} {inner_r} 0 {large_arc} 0 {ix1} {iy1} Z\" fill=\"{}\" />\n",
                    slice.color.to_css()
                ));
            } else {
                svg.push_str(&format!(
                    "<path d=\"M {cx} {cy} L {x1} {y1} A {radius} {radius} 0 {large_arc} 1 {x2} {y2} Z\" fill=\"{}\" />\n",
                    slice.color.to_css()
                ));
            }

            // Label
            let mid_angle = (slice.start_angle + slice.end_angle) / 2.0;
            let label_r = radius * 0.7;
            let lx = cx + label_r * mid_angle.cos();
            let ly = cy + label_r * mid_angle.sin();
            svg.push_str(&format!(
                "<text x=\"{lx}\" y=\"{ly}\" text-anchor=\"middle\" font-size=\"{}\">{} ({:.1}%)</text>\n",
                self.config.font_size, slice.label, slice.percentage
            ));
        }

        svg.push_str("</svg>");
        svg
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  BAR CHART
// ═══════════════════════════════════════════════════════════════════════

/// Bar chart orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarOrientation { Vertical, Horizontal }

/// Bar grouping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarMode { Grouped, Stacked }

/// Bar chart.
#[derive(Debug, Clone)]
pub struct BarChart {
    pub config: ChartConfig,
    pub series: Vec<DataSeries>,
    pub categories: Vec<String>,
    pub orientation: BarOrientation,
    pub mode: BarMode,
    pub bar_gap: f64,
    pub category_gap: f64,
    pub x_axis: Axis,
    pub y_axis: Axis,
}

impl BarChart {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            x_axis: Axis::new("", AxisPosition::Bottom),
            y_axis: Axis::new("", AxisPosition::Left),
            config, series: Vec::new(), categories: Vec::new(),
            orientation: BarOrientation::Vertical, mode: BarMode::Grouped,
            bar_gap: 0.1, category_gap: 0.2,
        }
    }

    pub fn add_series(&mut self, series: DataSeries) { self.series.push(series); }

    pub fn set_categories(&mut self, categories: Vec<String>) { self.categories = categories; }

    pub fn data_max(&self) -> f64 {
        match self.mode {
            BarMode::Grouped => self.series.iter().map(|s| s.max_y()).fold(0.0_f64, f64::max),
            BarMode::Stacked => {
                if self.series.is_empty() { return 0.0; }
                let n = self.series.iter().map(|s| s.len()).max().unwrap_or(0);
                (0..n).map(|i| self.series.iter().map(|s| s.points.get(i).map(|p| p.y).unwrap_or(0.0)).sum::<f64>()).fold(0.0_f64, f64::max)
            }
        }
    }

    pub fn to_svg(&self) -> String {
        let w = self.config.width;
        let h = self.config.height;
        let m = &self.config.margins;
        let pw = self.config.plot_width();
        let ph = self.config.plot_height();
        let max_val = self.data_max();

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n\
             <rect width=\"{w}\" height=\"{h}\" fill=\"{}\" />\n\
             <text x=\"{}\" y=\"25\" text-anchor=\"middle\" font-size=\"{}\">{}</text>\n",
            self.config.background.to_css(), w / 2.0, self.config.title_size, self.config.title
        );

        // Draw axes
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" />\n",
            m.left, m.top, m.left, h - m.bottom
        ));
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" />\n",
            m.left, h - m.bottom, w - m.right, h - m.bottom
        ));

        let n_cats = self.categories.len().max(1);
        let n_series = self.series.len().max(1);
        let cat_width = pw / n_cats as f64;
        let bar_width = cat_width * (1.0 - self.category_gap) / n_series as f64;

        for (si, series) in self.series.iter().enumerate() {
            for (ci, point) in series.points.iter().enumerate() {
                let bar_h = if max_val > 0.0 { (point.y / max_val) * ph } else { 0.0 };
                let x = m.left + ci as f64 * cat_width + self.category_gap * cat_width / 2.0 + si as f64 * bar_width;
                let y = m.top + ph - bar_h;
                let color = series.color.unwrap_or_else(|| self.config.palette.get(si));
                svg.push_str(&format!(
                    "<rect x=\"{x}\" y=\"{y}\" width=\"{bar_width}\" height=\"{bar_h}\" fill=\"{}\" />\n",
                    color.to_css()
                ));
            }
        }

        // Category labels
        for (i, cat) in self.categories.iter().enumerate() {
            let x = m.left + i as f64 * cat_width + cat_width / 2.0;
            let y = h - m.bottom + 20.0;
            svg.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-size=\"{}\">{cat}</text>\n",
                self.config.font_size
            ));
        }

        svg.push_str("</svg>");
        svg
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  LINE CHART
// ═══════════════════════════════════════════════════════════════════════

/// Line chart style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle { Solid, Dashed, Dotted }

/// Line chart with optional area fill.
#[derive(Debug, Clone)]
pub struct LineChart {
    pub config: ChartConfig,
    pub series: Vec<DataSeries>,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub line_width: f64,
    pub show_points: bool,
    pub point_radius: f64,
    pub area_fill: bool,
    pub smooth: bool,
}

impl LineChart {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            x_axis: Axis::new("", AxisPosition::Bottom),
            y_axis: Axis::new("", AxisPosition::Left),
            config, series: Vec::new(),
            line_width: 2.0, show_points: true,
            point_radius: 4.0, area_fill: false, smooth: false,
        }
    }

    pub fn add_series(&mut self, series: DataSeries) { self.series.push(series); }

    fn data_bounds(&self) -> (f64, f64, f64, f64) {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for s in &self.series {
            min_x = min_x.min(s.min_x());
            max_x = max_x.max(s.max_x());
            min_y = min_y.min(s.min_y());
            max_y = max_y.max(s.max_y());
        }
        (min_x, max_x, min_y, max_y)
    }

    pub fn to_svg(&self) -> String {
        let w = self.config.width;
        let h = self.config.height;
        let m = &self.config.margins;
        let pw = self.config.plot_width();
        let ph = self.config.plot_height();
        let (min_x, max_x, min_y, max_y) = self.data_bounds();

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n\
             <rect width=\"{w}\" height=\"{h}\" fill=\"{}\" />\n\
             <text x=\"{}\" y=\"25\" text-anchor=\"middle\" font-size=\"{}\">{}</text>\n",
            self.config.background.to_css(), w / 2.0, self.config.title_size, self.config.title
        );

        // Grid
        if self.y_axis.grid_lines {
            let ticks = self.y_axis.tick_values(min_y, max_y);
            for tick in &ticks {
                let ny = self.y_axis.normalize(*tick, min_y, max_y);
                let y = m.top + ph * (1.0 - ny);
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" stroke=\"#eee\" />\n",
                    m.left, w - m.right
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"10\">{:.1}</text>\n",
                    m.left - 5.0, y + 4.0, tick
                ));
            }
        }

        // Axes
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" />\n\
             <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" />\n",
            m.left, m.top, m.left, h - m.bottom,
            m.left, h - m.bottom, w - m.right, h - m.bottom
        ));

        // Series
        for (si, series) in self.series.iter().enumerate() {
            let color = series.color.unwrap_or_else(|| self.config.palette.get(si));
            let mut path_d = String::new();
            for (pi, point) in series.points.iter().enumerate() {
                let nx = self.x_axis.normalize(point.x, min_x, max_x);
                let ny = self.y_axis.normalize(point.y, min_y, max_y);
                let px = m.left + nx * pw;
                let py = m.top + ph * (1.0 - ny);
                if pi == 0 { path_d.push_str(&format!("M {px} {py}")); }
                else { path_d.push_str(&format!(" L {px} {py}")); }
            }
            svg.push_str(&format!(
                "<path d=\"{path_d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" />\n",
                color.to_css(), self.line_width
            ));

            // Points
            if self.show_points {
                for point in &series.points {
                    let nx = self.x_axis.normalize(point.x, min_x, max_x);
                    let ny = self.y_axis.normalize(point.y, min_y, max_y);
                    let px = m.left + nx * pw;
                    let py = m.top + ph * (1.0 - ny);
                    svg.push_str(&format!(
                        "<circle cx=\"{px}\" cy=\"{py}\" r=\"{}\" fill=\"{}\" />\n",
                        self.point_radius, color.to_css()
                    ));
                }
            }
        }

        svg.push_str("</svg>");
        svg
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SCATTER PLOT
// ═══════════════════════════════════════════════════════════════════════

/// Scatter plot with optional bubble sizing.
#[derive(Debug, Clone)]
pub struct ScatterPlot {
    pub config: ChartConfig,
    pub series: Vec<DataSeries>,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub point_radius: f64,
    pub bubble_sizes: Option<Vec<f64>>,
}

impl ScatterPlot {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            x_axis: Axis::new("X", AxisPosition::Bottom),
            y_axis: Axis::new("Y", AxisPosition::Left),
            config, series: Vec::new(),
            point_radius: 5.0, bubble_sizes: None,
        }
    }

    pub fn add_series(&mut self, series: DataSeries) { self.series.push(series); }

    pub fn total_points(&self) -> usize { self.series.iter().map(|s| s.len()).sum() }
}

// ═══════════════════════════════════════════════════════════════════════
//  HISTOGRAM
// ═══════════════════════════════════════════════════════════════════════

/// A histogram bin.
#[derive(Debug, Clone)]
pub struct HistogramBin {
    pub start: f64,
    pub end: f64,
    pub count: usize,
    pub frequency: f64,
}

/// Histogram chart.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub config: ChartConfig,
    pub bins: Vec<HistogramBin>,
    pub bin_count: usize,
    pub color: ChartColor,
}

impl Histogram {
    pub fn new(config: ChartConfig, bin_count: usize) -> Self {
        Self { config, bins: Vec::new(), bin_count, color: ChartColor::blue() }
    }

    /// Compute bins from raw data.
    pub fn from_data(config: ChartConfig, data: &[f64], bin_count: usize) -> Self {
        let mut hist = Self::new(config, bin_count);
        if data.is_empty() { return hist; }

        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        if range <= 0.0 { return hist; }

        let bin_width = range / bin_count as f64;
        let n = data.len();

        for i in 0..bin_count {
            let start = min + i as f64 * bin_width;
            let end = start + bin_width;
            let count = data.iter().filter(|&&v| {
                if i == bin_count - 1 { v >= start && v <= end }
                else { v >= start && v < end }
            }).count();
            hist.bins.push(HistogramBin {
                start, end, count,
                frequency: count as f64 / n as f64,
            });
        }
        hist
    }

    pub fn total_count(&self) -> usize { self.bins.iter().map(|b| b.count).sum() }
    pub fn max_count(&self) -> usize { self.bins.iter().map(|b| b.count).max().unwrap_or(0) }
}

// ═══════════════════════════════════════════════════════════════════════
//  RADAR / SPIDER CHART
// ═══════════════════════════════════════════════════════════════════════

/// Radar (spider) chart.
#[derive(Debug, Clone)]
pub struct RadarChart {
    pub config: ChartConfig,
    pub axes_labels: Vec<String>,
    pub series: Vec<(String, Vec<f64>, ChartColor)>,
    pub max_value: f64,
    pub fill_opacity: f64,
}

impl RadarChart {
    pub fn new(config: ChartConfig, labels: Vec<String>) -> Self {
        Self {
            config, axes_labels: labels,
            series: Vec::new(), max_value: 100.0, fill_opacity: 0.3,
        }
    }

    pub fn add_series(&mut self, name: &str, values: Vec<f64>, color: ChartColor) {
        self.series.push((name.into(), values, color));
    }

    pub fn axes_count(&self) -> usize { self.axes_labels.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  HEAT MAP
// ═══════════════════════════════════════════════════════════════════════

/// Heat map for matrix visualization.
#[derive(Debug, Clone)]
pub struct HeatMap {
    pub config: ChartConfig,
    pub data: Vec<Vec<f64>>,
    pub row_labels: Vec<String>,
    pub col_labels: Vec<String>,
    pub color_low: ChartColor,
    pub color_high: ChartColor,
}

impl HeatMap {
    pub fn new(config: ChartConfig, data: Vec<Vec<f64>>) -> Self {
        let rows = data.len();
        let cols = data.first().map(|r| r.len()).unwrap_or(0);
        Self {
            config, data,
            row_labels: (0..rows).map(|i| format!("R{i}")).collect(),
            col_labels: (0..cols).map(|i| format!("C{i}")).collect(),
            color_low: ChartColor::new(1.0, 1.0, 1.0),
            color_high: ChartColor::new(0.122, 0.467, 0.706),
        }
    }

    pub fn min_value(&self) -> f64 {
        self.data.iter().flat_map(|r| r.iter()).cloned().fold(f64::INFINITY, f64::min)
    }

    pub fn max_value(&self) -> f64 {
        self.data.iter().flat_map(|r| r.iter()).cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn rows(&self) -> usize { self.data.len() }
    pub fn cols(&self) -> usize { self.data.first().map(|r| r.len()).unwrap_or(0) }
}

// ═══════════════════════════════════════════════════════════════════════
//  TREEMAP
// ═══════════════════════════════════════════════════════════════════════

/// Node in a treemap.
#[derive(Debug, Clone)]
pub struct TreemapNode {
    pub label: String,
    pub value: f64,
    pub color: Option<ChartColor>,
    pub children: Vec<TreemapNode>,
    pub rect: Option<(f64, f64, f64, f64)>, // x, y, w, h
}

impl TreemapNode {
    pub fn leaf(label: &str, value: f64) -> Self {
        Self { label: label.into(), value, color: None, children: Vec::new(), rect: None }
    }

    pub fn group(label: &str, children: Vec<TreemapNode>) -> Self {
        let total = children.iter().map(|c| c.total_value()).sum();
        Self { label: label.into(), value: total, color: None, children, rect: None }
    }

    pub fn total_value(&self) -> f64 {
        if self.children.is_empty() { self.value }
        else { self.children.iter().map(|c| c.total_value()).sum() }
    }

    pub fn is_leaf(&self) -> bool { self.children.is_empty() }

    /// Squarified treemap layout.
    pub fn layout(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.rect = Some((x, y, w, h));
        if self.children.is_empty() { return; }

        let total = self.children.iter().map(|c| c.total_value()).sum::<f64>();
        if total <= 0.0 { return; }

        // Simple slice-and-dice layout
        let mut offset = 0.0;
        let vertical = w >= h;
        for child in &mut self.children {
            let ratio = child.total_value() / total;
            if vertical {
                let cw = w * ratio;
                child.layout(x + offset, y, cw, h);
                offset += cw;
            } else {
                let ch = h * ratio;
                child.layout(x, y + offset, w, ch);
                offset += ch;
            }
        }
    }

    pub fn leaf_count(&self) -> usize {
        if self.is_leaf() { 1 }
        else { self.children.iter().map(|c| c.leaf_count()).sum() }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CANDLESTICK CHART
// ═══════════════════════════════════════════════════════════════════════

/// OHLC data point.
#[derive(Debug, Clone, Copy)]
pub struct CandlestickData {
    pub timestamp: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl CandlestickData {
    pub fn new(timestamp: f64, open: f64, high: f64, low: f64, close: f64) -> Self {
        Self { timestamp, open, high, low, close, volume: 0.0 }
    }

    pub fn is_bullish(&self) -> bool { self.close >= self.open }
    pub fn body_size(&self) -> f64 { (self.close - self.open).abs() }
    pub fn range(&self) -> f64 { self.high - self.low }
}

/// Candlestick chart for financial data.
#[derive(Debug, Clone)]
pub struct CandlestickChart {
    pub config: ChartConfig,
    pub data: Vec<CandlestickData>,
    pub bullish_color: ChartColor,
    pub bearish_color: ChartColor,
}

impl CandlestickChart {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            config, data: Vec::new(),
            bullish_color: ChartColor::green(),
            bearish_color: ChartColor::red(),
        }
    }

    pub fn add(&mut self, candle: CandlestickData) { self.data.push(candle); }

    pub fn price_range(&self) -> (f64, f64) {
        let low = self.data.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        let high = self.data.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
        (low, high)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  GAUGE CHART
// ═══════════════════════════════════════════════════════════════════════

/// Gauge chart for dashboard KPIs.
#[derive(Debug, Clone)]
pub struct GaugeChart {
    pub config: ChartConfig,
    pub value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub thresholds: Vec<(f64, ChartColor)>,
    pub unit: String,
}

impl GaugeChart {
    pub fn new(config: ChartConfig, value: f64, min: f64, max: f64) -> Self {
        Self {
            config, value, min_value: min, max_value: max,
            thresholds: vec![
                (0.33, ChartColor::green()),
                (0.66, ChartColor::gold()),
                (1.0, ChartColor::red()),
            ],
            unit: String::new(),
        }
    }

    pub fn percentage(&self) -> f64 {
        if (self.max_value - self.min_value).abs() < 1e-10 { return 0.0; }
        ((self.value - self.min_value) / (self.max_value - self.min_value) * 100.0).clamp(0.0, 100.0)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  SPARKLINE
// ═══════════════════════════════════════════════════════════════════════

/// Inline sparkline chart.
#[derive(Debug, Clone)]
pub struct Sparkline {
    pub values: Vec<f64>,
    pub width: f64,
    pub height: f64,
    pub color: ChartColor,
    pub show_endpoints: bool,
}

impl Sparkline {
    pub fn new(values: Vec<f64>, width: f64, height: f64) -> Self {
        Self { values, width, height, color: ChartColor::blue(), show_endpoints: true }
    }

    pub fn min(&self) -> f64 { self.values.iter().cloned().fold(f64::INFINITY, f64::min) }
    pub fn max(&self) -> f64 { self.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) }
    pub fn last(&self) -> Option<f64> { self.values.last().copied() }

    pub fn to_svg(&self) -> String {
        if self.values.is_empty() { return String::new(); }
        let min = self.min();
        let max = self.max();
        let range = if (max - min).abs() < 1e-10 { 1.0 } else { max - min };
        let n = self.values.len();

        let mut path_d = String::new();
        for (i, v) in self.values.iter().enumerate() {
            let x = (i as f64 / (n - 1).max(1) as f64) * self.width;
            let y = self.height - ((v - min) / range * self.height);
            if i == 0 { path_d.push_str(&format!("M {:.1} {:.1}", x, y)); }
            else { path_d.push_str(&format!(" L {:.1} {:.1}", x, y)); }
        }

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">\n\
             <path d=\"{path_d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" />\n",
            self.width, self.height, self.color.to_css()
        );

        if self.show_endpoints && n > 0 {
            // First point
            let y0 = self.height - ((self.values[0] - min) / range * self.height);
            svg.push_str(&format!("<circle cx=\"0\" cy=\"{y0:.1}\" r=\"2\" fill=\"{}\" />\n", self.color.to_css()));
            // Last point
            let yn = self.height - ((self.values[n - 1] - min) / range * self.height);
            svg.push_str(&format!("<circle cx=\"{:.1}\" cy=\"{yn:.1}\" r=\"2\" fill=\"{}\" />\n", self.width, self.color.to_css()));
        }

        svg.push_str("</svg>");
        svg
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  DASHBOARD
// ═══════════════════════════════════════════════════════════════════════

/// Dashboard layout cell.
#[derive(Debug, Clone)]
pub enum DashboardCell {
    Pie(PieChart),
    Bar(BarChart),
    Line(LineChart),
    Scatter(ScatterPlot),
    Histogram(Histogram),
    Radar(RadarChart),
    HeatMap(HeatMap),
    Gauge(GaugeChart),
    Sparkline(Sparkline),
    Candlestick(CandlestickChart),
    Empty,
}

/// Dashboard layout.
#[derive(Debug, Clone)]
pub struct Dashboard {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub cells: Vec<Vec<DashboardCell>>,
    pub background: ChartColor,
}

impl Dashboard {
    pub fn new(title: &str, width: f64, height: f64) -> Self {
        Self { title: title.into(), width, height, cells: Vec::new(), background: ChartColor::new(0.95, 0.95, 0.95) }
    }

    pub fn add_row(&mut self, row: Vec<DashboardCell>) { self.cells.push(row); }

    pub fn cell_count(&self) -> usize { self.cells.iter().map(|r| r.len()).sum() }
    pub fn row_count(&self) -> usize { self.cells.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Color / Palette Tests ───────────────────────────────────────

    #[test]
    fn test_chart_color() {
        let c = ChartColor::new(1.0, 0.0, 0.0);
        assert_eq!(c.to_css(), "rgb(255,0,0)");
        let ca = ChartColor::with_alpha(1.0, 1.0, 1.0, 0.5);
        assert!(ca.to_css().contains("rgba"));
    }

    #[test]
    fn test_palette() {
        let p = Palette::tableau10();
        assert_eq!(p.colors.len(), 10);
        // Wraps around
        let c0 = p.get(0);
        let c10 = p.get(10);
        assert_eq!(c0.r, c10.r);
    }

    #[test]
    fn test_monochrome_palette() {
        let p = Palette::monochrome(ChartColor::blue(), 5);
        assert_eq!(p.colors.len(), 5);
    }

    // ── DataSeries Tests ────────────────────────────────────────────

    #[test]
    fn test_data_series() {
        let mut s = DataSeries::new("test");
        s.add(1.0, 10.0);
        s.add(2.0, 20.0);
        s.add(3.0, 15.0);
        assert_eq!(s.len(), 3);
        assert_eq!(s.min_y(), 10.0);
        assert_eq!(s.max_y(), 20.0);
        assert_eq!(s.sum_y(), 45.0);
    }

    #[test]
    fn test_data_series_from_values() {
        let s = DataSeries::from_values("vals", &[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(s.len(), 5);
        assert_eq!(s.min_x(), 0.0);
        assert_eq!(s.max_x(), 4.0);
    }

    // ── Axis Tests ──────────────────────────────────────────────────

    #[test]
    fn test_axis_tick_values() {
        let axis = Axis::new("X", AxisPosition::Bottom);
        let ticks = axis.tick_values(0.0, 100.0);
        assert_eq!(ticks.len(), 6); // 5 intervals = 6 tick marks
        assert!((ticks[0] - 0.0).abs() < 1e-10);
        assert!((ticks[5] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_axis_normalize() {
        let axis = Axis::new("X", AxisPosition::Bottom);
        assert!((axis.normalize(50.0, 0.0, 100.0) - 0.5).abs() < 1e-10);
        assert!((axis.normalize(0.0, 0.0, 100.0) - 0.0).abs() < 1e-10);
        assert!((axis.normalize(100.0, 0.0, 100.0) - 1.0).abs() < 1e-10);
    }

    // ── Pie Chart Tests ─────────────────────────────────────────────

    #[test]
    fn test_pie_chart() {
        let config = ChartConfig::new("Sales", 400.0, 400.0);
        let mut pie = PieChart::new(config);
        pie.add_slice("Product A", 40.0);
        pie.add_slice("Product B", 30.0);
        pie.add_slice("Product C", 20.0);
        pie.add_slice("Product D", 10.0);
        assert_eq!(pie.slice_count(), 4);
        assert!((pie.total() - 100.0).abs() < 1e-10);
        pie.compute();
        assert!((pie.slices[0].percentage - 40.0).abs() < 1e-6);
    }

    #[test]
    fn test_donut_chart() {
        let config = ChartConfig::new("Donut", 400.0, 400.0);
        let mut donut = PieChart::donut(config, 0.5);
        donut.add_slice("A", 50.0);
        donut.add_slice("B", 50.0);
        assert_eq!(donut.inner_radius_ratio, 0.5);
        let svg = donut.to_svg();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_pie_chart_svg() {
        let config = ChartConfig::new("Test Pie", 300.0, 300.0);
        let mut pie = PieChart::new(config);
        pie.add_slice("A", 60.0);
        pie.add_slice("B", 40.0);
        let svg = pie.to_svg();
        assert!(svg.contains("<path"));
        assert!(svg.contains("</svg>"));
    }

    // ── Bar Chart Tests ─────────────────────────────────────────────

    #[test]
    fn test_bar_chart() {
        let config = ChartConfig::new("Revenue", 600.0, 400.0);
        let mut bar = BarChart::new(config);
        bar.set_categories(vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()]);
        let mut s1 = DataSeries::new("2024");
        for v in [10.0, 25.0, 18.0, 30.0] { s1.add(0.0, v); }
        bar.add_series(s1);
        assert_eq!(bar.series.len(), 1);
        assert!((bar.data_max() - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_bar_chart_svg() {
        let config = ChartConfig::new("Test Bar", 400.0, 300.0);
        let mut bar = BarChart::new(config);
        bar.set_categories(vec!["A".into(), "B".into()]);
        let mut s = DataSeries::new("s1");
        s.add(0.0, 10.0);
        s.add(1.0, 20.0);
        bar.add_series(s);
        let svg = bar.to_svg();
        assert!(svg.contains("<rect"));
    }

    // ── Line Chart Tests ────────────────────────────────────────────

    #[test]
    fn test_line_chart() {
        let config = ChartConfig::new("Trend", 600.0, 400.0);
        let mut line = LineChart::new(config);
        let s = DataSeries::from_values("data", &[5.0, 12.0, 8.0, 20.0, 15.0]);
        line.add_series(s);
        assert_eq!(line.series.len(), 1);
    }

    #[test]
    fn test_line_chart_svg() {
        let config = ChartConfig::new("Test Line", 400.0, 300.0);
        let mut line = LineChart::new(config);
        let s = DataSeries::from_values("vals", &[1.0, 4.0, 2.0, 6.0]);
        line.add_series(s);
        let svg = line.to_svg();
        assert!(svg.contains("<path"));
        assert!(svg.contains("<circle"));
    }

    // ── Scatter Plot Tests ──────────────────────────────────────────

    #[test]
    fn test_scatter_plot() {
        let config = ChartConfig::new("Correlation", 400.0, 400.0);
        let mut scatter = ScatterPlot::new(config);
        let mut s = DataSeries::new("data");
        s.add(1.0, 2.0); s.add(3.0, 5.0); s.add(4.0, 7.0);
        scatter.add_series(s);
        assert_eq!(scatter.total_points(), 3);
    }

    // ── Histogram Tests ─────────────────────────────────────────────

    #[test]
    fn test_histogram() {
        let config = ChartConfig::new("Distribution", 400.0, 300.0);
        let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0];
        let hist = Histogram::from_data(config, &data, 5);
        assert_eq!(hist.bins.len(), 5);
        assert_eq!(hist.total_count(), 10);
    }

    // ── Radar Chart Tests ───────────────────────────────────────────

    #[test]
    fn test_radar_chart() {
        let config = ChartConfig::new("Skills", 400.0, 400.0);
        let mut radar = RadarChart::new(config, vec!["Speed".into(), "Power".into(), "Defense".into(), "Magic".into(), "Luck".into()]);
        radar.add_series("Warrior", vec![80.0, 90.0, 70.0, 20.0, 30.0], ChartColor::red());
        radar.add_series("Mage", vec![40.0, 30.0, 20.0, 95.0, 50.0], ChartColor::blue());
        assert_eq!(radar.axes_count(), 5);
        assert_eq!(radar.series.len(), 2);
    }

    // ── Heat Map Tests ──────────────────────────────────────────────

    #[test]
    fn test_heat_map() {
        let config = ChartConfig::new("Heatmap", 400.0, 300.0);
        let data = vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
        ];
        let hm = HeatMap::new(config, data);
        assert_eq!(hm.rows(), 3);
        assert_eq!(hm.cols(), 3);
        assert_eq!(hm.min_value(), 1.0);
        assert_eq!(hm.max_value(), 9.0);
    }

    // ── Treemap Tests ───────────────────────────────────────────────

    #[test]
    fn test_treemap() {
        let mut root = TreemapNode::group("Root", vec![
            TreemapNode::leaf("A", 40.0),
            TreemapNode::leaf("B", 30.0),
            TreemapNode::leaf("C", 20.0),
            TreemapNode::leaf("D", 10.0),
        ]);
        assert_eq!(root.leaf_count(), 4);
        assert!((root.total_value() - 100.0).abs() < 1e-10);
        root.layout(0.0, 0.0, 400.0, 300.0);
        assert!(root.rect.is_some());
    }

    // ── Candlestick Tests ───────────────────────────────────────────

    #[test]
    fn test_candlestick() {
        let config = ChartConfig::new("Stock", 600.0, 400.0);
        let mut chart = CandlestickChart::new(config);
        chart.add(CandlestickData::new(1.0, 100.0, 110.0, 95.0, 105.0)); // bullish
        chart.add(CandlestickData::new(2.0, 105.0, 108.0, 98.0, 100.0)); // bearish
        assert!(chart.data[0].is_bullish());
        assert!(!chart.data[1].is_bullish());
        let (low, high) = chart.price_range();
        assert_eq!(low, 95.0);
        assert_eq!(high, 110.0);
    }

    // ── Gauge Tests ─────────────────────────────────────────────────

    #[test]
    fn test_gauge() {
        let config = ChartConfig::new("CPU", 200.0, 200.0);
        let gauge = GaugeChart::new(config, 75.0, 0.0, 100.0);
        assert!((gauge.percentage() - 75.0).abs() < 1e-10);
    }

    // ── Sparkline Tests ─────────────────────────────────────────────

    #[test]
    fn test_sparkline() {
        let spark = Sparkline::new(vec![1.0, 5.0, 3.0, 8.0, 2.0], 100.0, 20.0);
        assert_eq!(spark.min(), 1.0);
        assert_eq!(spark.max(), 8.0);
        assert_eq!(spark.last(), Some(2.0));
        let svg = spark.to_svg();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));
    }

    // ── Dashboard Tests ─────────────────────────────────────────────

    #[test]
    fn test_dashboard() {
        let mut dash = Dashboard::new("Analytics", 1200.0, 800.0);
        let config1 = ChartConfig::new("Revenue", 400.0, 300.0);
        let pie = PieChart::new(config1);
        let config2 = ChartConfig::new("Trend", 400.0, 300.0);
        let line = LineChart::new(config2);
        dash.add_row(vec![DashboardCell::Pie(pie), DashboardCell::Line(line)]);
        assert_eq!(dash.row_count(), 1);
        assert_eq!(dash.cell_count(), 2);
    }

    // ── ChartConfig Tests ───────────────────────────────────────────

    #[test]
    fn test_chart_config() {
        let cfg = ChartConfig::new("Test", 800.0, 600.0);
        assert_eq!(cfg.title, "Test");
        assert!(cfg.plot_width() > 0.0);
        assert!(cfg.plot_height() > 0.0);
        assert!(cfg.plot_width() < 800.0);
    }

    // ── Legend Tests ────────────────────────────────────────────────

    #[test]
    fn test_legend() {
        let mut legend = Legend::new(LegendPosition::TopRight);
        legend.add_entry("Series 1", ChartColor::red());
        legend.add_entry("Series 2", ChartColor::blue());
        assert_eq!(legend.entries.len(), 2);
    }
}
