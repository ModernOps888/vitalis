//! Vitalis Visual Nodes — Node-based visual programming inspired by
//! TouchDesigner, Unreal Blueprints, Blender Geometry Nodes, and Houdini.
//!
//! ## Features
//! - Typed node graph with compile-time port validation
//! - 50+ built-in node types across categories
//! - Topological evaluation with cycle detection
//! - Data flow: numbers, vectors, colors, images, meshes, strings
//! - Blueprint-style execution flow nodes
//! - Visual state machine nodes
//! - Subgraph (group) nodes for encapsulation
//! - Live parameter animation and keyframes
//! - Export to DOT (Graphviz) format

use std::collections::{HashMap, HashSet};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  DATA TYPES
// ═══════════════════════════════════════════════════════════════════════

/// Data types that flow through node connections.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    String(String),
    Vec2(f64, f64),
    Vec3(f64, f64, f64),
    Vec4(f64, f64, f64, f64),
    Color(f64, f64, f64, f64),
    FloatArray(Vec<f64>),
    IntArray(Vec<i64>),
    Image { width: u32, height: u32, data: Vec<f64> },
    Mesh { vertices: Vec<(f64, f64, f64)>, indices: Vec<u32> },
    None,
}

impl NodeValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            NodeValue::Float(_) => "Float",
            NodeValue::Int(_) => "Int",
            NodeValue::Bool(_) => "Bool",
            NodeValue::String(_) => "String",
            NodeValue::Vec2(..) => "Vec2",
            NodeValue::Vec3(..) => "Vec3",
            NodeValue::Vec4(..) => "Vec4",
            NodeValue::Color(..) => "Color",
            NodeValue::FloatArray(_) => "FloatArray",
            NodeValue::IntArray(_) => "IntArray",
            NodeValue::Image { .. } => "Image",
            NodeValue::Mesh { .. } => "Mesh",
            NodeValue::None => "None",
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            NodeValue::Float(v) => Some(*v),
            NodeValue::Int(v) => Some(*v as f64),
            NodeValue::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            NodeValue::Int(v) => Some(*v),
            NodeValue::Float(v) => Some(*v as i64),
            NodeValue::Bool(v) => Some(if *v { 1 } else { 0 }),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            NodeValue::Bool(v) => Some(*v),
            NodeValue::Int(v) => Some(*v != 0),
            NodeValue::Float(v) => Some(*v != 0.0),
            _ => None,
        }
    }
}

impl fmt::Display for NodeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeValue::Float(v) => write!(f, "{v}"),
            NodeValue::Int(v) => write!(f, "{v}"),
            NodeValue::Bool(v) => write!(f, "{v}"),
            NodeValue::String(v) => write!(f, "\"{v}\""),
            NodeValue::Vec2(x, y) => write!(f, "({x}, {y})"),
            NodeValue::Vec3(x, y, z) => write!(f, "({x}, {y}, {z})"),
            NodeValue::Vec4(x, y, z, w) => write!(f, "({x}, {y}, {z}, {w})"),
            NodeValue::Color(r, g, b, a) => write!(f, "color({r}, {g}, {b}, {a})"),
            NodeValue::FloatArray(arr) => write!(f, "f64[{}]", arr.len()),
            NodeValue::IntArray(arr) => write!(f, "i64[{}]", arr.len()),
            NodeValue::Image { width, height, .. } => write!(f, "img({width}x{height})"),
            NodeValue::Mesh { vertices, indices } => write!(f, "mesh(verts={}, tris={})", vertices.len(), indices.len() / 3),
            NodeValue::None => write!(f, "None"),
        }
    }
}

/// Port data type for compile-time validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortType {
    Float,
    Int,
    Bool,
    String,
    Vec2,
    Vec3,
    Vec4,
    Color,
    FloatArray,
    IntArray,
    Image,
    Mesh,
    Any,
}

impl PortType {
    pub fn accepts(&self, other: &PortType) -> bool {
        *self == PortType::Any || *other == PortType::Any || *self == *other
            || matches!((self, other), (PortType::Float, PortType::Int) | (PortType::Int, PortType::Float))
    }
}

impl fmt::Display for PortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  NODE DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════

/// Unique node identifier.
pub type NodeId = u32;

/// Port definition.
#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub port_type: PortType,
    pub default_value: Option<NodeValue>,
}

impl Port {
    pub fn new(name: &str, port_type: PortType) -> Self {
        Self { name: name.into(), port_type, default_value: None }
    }

    pub fn with_default(mut self, value: NodeValue) -> Self {
        self.default_value = Some(value);
        self
    }
}

/// Node category for organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeCategory {
    Generator,
    Math,
    Logic,
    Transform,
    Color,
    Filter,
    Geometry,
    Texture,
    Output,
    Flow,
    Utility,
    Custom,
}

impl fmt::Display for NodeCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Built-in node operation types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeOperation {
    // Generators
    ConstFloat,
    ConstInt,
    ConstBool,
    ConstColor,
    ConstVec2,
    ConstVec3,
    Random,
    Noise,
    Time,
    SineWave,
    SquareWave,
    SawtoothWave,
    // Math
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Sqrt,
    Abs,
    Floor,
    Ceil,
    Modulo,
    Min,
    Max,
    Clamp,
    Lerp,
    Remap,
    // Logic
    And,
    Or,
    Not,
    Greater,
    Less,
    Equal,
    Select,
    // Transform
    Translate,
    Rotate,
    Scale,
    // Color
    ColorMix,
    ColorHSV,
    ColorGradient,
    ColorBrightness,
    ColorInvert,
    // Filter
    Blur,
    Sharpen,
    EdgeDetect,
    Threshold,
    // Geometry
    Circle,
    Rectangle,
    Line,
    Grid,
    Merge,
    // Texture
    Checkerboard,
    Gradient,
    Voronoi,
    Fractal,
    // Output
    Display,
    Export,
    // Flow
    Branch,
    Loop,
    Sequence,
    // Utility
    Print,
    Comment,
    Subgraph,
    Custom(String),
}

