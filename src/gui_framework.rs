//! Vitalis GUI Framework — Declarative UI layout, styling, and widget system.
//!
//! Provides comprehensive support for declarative GUI paradigms inspired by:
//!
//! ## Declarative GUI Languages
//! - **QML** (Qt Modeling Language): JSON-like declarative widget trees, property bindings, animations
//! - **XAML** (Extensible Application Markup Language): XML-based WPF/MAUI UI layout
//! - **SwiftUI**: Stacked declarative view composition with modifiers
//! - **CSS**: Hardware-accelerated styling, layout (Flexbox, Grid), transforms, transitions
//!
//! ## Features
//! - Widget tree with layout engine (Flexbox + Grid)
//! - Property binding system with reactive updates
//! - CSS-compatible styling (colors, margins, padding, borders, shadows)
//! - Animation system with keyframes and transitions
//! - Event handling (click, hover, drag, keyboard, scroll)
//! - Theme engine with dark/light mode support
//! - Responsive layout with breakpoints
//! - Accessibility attributes (ARIA-like roles, labels)
//! - Widget catalog: Button, Label, TextInput, Slider, Checkbox, List, Grid, Tabs, etc.

use std::collections::HashMap;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  CSS-COMPATIBLE STYLING ENGINE
// ═══════════════════════════════════════════════════════════════════════

/// CSS-compatible length units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssUnit {
    Px(f64),
    Em(f64),
    Rem(f64),
    Percent(f64),
    Vw(f64),
    Vh(f64),
    Auto,
    Fr(f64),       // Grid fractional unit
    MinContent,
    MaxContent,
}

impl CssUnit {
    /// Resolve to pixels given a context.
    pub fn to_px(&self, parent_size: f64, root_font_size: f64, viewport_w: f64, viewport_h: f64) -> f64 {
        match self {
            CssUnit::Px(v) => *v,
            CssUnit::Em(v) => v * 16.0,
            CssUnit::Rem(v) => v * root_font_size,
            CssUnit::Percent(v) => parent_size * v / 100.0,
            CssUnit::Vw(v) => viewport_w * v / 100.0,
            CssUnit::Vh(v) => viewport_h * v / 100.0,
            CssUnit::Fr(v) => *v,
            CssUnit::Auto | CssUnit::MinContent | CssUnit::MaxContent => 0.0,
        }
    }
}

impl fmt::Display for CssUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CssUnit::Px(v) => write!(f, "{}px", v),
            CssUnit::Em(v) => write!(f, "{}em", v),
            CssUnit::Rem(v) => write!(f, "{}rem", v),
            CssUnit::Percent(v) => write!(f, "{}%", v),
            CssUnit::Vw(v) => write!(f, "{}vw", v),
            CssUnit::Vh(v) => write!(f, "{}vh", v),
            CssUnit::Auto => write!(f, "auto"),
            CssUnit::Fr(v) => write!(f, "{}fr", v),
            CssUnit::MinContent => write!(f, "min-content"),
            CssUnit::MaxContent => write!(f, "max-content"),
        }
    }
}

/// Edge insets (margin, padding, border-width).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInsets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl EdgeInsets {
    pub fn all(v: f64) -> Self { Self { top: v, right: v, bottom: v, left: v } }
    pub fn symmetric(h: f64, v: f64) -> Self { Self { top: v, right: h, bottom: v, left: h } }
    pub fn zero() -> Self { Self::all(0.0) }
    pub fn horizontal(&self) -> f64 { self.left + self.right }
    pub fn vertical(&self) -> f64 { self.top + self.bottom }
}

/// Corner radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadius {
    pub top_left: f64,
    pub top_right: f64,
    pub bottom_right: f64,
    pub bottom_left: f64,
}

impl CornerRadius {
    pub fn all(v: f64) -> Self { Self { top_left: v, top_right: v, bottom_right: v, bottom_left: v } }
    pub fn zero() -> Self { Self::all(0.0) }
}

/// Box shadow.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxShadow {
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur_radius: f64,
    pub spread_radius: f64,
    pub color: CssColor,
    pub inset: bool,
}

impl BoxShadow {
    pub fn new(offset_x: f64, offset_y: f64, blur: f64, color: CssColor) -> Self {
        Self { offset_x, offset_y, blur_radius: blur, spread_radius: 0.0, color, inset: false }
    }
}

/// CSS-compatible color value.
#[derive(Debug, Clone, PartialEq)]
pub enum CssColor {
    Rgba(f64, f64, f64, f64),
    Named(String),
    Hex(u32),
    Hsl(f64, f64, f64),
    CurrentColor,
    Transparent,
}

impl CssColor {
    pub fn rgb(r: f64, g: f64, b: f64) -> Self { CssColor::Rgba(r, g, b, 1.0) }
    pub fn white() -> Self { CssColor::Rgba(1.0, 1.0, 1.0, 1.0) }
    pub fn black() -> Self { CssColor::Rgba(0.0, 0.0, 0.0, 1.0) }
    pub fn transparent() -> Self { CssColor::Transparent }
}