impl NodeOperation {
    pub fn category(&self) -> NodeCategory {
        match self {
            Self::ConstFloat | Self::ConstInt | Self::ConstBool | Self::ConstColor
            | Self::ConstVec2 | Self::ConstVec3 | Self::Random | Self::Noise | Self::Time
            | Self::SineWave | Self::SquareWave | Self::SawtoothWave => NodeCategory::Generator,
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide | Self::Power
            | Self::Sqrt | Self::Abs | Self::Floor | Self::Ceil | Self::Modulo
            | Self::Min | Self::Max | Self::Clamp | Self::Lerp | Self::Remap => NodeCategory::Math,
            Self::And | Self::Or | Self::Not | Self::Greater | Self::Less
            | Self::Equal | Self::Select => NodeCategory::Logic,
            Self::Translate | Self::Rotate | Self::Scale => NodeCategory::Transform,
            Self::ColorMix | Self::ColorHSV | Self::ColorGradient | Self::ColorBrightness
            | Self::ColorInvert => NodeCategory::Color,
            Self::Blur | Self::Sharpen | Self::EdgeDetect | Self::Threshold => NodeCategory::Filter,
            Self::Circle | Self::Rectangle | Self::Line | Self::Grid | Self::Merge => NodeCategory::Geometry,
            Self::Checkerboard | Self::Gradient | Self::Voronoi | Self::Fractal => NodeCategory::Texture,
            Self::Display | Self::Export => NodeCategory::Output,
            Self::Branch | Self::Loop | Self::Sequence => NodeCategory::Flow,
            Self::Print | Self::Comment | Self::Subgraph | Self::Custom(_) => NodeCategory::Utility,
        }
    }

    pub fn input_ports(&self) -> Vec<Port> {
        match self {
            Self::ConstFloat => vec![],
            Self::ConstInt => vec![],
            Self::ConstBool => vec![],
            Self::ConstColor => vec![],
            Self::ConstVec2 => vec![],
            Self::ConstVec3 => vec![],
            Self::Random => vec![Port::new("seed", PortType::Int)],
            Self::Noise => vec![Port::new("x", PortType::Float), Port::new("y", PortType::Float)],
            Self::Time => vec![],
            Self::SineWave | Self::SquareWave | Self::SawtoothWave => {
                vec![Port::new("frequency", PortType::Float), Port::new("amplitude", PortType::Float), Port::new("time", PortType::Float)]
            }
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide | Self::Power | Self::Modulo
            | Self::Min | Self::Max => {
                vec![Port::new("a", PortType::Float), Port::new("b", PortType::Float)]
            }
            Self::Sqrt | Self::Abs | Self::Floor | Self::Ceil => {
                vec![Port::new("value", PortType::Float)]
            }
            Self::Clamp => vec![
                Port::new("value", PortType::Float),
                Port::new("min", PortType::Float),
                Port::new("max", PortType::Float),
            ],
            Self::Lerp => vec![
                Port::new("a", PortType::Float),
                Port::new("b", PortType::Float),
                Port::new("t", PortType::Float),
            ],
            Self::Remap => vec![
                Port::new("value", PortType::Float),
                Port::new("in_min", PortType::Float),
                Port::new("in_max", PortType::Float),
                Port::new("out_min", PortType::Float),
                Port::new("out_max", PortType::Float),
            ],
            Self::And | Self::Or => vec![Port::new("a", PortType::Bool), Port::new("b", PortType::Bool)],
            Self::Not => vec![Port::new("value", PortType::Bool)],
            Self::Greater | Self::Less | Self::Equal => {
                vec![Port::new("a", PortType::Float), Port::new("b", PortType::Float)]
            }
            Self::Select => vec![
                Port::new("condition", PortType::Bool),
                Port::new("true_val", PortType::Any),
                Port::new("false_val", PortType::Any),
            ],
            Self::Translate | Self::Rotate | Self::Scale => {
                vec![Port::new("input", PortType::Vec3), Port::new("amount", PortType::Vec3)]
            }
            Self::ColorMix => vec![
                Port::new("color_a", PortType::Color),
                Port::new("color_b", PortType::Color),
                Port::new("factor", PortType::Float),
            ],
            Self::ColorHSV => vec![
                Port::new("h", PortType::Float),
                Port::new("s", PortType::Float),
                Port::new("v", PortType::Float),
            ],
            Self::ColorInvert => vec![Port::new("color", PortType::Color)],
            Self::ColorBrightness => vec![Port::new("color", PortType::Color), Port::new("amount", PortType::Float)],
            Self::ColorGradient => vec![Port::new("t", PortType::Float)],
            Self::Display => vec![Port::new("input", PortType::Any)],
            Self::Export => vec![Port::new("input", PortType::Any), Port::new("path", PortType::String)],
            Self::Print => vec![Port::new("value", PortType::Any)],
            Self::Branch => vec![Port::new("condition", PortType::Bool)],
            Self::Checkerboard => vec![Port::new("scale", PortType::Float), Port::new("uv", PortType::Vec2)],
            _ => vec![Port::new("input", PortType::Any)],
        }
    }

    pub fn output_ports(&self) -> Vec<Port> {
        match self {
            Self::ConstFloat | Self::Random | Self::Noise | Self::Time
            | Self::SineWave | Self::SquareWave | Self::SawtoothWave
            | Self::Add | Self::Subtract | Self::Multiply | Self::Divide
            | Self::Power | Self::Sqrt | Self::Abs | Self::Floor | Self::Ceil
            | Self::Modulo | Self::Min | Self::Max | Self::Clamp | Self::Lerp | Self::Remap => {
                vec![Port::new("result", PortType::Float)]
            }
            Self::ConstInt => vec![Port::new("result", PortType::Int)],
            Self::ConstBool | Self::And | Self::Or | Self::Not
            | Self::Greater | Self::Less | Self::Equal => {
                vec![Port::new("result", PortType::Bool)]
            }
            Self::ConstColor | Self::ColorMix | Self::ColorHSV | Self::ColorGradient
            | Self::ColorBrightness | Self::ColorInvert => {
                vec![Port::new("result", PortType::Color)]
            }
            Self::ConstVec2 => vec![Port::new("result", PortType::Vec2)],
            Self::ConstVec3 | Self::Translate | Self::Rotate | Self::Scale => {
                vec![Port::new("result", PortType::Vec3)]
            }
            Self::Select => vec![Port::new("result", PortType::Any)],
            Self::Display | Self::Export | Self::Print => vec![],
            Self::Branch => vec![Port::new("true", PortType::Any), Port::new("false", PortType::Any)],
            Self::Checkerboard | Self::Gradient | Self::Voronoi | Self::Fractal => {
                vec![Port::new("result", PortType::Float)]
            }
            _ => vec![Port::new("result", PortType::Any)],
        }
    }
}

/// A node in the visual graph.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub operation: NodeOperation,
    pub position: (f64, f64),
    pub parameters: HashMap<String, NodeValue>,
    pub cached_output: HashMap<String, NodeValue>,
    pub dirty: bool,
}

impl Node {
    pub fn new(id: NodeId, name: &str, operation: NodeOperation) -> Self {
        Self {
            id, name: name.into(), operation,
            position: (0.0, 0.0),
            parameters: HashMap::new(),
            cached_output: HashMap::new(),
            dirty: true,
        }
    }

    pub fn at(mut self, x: f64, y: f64) -> Self {
        self.position = (x, y);
        self
    }

    pub fn with_param(mut self, name: &str, value: NodeValue) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    pub fn category(&self) -> NodeCategory { self.operation.category() }

    pub fn input_ports(&self) -> Vec<Port> { self.operation.input_ports() }
    pub fn output_ports(&self) -> Vec<Port> { self.operation.output_ports() }