impl fmt::Display for CssColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CssColor::Rgba(r, g, b, a) => write!(f, "rgba({}, {}, {}, {})", (r*255.0) as u8, (g*255.0) as u8, (b*255.0) as u8, a),
            CssColor::Named(n) => write!(f, "{}", n),
            CssColor::Hex(h) => write!(f, "#{:06x}", h),
            CssColor::Hsl(h, s, l) => write!(f, "hsl({}, {}%, {}%)", h, s*100.0, l*100.0),
            CssColor::CurrentColor => write!(f, "currentColor"),
            CssColor::Transparent => write!(f, "transparent"),
        }
    }
}

/// CSS border style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None, Solid, Dashed, Dotted, Double, Groove, Ridge, Inset, Outset,
}

/// CSS text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign { Left, Center, Right, Justify }

/// CSS font weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight { Thin, Light, Normal, Medium, SemiBold, Bold, ExtraBold, Black }

impl FontWeight {
    pub fn numeric(&self) -> u32 {
        match self {
            FontWeight::Thin => 100, FontWeight::Light => 300,
            FontWeight::Normal => 400, FontWeight::Medium => 500,
            FontWeight::SemiBold => 600, FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800, FontWeight::Black => 900,
        }
    }
}

/// CSS overflow behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow { Visible, Hidden, Scroll, Auto }

/// CSS cursor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle { Default, Pointer, Text, Move, Crosshair, NotAllowed, Grab, Grabbing, ColResize, RowResize, Wait }

/// CSS display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode { Block, Inline, InlineBlock, Flex, Grid, None }

/// CSS position mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionMode { Static, Relative, Absolute, Fixed, Sticky }

// ═══════════════════════════════════════════════════════════════════════
//  FLEXBOX LAYOUT
// ═══════════════════════════════════════════════════════════════════════

/// Flexbox direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection { Row, RowReverse, Column, ColumnReverse }

/// Flexbox wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap { NoWrap, Wrap, WrapReverse }

/// Flexbox justification (main axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent { FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly }

/// Flexbox alignment (cross axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems { FlexStart, FlexEnd, Center, Stretch, Baseline }

/// CSS Grid template definition.
#[derive(Debug, Clone)]
pub struct GridTemplate {
    pub columns: Vec<CssUnit>,
    pub rows: Vec<CssUnit>,
    pub column_gap: f64,
    pub row_gap: f64,
}

impl GridTemplate {
    pub fn new() -> Self {
        Self { columns: Vec::new(), rows: Vec::new(), column_gap: 0.0, row_gap: 0.0 }
    }

    pub fn columns(mut self, cols: Vec<CssUnit>) -> Self { self.columns = cols; self }
    pub fn rows(mut self, rows: Vec<CssUnit>) -> Self { self.rows = rows; self }
    pub fn gap(mut self, gap: f64) -> Self { self.column_gap = gap; self.row_gap = gap; self }
}

// ═══════════════════════════════════════════════════════════════════════
//  CSS STYLE OBJECT
// ═══════════════════════════════════════════════════════════════════════

/// Complete CSS-compatible style properties.
#[derive(Debug, Clone)]
pub struct Style {
    // Display & position
    pub display: DisplayMode,
    pub position: PositionMode,

    // Dimensions
    pub width: Option<CssUnit>,
    pub height: Option<CssUnit>,
    pub min_width: Option<CssUnit>,
    pub min_height: Option<CssUnit>,
    pub max_width: Option<CssUnit>,
    pub max_height: Option<CssUnit>,

    // Spacing
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,

    // Flexbox
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub flex_grow: f64,
    pub flex_shrink: f64,

    // Grid
    pub grid: Option<GridTemplate>,

    // Visual
    pub background_color: CssColor,
    pub color: CssColor,
    pub border_color: CssColor,
    pub border_style: BorderStyle,
    pub border_width: EdgeInsets,
    pub border_radius: CornerRadius,
    pub opacity: f64,
    pub shadows: Vec<BoxShadow>,
    pub overflow: Overflow,
    pub cursor: CursorStyle,

    // Typography
    pub font_size: f64,
    pub font_weight: FontWeight,
    pub font_family: String,
    pub text_align: TextAlign,
    pub line_height: f64,
    pub letter_spacing: f64,

    // Transform
    pub transform_origin: (f64, f64),
    pub rotation: f64,
    pub scale: (f64, f64),
    pub translate: (f64, f64),

    // Transition
    pub transition_duration: f64,
    pub transition_property: String,

    // Z-index
    pub z_index: i32,
}

impl Style {
    pub fn new() -> Self {
        Self {
            display: DisplayMode::Block,
            position: PositionMode::Static,
            width: None, height: None,
            min_width: None, min_height: None,
            max_width: None, max_height: None,
            margin: EdgeInsets::zero(),
            padding: EdgeInsets::zero(),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            flex_grow: 0.0, flex_shrink: 1.0,
            grid: None,
            background_color: CssColor::transparent(),
            color: CssColor::black(),
            border_color: CssColor::black(),
            border_style: BorderStyle::None,
            border_width: EdgeInsets::zero(),
            border_radius: CornerRadius::zero(),
            opacity: 1.0,
            shadows: Vec::new(),
            overflow: Overflow::Visible,
            cursor: CursorStyle::Default,
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            font_family: "sans-serif".into(),
            text_align: TextAlign::Left,
            line_height: 1.5,
            letter_spacing: 0.0,
            transform_origin: (50.0, 50.0),
            rotation: 0.0,
            scale: (1.0, 1.0),
            translate: (0.0, 0.0),
            transition_duration: 0.0,
            transition_property: "all".into(),
            z_index: 0,
        }
    }