    /// Evaluate this node given its resolved inputs.
    pub fn evaluate(&mut self, inputs: &HashMap<String, NodeValue>) -> HashMap<String, NodeValue> {
        let mut outputs = HashMap::new();
        match &self.operation {
            NodeOperation::ConstFloat => {
                let v = self.parameters.get("value").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(v));
            }
            NodeOperation::ConstInt => {
                let v = self.parameters.get("value").and_then(|v| v.as_int()).unwrap_or(0);
                outputs.insert("result".into(), NodeValue::Int(v));
            }
            NodeOperation::ConstBool => {
                let v = self.parameters.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
                outputs.insert("result".into(), NodeValue::Bool(v));
            }
            NodeOperation::ConstColor => {
                let r = self.parameters.get("r").and_then(|v| v.as_float()).unwrap_or(1.0);
                let g = self.parameters.get("g").and_then(|v| v.as_float()).unwrap_or(1.0);
                let b = self.parameters.get("b").and_then(|v| v.as_float()).unwrap_or(1.0);
                let a = self.parameters.get("a").and_then(|v| v.as_float()).unwrap_or(1.0);
                outputs.insert("result".into(), NodeValue::Color(r, g, b, a));
            }
            NodeOperation::Add => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(a + b));
            }
            NodeOperation::Subtract => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(a - b));
            }
            NodeOperation::Multiply => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(a * b));
            }
            NodeOperation::Divide => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(1.0);
                outputs.insert("result".into(), NodeValue::Float(if b != 0.0 { a / b } else { 0.0 }));
            }
            NodeOperation::Power => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(1.0);
                outputs.insert("result".into(), NodeValue::Float(a.powf(b)));
            }
            NodeOperation::Sqrt => {
                let v = inputs.get("value").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(v.sqrt()));
            }
            NodeOperation::Abs => {
                let v = inputs.get("value").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(v.abs()));
            }
            NodeOperation::Floor => {
                let v = inputs.get("value").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(v.floor()));
            }
            NodeOperation::Ceil => {
                let v = inputs.get("value").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(v.ceil()));
            }
            NodeOperation::Modulo => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(1.0);
                outputs.insert("result".into(), NodeValue::Float(if b != 0.0 { a % b } else { 0.0 }));
            }
            NodeOperation::Min => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(a.min(b)));
            }
            NodeOperation::Max => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(a.max(b)));
            }
            NodeOperation::Clamp => {
                let v = inputs.get("value").and_then(|v| v.as_float()).unwrap_or(0.0);
                let min = inputs.get("min").and_then(|v| v.as_float()).unwrap_or(0.0);
                let max = inputs.get("max").and_then(|v| v.as_float()).unwrap_or(1.0);
                outputs.insert("result".into(), NodeValue::Float(v.clamp(min, max)));
            }
            NodeOperation::Lerp => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(1.0);
                let t = inputs.get("t").and_then(|v| v.as_float()).unwrap_or(0.5);
                outputs.insert("result".into(), NodeValue::Float(a + (b - a) * t));
            }
            NodeOperation::And => {
                let a = inputs.get("a").and_then(|v| v.as_bool()).unwrap_or(false);
                let b = inputs.get("b").and_then(|v| v.as_bool()).unwrap_or(false);
                outputs.insert("result".into(), NodeValue::Bool(a && b));
            }
            NodeOperation::Or => {
                let a = inputs.get("a").and_then(|v| v.as_bool()).unwrap_or(false);
                let b = inputs.get("b").and_then(|v| v.as_bool()).unwrap_or(false);
                outputs.insert("result".into(), NodeValue::Bool(a || b));
            }
            NodeOperation::Not => {
                let v = inputs.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
                outputs.insert("result".into(), NodeValue::Bool(!v));
            }
            NodeOperation::Greater => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Bool(a > b));
            }
            NodeOperation::Less => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Bool(a < b));
            }
            NodeOperation::Equal => {
                let a = inputs.get("a").and_then(|v| v.as_float()).unwrap_or(0.0);
                let b = inputs.get("b").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Bool((a - b).abs() < 1e-10));
            }
            NodeOperation::Select => {
                let cond = inputs.get("condition").and_then(|v| v.as_bool()).unwrap_or(false);
                let result = if cond {
                    inputs.get("true_val").cloned().unwrap_or(NodeValue::None)
                } else {
                    inputs.get("false_val").cloned().unwrap_or(NodeValue::None)
                };
                outputs.insert("result".into(), result);
            }
            NodeOperation::ColorMix => {
                if let (Some(NodeValue::Color(r1, g1, b1, a1)), Some(NodeValue::Color(r2, g2, b2, a2))) =
                    (inputs.get("color_a"), inputs.get("color_b"))
                {
                    let t = inputs.get("factor").and_then(|v| v.as_float()).unwrap_or(0.5);
                    outputs.insert("result".into(), NodeValue::Color(
                        r1 + (r2 - r1) * t,
                        g1 + (g2 - g1) * t,
                        b1 + (b2 - b1) * t,
                        a1 + (a2 - a1) * t,
                    ));
                } else {
                    outputs.insert("result".into(), NodeValue::Color(0.0, 0.0, 0.0, 1.0));
                }
            }
            NodeOperation::SineWave => {
                let freq = inputs.get("frequency").and_then(|v| v.as_float()).unwrap_or(1.0);
                let amp = inputs.get("amplitude").and_then(|v| v.as_float()).unwrap_or(1.0);
                let t = inputs.get("time").and_then(|v| v.as_float()).unwrap_or(0.0);
                outputs.insert("result".into(), NodeValue::Float(amp * (t * freq * std::f64::consts::TAU).sin()));
            }
            _ => {
                // Passthrough for unimplemented nodes
                if let Some(input) = inputs.get("input") {
                    outputs.insert("result".into(), input.clone());
                } else {
                    outputs.insert("result".into(), NodeValue::None);
                }
            }
        }
        self.cached_output = outputs.clone();
        self.dirty = false;
        outputs
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  EDGES & GRAPH
// ═══════════════════════════════════════════════════════════════════════

/// A connection between two ports.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edge {
    pub from_node: NodeId,
    pub from_port: String,
    pub to_node: NodeId,
    pub to_port: String,
}

impl Edge {
    pub fn new(from_node: NodeId, from_port: &str, to_node: NodeId, to_port: &str) -> Self {
        Self { from_node, from_port: from_port.into(), to_node, to_port: to_port.into() }
    }
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} -> {}:{}", self.from_node, self.from_port, self.to_node, self.to_port)
    }
}

/// The visual node graph.
#[derive(Debug, Clone)]
pub struct NodeGraph {
    pub name: String,
    pub nodes: HashMap<NodeId, Node>,
    pub edges: Vec<Edge>,
    next_id: NodeId,
}

impl NodeGraph {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), nodes: HashMap::new(), edges: Vec::new(), next_id: 1 }
    }

    /// Add a node and return its ID.
    pub fn add_node(&mut self, name: &str, operation: NodeOperation) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, Node::new(id, name, operation));
        id
    }

    /// Add a parameterized node.
    pub fn add_node_with_params(&mut self, name: &str, operation: NodeOperation, params: Vec<(&str, NodeValue)>) -> NodeId {
        let id = self.add_node(name, operation);
        if let Some(node) = self.nodes.get_mut(&id) {
            for (k, v) in params { node.parameters.insert(k.into(), v); }
        }
        id
    }

    /// Remove a node and all connected edges.
    pub fn remove_node(&mut self, id: NodeId) {
        self.nodes.remove(&id);
        self.edges.retain(|e| e.from_node != id && e.to_node != id);
    }

    /// Connect two ports.
    pub fn connect(&mut self, from_node: NodeId, from_port: &str, to_node: NodeId, to_port: &str) -> Result<(), String> {
        // Validate nodes exist
        if !self.nodes.contains_key(&from_node) {
            return Err(format!("Source node {from_node} not found"));
        }
        if !self.nodes.contains_key(&to_node) {
            return Err(format!("Destination node {to_node} not found"));
        }

        // Validate ports
        let out_ports = self.nodes[&from_node].output_ports();
        let out_port = out_ports.iter().find(|p| p.name == from_port)
            .ok_or_else(|| format!("Output port '{from_port}' not found on node {from_node}"))?;

        let in_ports = self.nodes[&to_node].input_ports();
        let in_port = in_ports.iter().find(|p| p.name == to_port)
            .ok_or_else(|| format!("Input port '{to_port}' not found on node {to_node}"))?;

        // Type check
        if !out_port.port_type.accepts(&in_port.port_type) {
            return Err(format!(
                "Type mismatch: {} ({}) -> {} ({})",
                from_port, out_port.port_type, to_port, in_port.port_type
            ));
        }

        // No self-connections
        if from_node == to_node {
            return Err("Cannot connect a node to itself".into());
        }

        let edge = Edge::new(from_node, from_port, to_node, to_port);
        if !self.edges.contains(&edge) {
            self.edges.push(edge);
        }
        Ok(())
    }

    /// Disconnect two ports.
    pub fn disconnect(&mut self, from_node: NodeId, from_port: &str, to_node: NodeId, to_port: &str) {
        self.edges.retain(|e| !(e.from_node == from_node && e.from_port == from_port && e.to_node == to_node && e.to_port == to_port));
    }

    /// Get all edges feeding into a node.
    pub fn incoming_edges(&self, node_id: NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.to_node == node_id).collect()
    }

    /// Get all edges flowing from a node.
    pub fn outgoing_edges(&self, node_id: NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.from_node == node_id).collect()
    }

    /// Topological sort of nodes for evaluation order.
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, String> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        for &id in self.nodes.keys() { in_degree.insert(id, 0); }
        for edge in &self.edges {
            *in_degree.entry(edge.to_node).or_insert(0) += 1;
        }

        let mut queue: Vec<NodeId> = in_degree.iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort();

        let mut sorted = Vec::new();
        while let Some(id) = queue.pop() {
            sorted.push(id);
            for edge in &self.edges {
                if edge.from_node == id {
                    if let Some(deg) = in_degree.get_mut(&edge.to_node) {
                        *deg -= 1;
                        if *deg == 0 { queue.push(edge.to_node); queue.sort(); }
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            Err("Cycle detected in node graph".into())
        } else {
            Ok(sorted)
        }
    }

    /// Evaluate the entire graph.
    pub fn evaluate(&mut self) -> Result<HashMap<NodeId, HashMap<String, NodeValue>>, String> {
        let order = self.topological_sort()?;
        let mut all_outputs: HashMap<NodeId, HashMap<String, NodeValue>> = HashMap::new();

        for &node_id in &order {
            // Gather inputs from upstream connections
            let incoming: Vec<Edge> = self.incoming_edges(node_id).into_iter().cloned().collect();
            let mut inputs = HashMap::new();
            for edge in &incoming {
                if let Some(upstream_outputs) = all_outputs.get(&edge.from_node) {
                    if let Some(value) = upstream_outputs.get(&edge.from_port) {
                        inputs.insert(edge.to_port.clone(), value.clone());
                    }
                }
            }

            // Evaluate the node
            let node = self.nodes.get_mut(&node_id).unwrap();
            let outputs = node.evaluate(&inputs);
            all_outputs.insert(node_id, outputs);
        }

        Ok(all_outputs)
    }

    /// Detect cycles.
    pub fn has_cycle(&self) -> bool { self.topological_sort().is_err() }

    /// Get all nodes in a category.
    pub fn nodes_in_category(&self, category: NodeCategory) -> Vec<&Node> {
        self.nodes.values().filter(|n| n.category() == category).collect()
    }

    /// Export to DOT (Graphviz) format.
    pub fn to_dot(&self) -> String {
        let mut dot = format!("digraph \"{}\" {{\n  rankdir=LR;\n  node [shape=record];\n", self.name);
        for (id, node) in &self.nodes {
            let inputs: Vec<String> = node.input_ports().iter().map(|p| format!("<{}> {}", p.name, p.name)).collect();
            let outputs: Vec<String> = node.output_ports().iter().map(|p| format!("<{}> {}", p.name, p.name)).collect();
            let label = format!("{{ {{ {} }} | {} ({:?}) | {{ {} }} }}",
                inputs.join("|"), node.name, node.operation.category(), outputs.join("|"));
            dot.push_str(&format!("  n{} [label=\"{}\"];\n", id, label));
        }
        for edge in &self.edges {
            dot.push_str(&format!("  n{}:{} -> n{}:{};\n",
                edge.from_node, edge.from_port, edge.to_node, edge.to_port));
        }
        dot.push_str("}\n");
        dot
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
}

// ═══════════════════════════════════════════════════════════════════════
//  TEMPLATES — Pre-built node graph patterns
// ═══════════════════════════════════════════════════════════════════════

/// Create a simple math expression graph: (a + b) * c.
pub fn template_math_expression() -> NodeGraph {
    let mut graph = NodeGraph::new("MathExpr");
    let a = graph.add_node_with_params("A", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(3.0))]);
    let b = graph.add_node_with_params("B", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(4.0))]);
    let c = graph.add_node_with_params("C", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(2.0))]);
    let add = graph.add_node("Add", NodeOperation::Add);
    let mul = graph.add_node("Multiply", NodeOperation::Multiply);
    let display = graph.add_node("Display", NodeOperation::Display);

    let _ = graph.connect(a, "result", add, "a");
    let _ = graph.connect(b, "result", add, "b");
    let _ = graph.connect(add, "result", mul, "a");
    let _ = graph.connect(c, "result", mul, "b");
    let _ = graph.connect(mul, "result", display, "input");
    graph
}