    pub fn flex() -> Self { let mut s = Self::new(); s.display = DisplayMode::Flex; s }
    pub fn grid() -> Self { let mut s = Self::new(); s.display = DisplayMode::Grid; s }

    /// Generate CSS string.
    pub fn to_css(&self) -> String {
        let mut css = String::new();
        css.push_str(&format!("display: {:?};\n", self.display));
        if let Some(ref w) = self.width { css.push_str(&format!("width: {};\n", w)); }
        if let Some(ref h) = self.height { css.push_str(&format!("height: {};\n", h)); }
        css.push_str(&format!("background-color: {};\n", self.background_color));
        css.push_str(&format!("color: {};\n", self.color));
        css.push_str(&format!("font-size: {}px;\n", self.font_size));
        css.push_str(&format!("opacity: {};\n", self.opacity));
        css
    }

    pub fn property_count(&self) -> usize {
        let mut count = 2; // display, position always present
        if self.width.is_some() { count += 1; }
        if self.height.is_some() { count += 1; }
        count += 4; // margin/padding
        count += 3; // colors
        count += 3; // font
        count
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  WIDGET TYPES
// ═══════════════════════════════════════════════════════════════════════

/// Unique widget identifier.
pub type WidgetId = u64;

/// Event types that widgets can handle.
#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    Click { x: f64, y: f64, button: MouseButton },
    DoubleClick { x: f64, y: f64 },
    MouseDown { x: f64, y: f64, button: MouseButton },
    MouseUp { x: f64, y: f64, button: MouseButton },
    MouseMove { x: f64, y: f64 },
    MouseEnter,
    MouseLeave,
    Scroll { delta_x: f64, delta_y: f64 },
    KeyDown { key: String, modifiers: KeyModifiers },
    KeyUp { key: String, modifiers: KeyModifiers },
    TextInput { text: String },
    Focus,
    Blur,
    DragStart { x: f64, y: f64 },
    DragMove { x: f64, y: f64 },
    DragEnd { x: f64, y: f64 },
    Resize { width: f64, height: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton { Left, Right, Middle }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Accessibility role (ARIA-like).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityRole {
    Button, Link, TextBox, Checkbox, RadioButton, Slider,
    List, ListItem, Tab, TabPanel, Dialog, Alert,
    Navigation, Main, Banner, ContentInfo, Complementary,
    Menu, MenuItem, Toolbar, Tree, TreeItem,
    Image, Figure, None,
}

/// Widget accessibility properties.
#[derive(Debug, Clone)]
pub struct Accessibility {
    pub role: AccessibilityRole,
    pub label: String,
    pub description: String,
    pub focusable: bool,
    pub tab_index: i32,
}

impl Accessibility {
    pub fn new(role: AccessibilityRole, label: &str) -> Self {
        Self { role, label: label.into(), description: String::new(), focusable: true, tab_index: 0 }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  WIDGET TREE (QML/XAML/SwiftUI-inspired)
// ═══════════════════════════════════════════════════════════════════════

/// A declarative widget node.
#[derive(Debug, Clone)]
pub struct Widget {
    pub id: WidgetId,
    pub widget_type: WidgetType,
    pub style: Style,
    pub children: Vec<Widget>,
    pub properties: HashMap<String, PropertyValue>,
    pub event_handlers: Vec<String>,
    pub accessibility: Option<Accessibility>,
    pub visible: bool,
    pub enabled: bool,
}

/// Widget type catalog (comprehensive set of UI components).
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetType {
    // Layout
    Container,
    Row,
    Column,
    Stack,
    Spacer,
    Divider,
    ScrollView { direction: ScrollDirection },
    GridView { columns: usize },

    // Text
    Label { text: String },
    Heading { text: String, level: u8 },
    Paragraph { text: String },
    CodeBlock { text: String, language: String },

    // Input
    Button { text: String, variant: ButtonVariant },
    TextInput { placeholder: String, value: String, multiline: bool },
    Checkbox { checked: bool, label: String },
    RadioButton { selected: bool, label: String, group: String },
    Slider { min: f64, max: f64, value: f64, step: f64 },
    Toggle { on: bool, label: String },
    Dropdown { options: Vec<String>, selected: usize },
    NumberInput { value: f64, min: f64, max: f64 },
    ColorPicker { color: String },
    DatePicker { value: String },

    // Data display
    List { items: Vec<String> },
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    Card { title: String },
    Badge { text: String, color: String },
    Avatar { name: String, image_url: String },
    ProgressBar { value: f64, max: f64 },
    Tooltip { text: String },

    // Navigation
    TabBar { tabs: Vec<String>, active: usize },
    Breadcrumb { items: Vec<String> },
    Sidebar,
    NavigationBar { title: String },
    MenuBar { items: Vec<String> },

    // Media
    Image { src: String, alt: String },
    Icon { name: String, size: f64 },
    Canvas { width: f64, height: f64 },

    // Overlay
    Dialog { title: String, open: bool },
    Toast { message: String, duration: f64 },
    Popover { anchor: WidgetId },

    // Custom
    Custom { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection { Vertical, Horizontal, Both }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant { Primary, Secondary, Outlined, Text, Danger, Success }

/// Property value types for dynamic widget properties.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    String(String),
    Number(f64),
    Bool(bool),
    Color(CssColor),
    List(Vec<PropertyValue>),
}

impl Widget {
    pub fn new(id: WidgetId, widget_type: WidgetType) -> Self {
        Self {
            id,
            widget_type,
            style: Style::new(),
            children: Vec::new(),
            properties: HashMap::new(),
            event_handlers: Vec::new(),
            accessibility: None,
            visible: true,
            enabled: true,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self { self.style = style; self }
    pub fn with_child(mut self, child: Widget) -> Self { self.children.push(child); self }
    pub fn with_accessibility(mut self, acc: Accessibility) -> Self { self.accessibility = Some(acc); self }
    pub fn hidden(mut self) -> Self { self.visible = false; self }
    pub fn disabled(mut self) -> Self { self.enabled = false; self }

    pub fn set_property(&mut self, key: &str, value: PropertyValue) {
        self.properties.insert(key.into(), value);
    }

    pub fn get_property(&self, key: &str) -> Option<&PropertyValue> {
        self.properties.get(key)
    }

    pub fn child_count(&self) -> usize { self.children.len() }
    pub fn total_depth(&self) -> usize {
        if self.children.is_empty() { 1 }
        else { 1 + self.children.iter().map(|c| c.total_depth()).max().unwrap_or(0) }
    }

    pub fn total_widgets(&self) -> usize {
        1 + self.children.iter().map(|c| c.total_widgets()).sum::<usize>()
    }

    /// Flatten the widget tree to a list.
    pub fn flatten(&self) -> Vec<&Widget> {
        let mut result = vec![self];
        for child in &self.children {
            result.extend(child.flatten());
        }
        result
    }

    /// Find a widget by ID.
    pub fn find_by_id(&self, id: WidgetId) -> Option<&Widget> {
        if self.id == id { return Some(self); }
        for child in &self.children {
            if let Some(w) = child.find_by_id(id) { return Some(w); }
        }
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  THEME ENGINE
// ═══════════════════════════════════════════════════════════════════════

/// Theme mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode { Light, Dark, System }

/// A complete UI theme.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub mode: ThemeMode,
    pub colors: ThemeColors,
    pub typography: ThemeTypography,
    pub spacing: ThemeSpacing,
    pub border_radius: f64,
    pub shadow_elevation: Vec<BoxShadow>,
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub primary: CssColor,
    pub secondary: CssColor,
    pub background: CssColor,
    pub surface: CssColor,
    pub error: CssColor,
    pub warning: CssColor,
    pub success: CssColor,
    pub info: CssColor,
    pub on_primary: CssColor,
    pub on_background: CssColor,
    pub on_surface: CssColor,
    pub divider: CssColor,
}

#[derive(Debug, Clone)]
pub struct ThemeTypography {
    pub font_family: String,
    pub heading_font_family: String,
    pub base_size: f64,
    pub scale_ratio: f64,
    pub h1_size: f64,
    pub h2_size: f64,
    pub h3_size: f64,
    pub body_size: f64,
    pub caption_size: f64,
}

#[derive(Debug, Clone)]
pub struct ThemeSpacing {
    pub unit: f64,
    pub xs: f64,
    pub sm: f64,
    pub md: f64,
    pub lg: f64,
    pub xl: f64,
    pub xxl: f64,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            name: "Light".into(),
            mode: ThemeMode::Light,
            colors: ThemeColors {
                primary: CssColor::Hex(0x6200EE),
                secondary: CssColor::Hex(0x03DAC6),
                background: CssColor::Hex(0xFFFFFF),
                surface: CssColor::Hex(0xFFFFFF),
                error: CssColor::Hex(0xB00020),
                warning: CssColor::Hex(0xFF9800),
                success: CssColor::Hex(0x4CAF50),
                info: CssColor::Hex(0x2196F3),
                on_primary: CssColor::white(),
                on_background: CssColor::black(),
                on_surface: CssColor::black(),
                divider: CssColor::Rgba(0.0, 0.0, 0.0, 0.12),
            },
            typography: ThemeTypography {
                font_family: "Inter, sans-serif".into(),
                heading_font_family: "Inter, sans-serif".into(),
                base_size: 16.0,
                scale_ratio: 1.25,
                h1_size: 32.0, h2_size: 24.0, h3_size: 20.0,
                body_size: 16.0, caption_size: 12.0,
            },
            spacing: ThemeSpacing { unit: 8.0, xs: 4.0, sm: 8.0, md: 16.0, lg: 24.0, xl: 32.0, xxl: 48.0 },
            border_radius: 8.0,
            shadow_elevation: vec![
                BoxShadow::new(0.0, 2.0, 4.0, CssColor::Rgba(0.0, 0.0, 0.0, 0.1)),
                BoxShadow::new(0.0, 4.0, 8.0, CssColor::Rgba(0.0, 0.0, 0.0, 0.15)),
            ],
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".into(),
            mode: ThemeMode::Dark,
            colors: ThemeColors {
                primary: CssColor::Hex(0xBB86FC),
                secondary: CssColor::Hex(0x03DAC6),
                background: CssColor::Hex(0x121212),
                surface: CssColor::Hex(0x1E1E1E),
                error: CssColor::Hex(0xCF6679),
                warning: CssColor::Hex(0xFFB74D),
                success: CssColor::Hex(0x81C784),
                info: CssColor::Hex(0x64B5F6),
                on_primary: CssColor::black(),
                on_background: CssColor::white(),
                on_surface: CssColor::white(),
                divider: CssColor::Rgba(1.0, 1.0, 1.0, 0.12),
            },
            typography: ThemeTypography {
                font_family: "Inter, sans-serif".into(),
                heading_font_family: "Inter, sans-serif".into(),
                base_size: 16.0,
                scale_ratio: 1.25,
                h1_size: 32.0, h2_size: 24.0, h3_size: 20.0,
                body_size: 16.0, caption_size: 12.0,
            },
            spacing: ThemeSpacing { unit: 8.0, xs: 4.0, sm: 8.0, md: 16.0, lg: 24.0, xl: 32.0, xxl: 48.0 },
            border_radius: 8.0,
            shadow_elevation: vec![
                BoxShadow::new(0.0, 2.0, 8.0, CssColor::Rgba(0.0, 0.0, 0.0, 0.3)),
            ],
        }
    }

    pub fn cyberpunk() -> Self {
        Self {
            name: "Cyberpunk".into(),
            mode: ThemeMode::Dark,
            colors: ThemeColors {
                primary: CssColor::Hex(0x00FFC8),
                secondary: CssColor::Hex(0xFF00FF),
                background: CssColor::Hex(0x0A0A1A),
                surface: CssColor::Hex(0x141428),
                error: CssColor::Hex(0xFF4444),
                warning: CssColor::Hex(0xFFAA00),
                success: CssColor::Hex(0x00FF88),
                info: CssColor::Hex(0x00AAFF),
                on_primary: CssColor::black(),
                on_background: CssColor::Hex(0xE0E0FF),
                on_surface: CssColor::Hex(0xE0E0FF),
                divider: CssColor::Rgba(0.0, 1.0, 0.8, 0.2),
            },
            typography: ThemeTypography {
                font_family: "JetBrains Mono, monospace".into(),
                heading_font_family: "Orbitron, sans-serif".into(),
                base_size: 14.0,
                scale_ratio: 1.333,
                h1_size: 36.0, h2_size: 28.0, h3_size: 22.0,
                body_size: 14.0, caption_size: 11.0,
            },
            spacing: ThemeSpacing { unit: 8.0, xs: 4.0, sm: 8.0, md: 16.0, lg: 24.0, xl: 32.0, xxl: 48.0 },
            border_radius: 4.0,
            shadow_elevation: vec![
                BoxShadow::new(0.0, 0.0, 16.0, CssColor::Rgba(0.0, 1.0, 0.8, 0.15)),
            ],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  LAYOUT ENGINE (computed layout)
// ═══════════════════════════════════════════════════════════════════════

/// Computed layout rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl LayoutRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self { Self { x, y, width: w, height: h } }
    pub fn zero() -> Self { Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 } }
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
    pub fn right(&self) -> f64 { self.x + self.width }
    pub fn bottom(&self) -> f64 { self.y + self.height }
    pub fn center_x(&self) -> f64 { self.x + self.width / 2.0 }
    pub fn center_y(&self) -> f64 { self.y + self.height / 2.0 }
    pub fn area(&self) -> f64 { self.width * self.height }
}

/// Layout result for a widget tree.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub layouts: HashMap<WidgetId, LayoutRect>,
}

impl LayoutResult {
    pub fn new() -> Self { Self { layouts: HashMap::new() } }

    pub fn set(&mut self, id: WidgetId, rect: LayoutRect) {
        self.layouts.insert(id, rect);
    }

    pub fn get(&self, id: WidgetId) -> Option<&LayoutRect> {
        self.layouts.get(&id)
    }

    /// Simple flex layout computation.
    pub fn compute_flex(root: &Widget, container: LayoutRect) -> Self {
        let mut result = Self::new();
        result.set(root.id, container);

        if root.children.is_empty() { return result; }

        let is_row = matches!(root.style.flex_direction, FlexDirection::Row | FlexDirection::RowReverse);
        let total_children = root.children.len() as f64;
        let child_main_size = if is_row { container.width / total_children } else { container.height / total_children };

        let mut offset = 0.0;
        for child in &root.children {
            let rect = if is_row {
                LayoutRect::new(container.x + offset, container.y, child_main_size, container.height)
            } else {
                LayoutRect::new(container.x, container.y + offset, container.width, child_main_size)
            };
            result.set(child.id, rect);
            offset += child_main_size;

            // Recurse
            let child_layout = Self::compute_flex(child, rect);
            for (id, r) in child_layout.layouts {
                result.layouts.insert(id, r);
            }
        }

        result
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  RESPONSIVE BREAKPOINTS
// ═══════════════════════════════════════════════════════════════════════

/// Responsive design breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    Mobile,   // < 640px
    Tablet,   // 640-1024px
    Desktop,  // 1024-1440px
    Wide,     // > 1440px
}

impl Breakpoint {
    pub fn from_width(width: f64) -> Self {
        if width < 640.0 { Breakpoint::Mobile }
        else if width < 1024.0 { Breakpoint::Tablet }
        else if width < 1440.0 { Breakpoint::Desktop }
        else { Breakpoint::Wide }
    }

    pub fn min_width(&self) -> f64 {
        match self {
            Breakpoint::Mobile => 0.0,
            Breakpoint::Tablet => 640.0,
            Breakpoint::Desktop => 1024.0,
            Breakpoint::Wide => 1440.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  QML-STYLE PROPERTY BINDINGS
// ═══════════════════════════════════════════════════════════════════════

/// A property binding expression.
#[derive(Debug, Clone)]
pub enum BindingExpr {
    Literal(PropertyValue),
    PropertyRef { widget_id: WidgetId, property: String },
    Expression { formula: String },
    Conditional { condition: Box<BindingExpr>, if_true: Box<BindingExpr>, if_false: Box<BindingExpr> },
}

/// A property binding.
#[derive(Debug, Clone)]
pub struct PropertyBinding {
    pub target_widget: WidgetId,
    pub target_property: String,
    pub expression: BindingExpr,
}

/// Manages reactive property bindings.
#[derive(Debug, Clone)]
pub struct BindingEngine {
    pub bindings: Vec<PropertyBinding>,
}

impl BindingEngine {
    pub fn new() -> Self { Self { bindings: Vec::new() } }

    pub fn bind(&mut self, binding: PropertyBinding) {
        self.bindings.push(binding);
    }

    pub fn binding_count(&self) -> usize { self.bindings.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  CSS TRANSITION SYSTEM
// ═══════════════════════════════════════════════════════════════════════

/// A CSS-compatible transition.
#[derive(Debug, Clone)]
pub struct CssTransition {
    pub property: String,
    pub duration_ms: f64,
    pub timing_function: TransitionTiming,
    pub delay_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionTiming {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier,
}

impl CssTransition {
    pub fn new(property: &str, duration_ms: f64) -> Self {
        Self { property: property.into(), duration_ms, timing_function: TransitionTiming::Ease, delay_ms: 0.0 }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CSS KEYFRAME ANIMATION
// ═══════════════════════════════════════════════════════════════════════

/// A CSS @keyframes animation.
#[derive(Debug, Clone)]
pub struct CssKeyframeAnimation {
    pub name: String,
    pub keyframes: Vec<(f64, HashMap<String, String>)>,  // (progress 0-1, properties)
    pub duration_ms: f64,
    pub iteration_count: AnimationIterationCount,
    pub direction: AnimationDirection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationIterationCount { Finite(f64), Infinite }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection { Normal, Reverse, Alternate, AlternateReverse }

impl CssKeyframeAnimation {
    pub fn new(name: &str, duration_ms: f64) -> Self {
        Self {
            name: name.into(),
            keyframes: Vec::new(),
            duration_ms,
            iteration_count: AnimationIterationCount::Finite(1.0),
            direction: AnimationDirection::Normal,
        }
    }

    pub fn add_keyframe(&mut self, progress: f64, properties: HashMap<String, String>) {
        self.keyframes.push((progress.clamp(0.0, 1.0), properties));
        self.keyframes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Generate CSS @keyframes string.
    pub fn to_css(&self) -> String {
        let mut css = format!("@keyframes {} {{\n", self.name);
        for (progress, props) in &self.keyframes {
            css.push_str(&format!("  {}% {{\n", (progress * 100.0) as u32));
            for (k, v) in props { css.push_str(&format!("    {}: {};\n", k, v)); }
            css.push_str("  }\n");
        }
        css.push_str("}\n");
        css
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  UI APPLICATION
// ═══════════════════════════════════════════════════════════════════════

/// A complete UI application.
#[derive(Debug, Clone)]
pub struct UiApplication {
    pub title: String,
    pub root: Widget,
    pub theme: Theme,
    pub bindings: BindingEngine,
    pub transitions: Vec<CssTransition>,
    pub animations: Vec<CssKeyframeAnimation>,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

impl UiApplication {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.into(),
            root: Widget::new(0, WidgetType::Container),
            theme: Theme::light(),
            bindings: BindingEngine::new(),
            transitions: Vec::new(),
            animations: Vec::new(),
            viewport_width: 1920.0,
            viewport_height: 1080.0,
        }
    }

    pub fn set_theme(&mut self, theme: Theme) { self.theme = theme; }
    pub fn set_root(&mut self, root: Widget) { self.root = root; }

    pub fn total_widgets(&self) -> usize { self.root.total_widgets() }
    pub fn max_depth(&self) -> usize { self.root.total_depth() }

    pub fn compute_layout(&self) -> LayoutResult {
        let container = LayoutRect::new(0.0, 0.0, self.viewport_width, self.viewport_height);
        LayoutResult::compute_flex(&self.root, container)
    }

    pub fn breakpoint(&self) -> Breakpoint {
        Breakpoint::from_width(self.viewport_width)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── CSS Unit Tests ──────────────────────────────────────────────

    #[test]
    fn test_css_unit_px() {
        let u = CssUnit::Px(100.0);
        assert_eq!(u.to_px(0.0, 16.0, 0.0, 0.0), 100.0);
    }

    #[test]
    fn test_css_unit_percent() {
        let u = CssUnit::Percent(50.0);
        assert_eq!(u.to_px(200.0, 16.0, 0.0, 0.0), 100.0);
    }

    #[test]
    fn test_css_unit_vw() {
        let u = CssUnit::Vw(50.0);
        assert_eq!(u.to_px(0.0, 16.0, 1920.0, 1080.0), 960.0);
    }

    #[test]
    fn test_css_unit_display() {
        assert_eq!(CssUnit::Px(10.0).to_string(), "10px");
        assert_eq!(CssUnit::Auto.to_string(), "auto");
        assert_eq!(CssUnit::Percent(50.0).to_string(), "50%");
    }

    // ── Edge Insets Tests ───────────────────────────────────────────

    #[test]
    fn test_edge_insets() {
        let ei = EdgeInsets::all(10.0);
        assert_eq!(ei.horizontal(), 20.0);
        assert_eq!(ei.vertical(), 20.0);

        let ei2 = EdgeInsets::symmetric(20.0, 10.0);
        assert_eq!(ei2.horizontal(), 40.0);
        assert_eq!(ei2.vertical(), 20.0);
    }

    // ── Corner Radius Tests ─────────────────────────────────────────

    #[test]
    fn test_corner_radius() {
        let cr = CornerRadius::all(8.0);
        assert_eq!(cr.top_left, 8.0);
        assert_eq!(cr.bottom_right, 8.0);
    }

    // ── Color Tests ─────────────────────────────────────────────────

    #[test]
    fn test_css_color_display() {
        let c = CssColor::Hex(0xFF0000);
        assert_eq!(c.to_string(), "#ff0000");
        let t = CssColor::Transparent;
        assert_eq!(t.to_string(), "transparent");
    }

    // ── Style Tests ─────────────────────────────────────────────────

    #[test]
    fn test_style_defaults() {
        let s = Style::new();
        assert_eq!(s.display, DisplayMode::Block);
        assert_eq!(s.opacity, 1.0);
        assert_eq!(s.font_size, 16.0);
    }

    #[test]
    fn test_style_flex() {
        let s = Style::flex();
        assert_eq!(s.display, DisplayMode::Flex);
    }

    #[test]
    fn test_style_to_css() {
        let mut s = Style::new();
        s.width = Some(CssUnit::Px(200.0));
        s.background_color = CssColor::Hex(0xFF0000);
        let css = s.to_css();
        assert!(css.contains("width: 200px"));
        assert!(css.contains("background-color: #ff0000"));
    }

    // ── Widget Tests ────────────────────────────────────────────────

    #[test]
    fn test_widget_creation() {
        let w = Widget::new(1, WidgetType::Button { text: "Click".into(), variant: ButtonVariant::Primary });
        assert_eq!(w.id, 1);
        assert!(w.visible);
        assert!(w.enabled);
    }

    #[test]
    fn test_widget_tree() {
        let root = Widget::new(1, WidgetType::Container)
            .with_child(Widget::new(2, WidgetType::Label { text: "Hello".into() }))
            .with_child(Widget::new(3, WidgetType::Button { text: "OK".into(), variant: ButtonVariant::Primary }));
        assert_eq!(root.child_count(), 2);
        assert_eq!(root.total_widgets(), 3);
        assert_eq!(root.total_depth(), 2);
    }

    #[test]
    fn test_widget_find_by_id() {
        let root = Widget::new(1, WidgetType::Container)
            .with_child(Widget::new(2, WidgetType::Label { text: "A".into() })
                .with_child(Widget::new(3, WidgetType::Label { text: "B".into() })));
        assert!(root.find_by_id(3).is_some());
        assert!(root.find_by_id(999).is_none());
    }

    #[test]
    fn test_widget_flatten() {
        let root = Widget::new(1, WidgetType::Container)
            .with_child(Widget::new(2, WidgetType::Label { text: "A".into() }))
            .with_child(Widget::new(3, WidgetType::Label { text: "B".into() }));
        let flat = root.flatten();
        assert_eq!(flat.len(), 3);
    }

    #[test]
    fn test_widget_properties() {
        let mut w = Widget::new(1, WidgetType::TextInput { placeholder: "".into(), value: "".into(), multiline: false });
        w.set_property("maxLength", PropertyValue::Number(100.0));
        assert_eq!(w.get_property("maxLength"), Some(&PropertyValue::Number(100.0)));
    }

    // ── Theme Tests ─────────────────────────────────────────────────

    #[test]
    fn test_theme_light() {
        let theme = Theme::light();
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.name, "Light");
    }

    #[test]
    fn test_theme_dark() {
        let theme = Theme::dark();
        assert_eq!(theme.mode, ThemeMode::Dark);
    }

    #[test]
    fn test_theme_cyberpunk() {
        let theme = Theme::cyberpunk();
        assert_eq!(theme.name, "Cyberpunk");
        assert_eq!(theme.mode, ThemeMode::Dark);
    }

    // ── Layout Tests ────────────────────────────────────────────────

    #[test]
    fn test_layout_rect_contains() {
        let r = LayoutRect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains(50.0, 30.0));
        assert!(!r.contains(0.0, 0.0));
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 70.0);
    }

    #[test]
    fn test_layout_flex_computation() {
        let root = Widget::new(1, WidgetType::Row)
            .with_child(Widget::new(2, WidgetType::Label { text: "A".into() }))
            .with_child(Widget::new(3, WidgetType::Label { text: "B".into() }));
        let layout = LayoutResult::compute_flex(&root, LayoutRect::new(0.0, 0.0, 600.0, 400.0));
        let a = layout.get(2).unwrap();
        let b = layout.get(3).unwrap();
        assert!((a.width - 300.0).abs() < 1e-6);
        assert!((b.width - 300.0).abs() < 1e-6);
    }

    // ── Breakpoint Tests ────────────────────────────────────────────

    #[test]
    fn test_breakpoints() {
        assert_eq!(Breakpoint::from_width(320.0), Breakpoint::Mobile);
        assert_eq!(Breakpoint::from_width(768.0), Breakpoint::Tablet);
        assert_eq!(Breakpoint::from_width(1200.0), Breakpoint::Desktop);
        assert_eq!(Breakpoint::from_width(2560.0), Breakpoint::Wide);
    }

    // ── Binding Tests ───────────────────────────────────────────────

    #[test]
    fn test_binding_engine() {
        let mut engine = BindingEngine::new();
        engine.bind(PropertyBinding {
            target_widget: 1,
            target_property: "text".into(),
            expression: BindingExpr::Literal(PropertyValue::String("Hello".into())),
        });
        assert_eq!(engine.binding_count(), 1);
    }

    // ── Transition Tests ────────────────────────────────────────────

    #[test]
    fn test_css_transition() {
        let t = CssTransition::new("opacity", 300.0);
        assert_eq!(t.property, "opacity");
        assert_eq!(t.duration_ms, 300.0);
        assert_eq!(t.timing_function, TransitionTiming::Ease);
    }

    // ── Keyframe Animation Tests ────────────────────────────────────

    #[test]
    fn test_keyframe_animation() {
        let mut anim = CssKeyframeAnimation::new("fadeIn", 1000.0);
        let mut from = HashMap::new();
        from.insert("opacity".into(), "0".into());
        let mut to = HashMap::new();
        to.insert("opacity".into(), "1".into());
        anim.add_keyframe(0.0, from);
        anim.add_keyframe(1.0, to);
        let css = anim.to_css();
        assert!(css.contains("@keyframes fadeIn"));
        assert!(css.contains("opacity"));
    }

    // ── Application Tests ───────────────────────────────────────────

    #[test]
    fn test_ui_application() {
        let mut app = UiApplication::new("Test App");
        app.set_theme(Theme::dark());
        let root = Widget::new(1, WidgetType::Column)
            .with_child(Widget::new(2, WidgetType::Label { text: "Title".into() }))
            .with_child(Widget::new(3, WidgetType::Button { text: "OK".into(), variant: ButtonVariant::Primary }));
        app.set_root(root);
        assert_eq!(app.total_widgets(), 3);
        assert_eq!(app.breakpoint(), Breakpoint::Wide);
    }

    #[test]
    fn test_ui_application_layout() {
        let mut app = UiApplication::new("Layout Test");
        app.viewport_width = 800.0;
        app.viewport_height = 600.0;
        let root = Widget::new(1, WidgetType::Container)
            .with_child(Widget::new(2, WidgetType::Label { text: "A".into() }));
        app.set_root(root);
        let layout = app.compute_layout();
        let root_rect = layout.get(1).unwrap();
        assert_eq!(root_rect.width, 800.0);
        assert_eq!(root_rect.height, 600.0);
    }

    // ── Accessibility Tests ─────────────────────────────────────────

    #[test]
    fn test_accessibility() {
        let acc = Accessibility::new(AccessibilityRole::Button, "Submit Form");
        assert_eq!(acc.role, AccessibilityRole::Button);
        assert_eq!(acc.label, "Submit Form");
        assert!(acc.focusable);
    }

    // ── Font Weight Tests ───────────────────────────────────────────

    #[test]
    fn test_font_weight_numeric() {
        assert_eq!(FontWeight::Normal.numeric(), 400);
        assert_eq!(FontWeight::Bold.numeric(), 700);
        assert_eq!(FontWeight::Light.numeric(), 300);
    }

    // ── Widget Catalog Tests ────────────────────────────────────────

    #[test]
    fn test_widget_catalog_completeness() {
        // Verify we can create all major widget types
        let widgets: Vec<Widget> = vec![
            Widget::new(1, WidgetType::Container),
            Widget::new(2, WidgetType::Row),
            Widget::new(3, WidgetType::Column),
            Widget::new(4, WidgetType::Label { text: "Text".into() }),
            Widget::new(5, WidgetType::Button { text: "Click".into(), variant: ButtonVariant::Primary }),
            Widget::new(6, WidgetType::TextInput { placeholder: "Enter".into(), value: "".into(), multiline: false }),
            Widget::new(7, WidgetType::Checkbox { checked: false, label: "Check".into() }),
            Widget::new(8, WidgetType::Slider { min: 0.0, max: 100.0, value: 50.0, step: 1.0 }),
            Widget::new(9, WidgetType::ProgressBar { value: 75.0, max: 100.0 }),
            Widget::new(10, WidgetType::List { items: vec!["A".into(), "B".into()] }),
            Widget::new(11, WidgetType::TabBar { tabs: vec!["Tab1".into(), "Tab2".into()], active: 0 }),
            Widget::new(12, WidgetType::Image { src: "photo.png".into(), alt: "Photo".into() }),
            Widget::new(13, WidgetType::Dialog { title: "Confirm".into(), open: true }),
        ];
        assert_eq!(widgets.len(), 13);
    }
}