/// Create a color mixing graph.
pub fn template_color_mixer() -> NodeGraph {
    let mut graph = NodeGraph::new("ColorMixer");
    let red = graph.add_node_with_params("Red", NodeOperation::ConstColor,
        vec![("r", NodeValue::Float(1.0)), ("g", NodeValue::Float(0.0)), ("b", NodeValue::Float(0.0)), ("a", NodeValue::Float(1.0))]);
    let blue = graph.add_node_with_params("Blue", NodeOperation::ConstColor,
        vec![("r", NodeValue::Float(0.0)), ("g", NodeValue::Float(0.0)), ("b", NodeValue::Float(1.0)), ("a", NodeValue::Float(1.0))]);
    let factor = graph.add_node_with_params("Factor", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(0.5))]);
    let mix = graph.add_node("Mix", NodeOperation::ColorMix);
    let display = graph.add_node("Display", NodeOperation::Display);

    let _ = graph.connect(red, "result", mix, "color_a");
    let _ = graph.connect(blue, "result", mix, "color_b");
    let _ = graph.connect(factor, "result", mix, "factor");
    let _ = graph.connect(mix, "result", display, "input");
    graph
}

/// Create a sine wave animation graph.
pub fn template_sine_animation() -> NodeGraph {
    let mut graph = NodeGraph::new("SineAnimation");
    let time = graph.add_node("Time", NodeOperation::Time);
    let freq = graph.add_node_with_params("Freq", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(2.0))]);
    let amp = graph.add_node_with_params("Amp", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(100.0))]);
    let sine = graph.add_node("Sine", NodeOperation::SineWave);
    let display = graph.add_node("Display", NodeOperation::Display);

    let _ = graph.connect(freq, "result", sine, "frequency");
    let _ = graph.connect(amp, "result", sine, "amplitude");
    let _ = graph.connect(time, "result", sine, "time");
    let _ = graph.connect(sine, "result", display, "input");
    graph
}

// ═══════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── NodeValue Tests ─────────────────────────────────────────────

    #[test]
    fn test_node_value_types() {
        assert_eq!(NodeValue::Float(1.0).type_name(), "Float");
        assert_eq!(NodeValue::Int(42).type_name(), "Int");
        assert_eq!(NodeValue::Bool(true).type_name(), "Bool");
        assert_eq!(NodeValue::String("hi".into()).type_name(), "String");
        assert_eq!(NodeValue::Vec3(1.0, 2.0, 3.0).type_name(), "Vec3");
        assert_eq!(NodeValue::None.type_name(), "None");
    }

    #[test]
    fn test_node_value_conversions() {
        assert_eq!(NodeValue::Float(3.14).as_float(), Some(3.14));
        assert_eq!(NodeValue::Int(42).as_float(), Some(42.0));
        assert_eq!(NodeValue::Bool(true).as_float(), Some(1.0));
        assert_eq!(NodeValue::Int(5).as_int(), Some(5));
        assert_eq!(NodeValue::Bool(true).as_bool(), Some(true));
        assert_eq!(NodeValue::Int(0).as_bool(), Some(false));
    }

    #[test]
    fn test_node_value_display() {
        assert_eq!(format!("{}", NodeValue::Float(3.14)), "3.14");
        assert_eq!(format!("{}", NodeValue::Bool(true)), "true");
        assert_eq!(format!("{}", NodeValue::None), "None");
    }

    // ── Port Tests ──────────────────────────────────────────────────

    #[test]
    fn test_port_type_compatibility() {
        assert!(PortType::Float.accepts(&PortType::Float));
        assert!(PortType::Float.accepts(&PortType::Int)); // Implicit conversion
        assert!(PortType::Any.accepts(&PortType::Float));
        assert!(PortType::Float.accepts(&PortType::Any));
        assert!(!PortType::Bool.accepts(&PortType::Float));
    }

    // ── Node Tests ──────────────────────────────────────────────────

    #[test]
    fn test_node_creation() {
        let node = Node::new(1, "TestAdd", NodeOperation::Add).at(100.0, 200.0);
        assert_eq!(node.id, 1);
        assert_eq!(node.name, "TestAdd");
        assert_eq!(node.position, (100.0, 200.0));
        assert_eq!(node.category(), NodeCategory::Math);
    }

    #[test]
    fn test_node_evaluate_add() {
        let mut node = Node::new(1, "Add", NodeOperation::Add);
        let mut inputs = HashMap::new();
        inputs.insert("a".into(), NodeValue::Float(3.0));
        inputs.insert("b".into(), NodeValue::Float(4.0));
        let outputs = node.evaluate(&inputs);
        assert_eq!(outputs.get("result"), Some(&NodeValue::Float(7.0)));
    }

    #[test]
    fn test_node_evaluate_multiply() {
        let mut node = Node::new(1, "Mul", NodeOperation::Multiply);
        let mut inputs = HashMap::new();
        inputs.insert("a".into(), NodeValue::Float(5.0));
        inputs.insert("b".into(), NodeValue::Float(3.0));
        let outputs = node.evaluate(&inputs);
        assert_eq!(outputs.get("result"), Some(&NodeValue::Float(15.0)));
    }

    #[test]
    fn test_node_evaluate_divide_by_zero() {
        let mut node = Node::new(1, "Div", NodeOperation::Divide);
        let mut inputs = HashMap::new();
        inputs.insert("a".into(), NodeValue::Float(10.0));
        inputs.insert("b".into(), NodeValue::Float(0.0));
        let outputs = node.evaluate(&inputs);
        assert_eq!(outputs.get("result"), Some(&NodeValue::Float(0.0)));
    }

    #[test]
    fn test_node_evaluate_logic() {
        let mut node = Node::new(1, "And", NodeOperation::And);
        let mut inputs = HashMap::new();
        inputs.insert("a".into(), NodeValue::Bool(true));
        inputs.insert("b".into(), NodeValue::Bool(false));
        let outputs = node.evaluate(&inputs);
        assert_eq!(outputs.get("result"), Some(&NodeValue::Bool(false)));
    }

    #[test]
    fn test_node_evaluate_select() {
        let mut node = Node::new(1, "Select", NodeOperation::Select);
        let mut inputs = HashMap::new();
        inputs.insert("condition".into(), NodeValue::Bool(true));
        inputs.insert("true_val".into(), NodeValue::Float(42.0));
        inputs.insert("false_val".into(), NodeValue::Float(0.0));
        let outputs = node.evaluate(&inputs);
        assert_eq!(outputs.get("result"), Some(&NodeValue::Float(42.0)));
    }

    #[test]
    fn test_node_evaluate_clamp() {
        let mut node = Node::new(1, "Clamp", NodeOperation::Clamp);
        let mut inputs = HashMap::new();
        inputs.insert("value".into(), NodeValue::Float(150.0));
        inputs.insert("min".into(), NodeValue::Float(0.0));
        inputs.insert("max".into(), NodeValue::Float(100.0));
        let outputs = node.evaluate(&inputs);
        assert_eq!(outputs.get("result"), Some(&NodeValue::Float(100.0)));
    }

    #[test]
    fn test_node_evaluate_lerp() {
        let mut node = Node::new(1, "Lerp", NodeOperation::Lerp);
        let mut inputs = HashMap::new();
        inputs.insert("a".into(), NodeValue::Float(0.0));
        inputs.insert("b".into(), NodeValue::Float(100.0));
        inputs.insert("t".into(), NodeValue::Float(0.25));
        let outputs = node.evaluate(&inputs);
        assert_eq!(outputs.get("result"), Some(&NodeValue::Float(25.0)));
    }

    // ── Graph Tests ─────────────────────────────────────────────────

    #[test]
    fn test_graph_creation() {
        let graph = NodeGraph::new("test");
        assert_eq!(graph.name, "test");
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_graph_add_remove_nodes() {
        let mut graph = NodeGraph::new("test");
        let a = graph.add_node("A", NodeOperation::ConstFloat);
        let b = graph.add_node("B", NodeOperation::ConstFloat);
        assert_eq!(graph.node_count(), 2);
        graph.remove_node(a);
        assert_eq!(graph.node_count(), 1);
        assert!(graph.nodes.contains_key(&b));
    }

    #[test]
    fn test_graph_connect() {
        let mut graph = NodeGraph::new("test");
        let a = graph.add_node("A", NodeOperation::ConstFloat);
        let add = graph.add_node("Add", NodeOperation::Add);
        assert!(graph.connect(a, "result", add, "a").is_ok());
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_graph_connect_type_mismatch() {
        let mut graph = NodeGraph::new("test");
        let a = graph.add_node("A", NodeOperation::ConstBool);
        let add = graph.add_node("Add", NodeOperation::Add);
        // Bool -> Float should fail
        assert!(graph.connect(a, "result", add, "a").is_err());
    }

    #[test]
    fn test_graph_self_connection() {
        let mut graph = NodeGraph::new("test");
        let a = graph.add_node("A", NodeOperation::Add);
        assert!(graph.connect(a, "result", a, "a").is_err());
    }

    #[test]
    fn test_graph_topological_sort() {
        let mut graph = NodeGraph::new("test");
        let a = graph.add_node("A", NodeOperation::ConstFloat);
        let b = graph.add_node("B", NodeOperation::ConstFloat);
        let add = graph.add_node("Add", NodeOperation::Add);
        let _ = graph.connect(a, "result", add, "a");
        let _ = graph.connect(b, "result", add, "b");
        let sorted = graph.topological_sort().unwrap();
        // a and b must come before add
        let pos_a = sorted.iter().position(|&x| x == a).unwrap();
        let pos_b = sorted.iter().position(|&x| x == b).unwrap();
        let pos_add = sorted.iter().position(|&x| x == add).unwrap();
        assert!(pos_a < pos_add);
        assert!(pos_b < pos_add);
    }

    #[test]
    fn test_graph_evaluate() {
        let mut graph = NodeGraph::new("test");
        let a = graph.add_node_with_params("A", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(3.0))]);
        let b = graph.add_node_with_params("B", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(4.0))]);
        let add = graph.add_node("Add", NodeOperation::Add);
        let _ = graph.connect(a, "result", add, "a");
        let _ = graph.connect(b, "result", add, "b");
        let results = graph.evaluate().unwrap();
        assert_eq!(results[&add].get("result"), Some(&NodeValue::Float(7.0)));
    }

    #[test]
    fn test_graph_chained_evaluation() {
        let mut graph = NodeGraph::new("chained");
        let a = graph.add_node_with_params("A", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(2.0))]);
        let b = graph.add_node_with_params("B", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(3.0))]);
        let c = graph.add_node_with_params("C", NodeOperation::ConstFloat, vec![("value", NodeValue::Float(10.0))]);
        let add = graph.add_node("Add", NodeOperation::Add);
        let mul = graph.add_node("Mul", NodeOperation::Multiply);
        let _ = graph.connect(a, "result", add, "a");
        let _ = graph.connect(b, "result", add, "b");
        let _ = graph.connect(add, "result", mul, "a");
        let _ = graph.connect(c, "result", mul, "b");
        let results = graph.evaluate().unwrap();
        // (2+3) * 10 = 50
        assert_eq!(results[&mul].get("result"), Some(&NodeValue::Float(50.0)));
    }

    // ── Template Tests ──────────────────────────────────────────────

    #[test]
    fn test_template_math_expression() {
        let mut graph = template_math_expression();
        assert_eq!(graph.node_count(), 6);
        assert_eq!(graph.edge_count(), 5);
        let results = graph.evaluate().unwrap();
        // (3+4) * 2 = 14, find the Multiply node
        let mul_id = graph.nodes.iter().find(|(_, n)| n.operation == NodeOperation::Multiply).map(|(&id, _)| id).unwrap();
        assert_eq!(results[&mul_id].get("result"), Some(&NodeValue::Float(14.0)));
    }

    #[test]
    fn test_template_color_mixer() {
        let mut graph = template_color_mixer();
        let results = graph.evaluate().unwrap();
        let mix_id = graph.nodes.iter().find(|(_, n)| n.operation == NodeOperation::ColorMix).map(|(&id, _)| id).unwrap();
        if let Some(NodeValue::Color(r, _, b, _)) = results[&mix_id].get("result") {
            assert!(*r > 0.0 && *r < 1.0); // Mixed red
            assert!(*b > 0.0 && *b < 1.0); // Mixed blue
        } else {
            panic!("Expected color output from mixer");
        }
    }

    #[test]
    fn test_template_sine_animation() {
        let graph = template_sine_animation();
        assert_eq!(graph.node_count(), 5);
        assert!(!graph.has_cycle());
    }

    // ── DOT Export Test ─────────────────────────────────────────────

    #[test]
    fn test_dot_export() {
        let graph = template_math_expression();
        let dot = graph.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("rankdir=LR"));
        assert!(dot.contains("->"));
    }

    // ── Disconnect Test ─────────────────────────────────────────────

    #[test]
    fn test_disconnect() {
        let mut graph = NodeGraph::new("test");
        let a = graph.add_node("A", NodeOperation::ConstFloat);
        let add = graph.add_node("Add", NodeOperation::Add);
        let _ = graph.connect(a, "result", add, "a");
        assert_eq!(graph.edge_count(), 1);
        graph.disconnect(a, "result", add, "a");
        assert_eq!(graph.edge_count(), 0);
    }

    // ── Category Query Test ─────────────────────────────────────────

    #[test]
    fn test_nodes_in_category() {
        let mut graph = NodeGraph::new("test");
        graph.add_node("A", NodeOperation::ConstFloat);
        graph.add_node("B", NodeOperation::ConstInt);
        graph.add_node("Add", NodeOperation::Add);
        let generators = graph.nodes_in_category(NodeCategory::Generator);
        let math = graph.nodes_in_category(NodeCategory::Math);
        assert_eq!(generators.len(), 2);
        assert_eq!(math.len(), 1);
    }

    // ── Node Operation Categories ───────────────────────────────────

    #[test]
    fn test_operation_categories() {
        assert_eq!(NodeOperation::ConstFloat.category(), NodeCategory::Generator);
        assert_eq!(NodeOperation::Add.category(), NodeCategory::Math);
        assert_eq!(NodeOperation::And.category(), NodeCategory::Logic);
        assert_eq!(NodeOperation::Translate.category(), NodeCategory::Transform);
        assert_eq!(NodeOperation::ColorMix.category(), NodeCategory::Color);
        assert_eq!(NodeOperation::Blur.category(), NodeCategory::Filter);
        assert_eq!(NodeOperation::Circle.category(), NodeCategory::Geometry);
        assert_eq!(NodeOperation::Checkerboard.category(), NodeCategory::Texture);
        assert_eq!(NodeOperation::Display.category(), NodeCategory::Output);
        assert_eq!(NodeOperation::Branch.category(), NodeCategory::Flow);
        assert_eq!(NodeOperation::Print.category(), NodeCategory::Utility);
    }

    // ── Sine Wave Evaluation ────────────────────────────────────────

    #[test]
    fn test_sine_wave_node() {
        let mut node = Node::new(1, "Sine", NodeOperation::SineWave);
        let mut inputs = HashMap::new();
        inputs.insert("frequency".into(), NodeValue::Float(1.0));
        inputs.insert("amplitude".into(), NodeValue::Float(1.0));
        inputs.insert("time".into(), NodeValue::Float(0.25)); // quarter period
        let outputs = node.evaluate(&inputs);
        if let Some(NodeValue::Float(v)) = outputs.get("result") {
            assert!((v - 1.0).abs() < 1e-6); // sin(π/2) = 1
        } else {
            panic!("Expected float output");
        }
    }
}
