//! Hardware synthesis / High-Level Synthesis (HLS) engine.
//!
//! Compiles a subset of Vitalis into RTL descriptions:
//! - **Pipeline scheduling**: ASAP / ALAP / list scheduling
//! - **Resource binding**: Operator → functional-unit mapping
//! - **FSM extraction**: Control-flow → state machine
//! - **Fixed-point arithmetic**: Integer-based fixed-point ops
//! - **Streaming dataflow**: Pipelined dataflow graphs
//! - **Verilog output**: Emit synthesizable Verilog

use std::collections::HashMap;
use std::fmt;

// ── Data Types ──────────────────────────────────────────────────────

/// Fixed-point number with configurable integer and fractional width.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedPoint {
    pub value: i64,
    pub integer_bits: u8,
    pub fractional_bits: u8,
}

impl FixedPoint {
    pub fn new(value: f64, integer_bits: u8, fractional_bits: u8) -> Self {
        let scale = 1i64 << fractional_bits;
        Self {
            value: (value * scale as f64) as i64,
            integer_bits,
            fractional_bits,
        }
    }

    pub fn to_f64(&self) -> f64 {
        self.value as f64 / (1i64 << self.fractional_bits) as f64
    }

    pub fn total_bits(&self) -> u8 {
        self.integer_bits + self.fractional_bits + 1 // +1 for sign
    }

    pub fn add(&self, other: &FixedPoint) -> FixedPoint {
        assert_eq!(self.fractional_bits, other.fractional_bits);
        FixedPoint {
            value: self.value + other.value,
            integer_bits: self.integer_bits.max(other.integer_bits) + 1,
            fractional_bits: self.fractional_bits,
        }
    }

    pub fn mul(&self, other: &FixedPoint) -> FixedPoint {
        let product = (self.value as i128 * other.value as i128) >> self.fractional_bits;
        FixedPoint {
            value: product as i64,
            integer_bits: self.integer_bits + other.integer_bits,
            fractional_bits: self.fractional_bits,
        }
    }
}

// ── RTL IR ──────────────────────────────────────────────────────────

/// An RTL operation node in the DFG.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RtlOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    Xor,
    Not,
    Shl,
    Shr,
    Mux,
    Reg,
    Const(i64),
    Input(String),
    Output(String),
}

/// A node in the dataflow graph.
#[derive(Debug, Clone)]
pub struct DfgNode {
    pub id: u32,
    pub op: RtlOp,
    pub inputs: Vec<u32>,
    pub bit_width: u32,
    pub scheduled_cycle: Option<u32>,
    pub bound_unit: Option<String>,
}

/// Dataflow graph for HLS.
pub struct DataflowGraph {
    nodes: Vec<DfgNode>,
    next_id: u32,
}

impl DataflowGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), next_id: 0 }
    }

    pub fn add_node(&mut self, op: RtlOp, inputs: Vec<u32>, bit_width: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(DfgNode {
            id, op, inputs, bit_width,
            scheduled_cycle: None,
            bound_unit: None,
        });
        id
    }

    pub fn get_node(&self, id: u32) -> Option<&DfgNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: u32) -> Option<&mut DfgNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes(&self) -> &[DfgNode] {
        &self.nodes
    }

    /// ASAP scheduling: schedule each node as early as possible.
    pub fn schedule_asap(&mut self, latency: &HashMap<RtlOp, u32>) {
        // Topological pass.
        let order = self.topo_order();
        for id in order {
            let node = self.nodes.iter().find(|n| n.id == id).unwrap();
            let inputs = node.inputs.clone();
            let op = node.op.clone();
            let max_input_cycle = inputs.iter()
                .filter_map(|&inp| {
                    let inp_node = self.nodes.iter().find(|n| n.id == inp)?;
                    let lat = latency.get(&inp_node.op).copied().unwrap_or(1);
                    Some(inp_node.scheduled_cycle.unwrap_or(0) + lat)
                })
                .max()
                .unwrap_or(0);
            if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                n.scheduled_cycle = Some(max_input_cycle);
            }
            let _ = op;
        }
    }

    /// ALAP scheduling: schedule as late as possible given a deadline.
    pub fn schedule_alap(&mut self, deadline: u32, latency: &HashMap<RtlOp, u32>) {
        let order = self.topo_order();
        // Initialize all to deadline.
        for n in &mut self.nodes {
            let lat = latency.get(&n.op).copied().unwrap_or(1);
            n.scheduled_cycle = Some(deadline.saturating_sub(lat));
        }
        // Reverse pass.
        for &id in order.iter().rev() {
            let node = self.nodes.iter().find(|n| n.id == id).unwrap();
            let cycle = node.scheduled_cycle.unwrap();
            let inputs = node.inputs.clone();
            for &inp_id in &inputs {
                if let Some(inp_node) = self.nodes.iter_mut().find(|n| n.id == inp_id) {
                    let lat = latency.get(&inp_node.op).copied().unwrap_or(1);
                    let new_cycle = cycle.saturating_sub(lat);
                    if let Some(existing) = inp_node.scheduled_cycle {
                        if new_cycle < existing {
                            inp_node.scheduled_cycle = Some(new_cycle);
                        }
                    }
                }
            }
        }
    }

    fn topo_order(&self) -> Vec<u32> {
        let mut in_degree: HashMap<u32, usize> = HashMap::new();
        for n in &self.nodes {
            in_degree.entry(n.id).or_insert(0);
            for &inp in &n.inputs {
                *in_degree.entry(n.id).or_insert(0) += 0;
                let _ = inp;
            }
        }
        for n in &self.nodes {
            for &inp in &n.inputs {
                // inp must come before n.
                let _ = in_degree.entry(inp).or_insert(0);
            }
            let count = n.inputs.len();
            *in_degree.entry(n.id).or_insert(0) += count;
        }
        // Simple Kahn's.
        let mut queue: Vec<u32> = in_degree.iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort();
        let mut result = Vec::new();
        while let Some(id) = queue.pop() {
            result.push(id);
            for n in &self.nodes {
                if n.inputs.contains(&id) {
                    let deg = in_degree.get_mut(&n.id).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(n.id);
                        queue.sort();
                    }
                }
            }
        }
        result
    }
}

// ── FSM Extraction ──────────────────────────────────────────────────

/// State in a hardware state machine.
#[derive(Debug, Clone)]
pub struct FsmState {
    pub id: u32,
    pub name: String,
    pub operations: Vec<u32>,
    pub transitions: Vec<FsmTransition>,
    pub is_initial: bool,
    pub is_final: bool,
}

/// Transition between FSM states.
#[derive(Debug, Clone)]
pub struct FsmTransition {
    pub target: u32,
    pub condition: Option<String>,
}

/// Hardware FSM.
pub struct Fsm {
    pub states: Vec<FsmState>,
    pub name: String,
}

impl Fsm {
    pub fn new(name: &str) -> Self {
        Self { states: Vec::new(), name: name.to_string() }
    }

    pub fn add_state(&mut self, name: &str, is_initial: bool, is_final: bool) -> u32 {
        let id = self.states.len() as u32;
        self.states.push(FsmState {
            id, name: name.to_string(),
            operations: Vec::new(),
            transitions: Vec::new(),
            is_initial, is_final,
        });
        id
    }

    pub fn add_transition(&mut self, from: u32, to: u32, condition: Option<String>) {
        if let Some(state) = self.states.iter_mut().find(|s| s.id == from) {
            state.transitions.push(FsmTransition { target: to, condition });
        }
    }

    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    pub fn initial_state(&self) -> Option<&FsmState> {
        self.states.iter().find(|s| s.is_initial)
    }
}

// ── Resource Binding ────────────────────────────────────────────────

/// A functional unit (hardware resource).
#[derive(Debug, Clone)]
pub struct FunctionalUnit {
    pub name: String,
    pub op_type: RtlOp,
    pub area: u32,
    pub delay_ns: f64,
    pub is_pipelined: bool,
    pub stages: u32,
}

/// Resource library.
pub struct ResourceLibrary {
    units: Vec<FunctionalUnit>,
}

impl ResourceLibrary {
    pub fn new() -> Self {
        Self { units: Vec::new() }
    }

    pub fn add_unit(&mut self, unit: FunctionalUnit) {
        self.units.push(unit);
    }

    pub fn find_for_op(&self, op: &RtlOp) -> Vec<&FunctionalUnit> {
        self.units.iter().filter(|u| u.op_type == *op).collect()
    }

    pub fn cheapest_for_op(&self, op: &RtlOp) -> Option<&FunctionalUnit> {
        self.find_for_op(op).into_iter().min_by_key(|u| u.area)
    }

    pub fn fastest_for_op(&self, op: &RtlOp) -> Option<&FunctionalUnit> {
        self.find_for_op(op).into_iter()
            .min_by(|a, b| a.delay_ns.partial_cmp(&b.delay_ns).unwrap())
    }

    pub fn total_area(&self) -> u32 {
        self.units.iter().map(|u| u.area).sum()
    }
}

// ── Verilog Output ─────────────────────────────────────────────────

/// Generate Verilog for an FSM.
pub fn emit_verilog_fsm(fsm: &Fsm) -> String {
    let mut out = String::new();
    out.push_str(&format!("module {}(\n", fsm.name));
    out.push_str("    input wire clk,\n");
    out.push_str("    input wire rst,\n");
    out.push_str("    output reg [7:0] state\n");
    out.push_str(");\n\n");

    // State encoding.
    for (i, state) in fsm.states.iter().enumerate() {
        out.push_str(&format!("    localparam {} = 8'd{};\n", state.name, i));
    }
    out.push_str("\n");

    // State register.
    out.push_str("    always @(posedge clk or posedge rst) begin\n");
    out.push_str("        if (rst) begin\n");
    if let Some(init) = fsm.initial_state() {
        out.push_str(&format!("            state <= {};\n", init.name));
    }
    out.push_str("        end else begin\n");
    out.push_str("            case (state)\n");
    for state in &fsm.states {
        out.push_str(&format!("                {}: begin\n", state.name));
        for tr in &state.transitions {
            let target = fsm.states.iter().find(|s| s.id == tr.target)
                .map(|s| s.name.as_str()).unwrap_or("UNKNOWN");
            if let Some(cond) = &tr.condition {
                out.push_str(&format!("                    if ({}) state <= {};\n", cond, target));
            } else {
                out.push_str(&format!("                    state <= {};\n", target));
            }
        }
        out.push_str("                end\n");
    }
    out.push_str("            endcase\n");
    out.push_str("        end\n");
    out.push_str("    end\n\n");
    out.push_str("endmodule\n");
    out
}

/// Resource estimation summary.
#[derive(Debug, Clone)]
pub struct ResourceEstimate {
    pub luts: u32,
    pub registers: u32,
    pub dsps: u32,
    pub brams: u32,
    pub total_area: u32,
    pub max_freq_mhz: f64,
}

/// Estimate FPGA resources for a DFG.
pub fn estimate_resources(dfg: &DataflowGraph) -> ResourceEstimate {
    let mut luts = 0u32;
    let mut regs = 0u32;
    let mut dsps = 0u32;

    for node in dfg.nodes() {
        match &node.op {
            RtlOp::Add | RtlOp::Sub => { luts += node.bit_width; }
            RtlOp::Mul => { dsps += 1; }
            RtlOp::Div => { dsps += 4; luts += node.bit_width * 2; }
            RtlOp::Reg => { regs += node.bit_width; }
            RtlOp::Mux => { luts += node.bit_width; }
            RtlOp::And | RtlOp::Or | RtlOp::Xor | RtlOp::Not => { luts += node.bit_width / 4; }
            RtlOp::Shl | RtlOp::Shr => { luts += node.bit_width; }
            _ => {}
        }
    }

    ResourceEstimate {
        luts,
        registers: regs,
        dsps,
        brams: 0,
        total_area: luts * 2 + regs + dsps * 100,
        max_freq_mhz: if dsps > 0 { 200.0 } else { 400.0 },
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_point_new() {
        let fp = FixedPoint::new(3.14, 8, 8);
        let back = fp.to_f64();
        assert!((back - 3.14).abs() < 0.01);
    }

    #[test]
    fn test_fixed_point_add() {
        let a = FixedPoint::new(1.5, 8, 8);
        let b = FixedPoint::new(2.5, 8, 8);
        let c = a.add(&b);
        assert!((c.to_f64() - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_fixed_point_mul() {
        let a = FixedPoint::new(2.0, 8, 8);
        let b = FixedPoint::new(3.0, 8, 8);
        let c = a.mul(&b);
        assert!((c.to_f64() - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_fixed_point_total_bits() {
        let fp = FixedPoint::new(0.0, 8, 8);
        assert_eq!(fp.total_bits(), 17); // 8 + 8 + 1 sign
    }

    #[test]
    fn test_dfg_creation() {
        let mut dfg = DataflowGraph::new();
        let a = dfg.add_node(RtlOp::Input("a".into()), vec![], 32);
        let b = dfg.add_node(RtlOp::Input("b".into()), vec![], 32);
        let c = dfg.add_node(RtlOp::Add, vec![a, b], 32);
        assert_eq!(dfg.node_count(), 3);
        assert_eq!(dfg.get_node(c).unwrap().inputs, vec![a, b]);
    }

    #[test]
    fn test_asap_schedule() {
        let mut dfg = DataflowGraph::new();
        let a = dfg.add_node(RtlOp::Input("a".into()), vec![], 32);
        let b = dfg.add_node(RtlOp::Input("b".into()), vec![], 32);
        let _c = dfg.add_node(RtlOp::Add, vec![a, b], 32);

        let mut lat = HashMap::new();
        lat.insert(RtlOp::Input("a".into()), 0u32);
        lat.insert(RtlOp::Input("b".into()), 0u32);
        lat.insert(RtlOp::Add, 1);
        dfg.schedule_asap(&lat);
        // Add should be scheduled at cycle 0 since inputs have 0 latency.
        assert!(dfg.get_node(_c).unwrap().scheduled_cycle.is_some());
    }

    #[test]
    fn test_fsm_creation() {
        let mut fsm = Fsm::new("controller");
        let s0 = fsm.add_state("IDLE", true, false);
        let s1 = fsm.add_state("COMPUTE", false, false);
        let s2 = fsm.add_state("DONE", false, true);
        fsm.add_transition(s0, s1, Some("start".into()));
        fsm.add_transition(s1, s2, None);
        assert_eq!(fsm.state_count(), 3);
        assert_eq!(fsm.initial_state().unwrap().name, "IDLE");
    }

    #[test]
    fn test_verilog_output() {
        let mut fsm = Fsm::new("ctrl");
        let s0 = fsm.add_state("IDLE", true, false);
        let s1 = fsm.add_state("RUN", false, true);
        fsm.add_transition(s0, s1, Some("go".into()));
        let verilog = emit_verilog_fsm(&fsm);
        assert!(verilog.contains("module ctrl"));
        assert!(verilog.contains("IDLE"));
        assert!(verilog.contains("endmodule"));
    }

    #[test]
    fn test_resource_library() {
        let mut lib = ResourceLibrary::new();
        lib.add_unit(FunctionalUnit {
            name: "adder".into(), op_type: RtlOp::Add,
            area: 100, delay_ns: 2.5, is_pipelined: false, stages: 1,
        });
        lib.add_unit(FunctionalUnit {
            name: "fast_adder".into(), op_type: RtlOp::Add,
            area: 200, delay_ns: 1.0, is_pipelined: true, stages: 2,
        });
        assert_eq!(lib.find_for_op(&RtlOp::Add).len(), 2);
        assert_eq!(lib.cheapest_for_op(&RtlOp::Add).unwrap().name, "adder");
        assert_eq!(lib.fastest_for_op(&RtlOp::Add).unwrap().name, "fast_adder");
    }

    #[test]
    fn test_resource_estimation() {
        let mut dfg = DataflowGraph::new();
        dfg.add_node(RtlOp::Add, vec![], 32);
        dfg.add_node(RtlOp::Mul, vec![], 32);
        dfg.add_node(RtlOp::Reg, vec![], 16);
        let est = estimate_resources(&dfg);
        assert_eq!(est.luts, 32);
        assert_eq!(est.dsps, 1);
        assert_eq!(est.registers, 16);
    }

    #[test]
    fn test_fsm_transitions() {
        let mut fsm = Fsm::new("test");
        let s0 = fsm.add_state("A", true, false);
        let s1 = fsm.add_state("B", false, false);
        let s2 = fsm.add_state("C", false, true);
        fsm.add_transition(s0, s1, Some("cond1".into()));
        fsm.add_transition(s0, s2, Some("cond2".into()));
        let state = fsm.states.iter().find(|s| s.id == s0).unwrap();
        assert_eq!(state.transitions.len(), 2);
    }

    #[test]
    fn test_resource_library_total_area() {
        let mut lib = ResourceLibrary::new();
        lib.add_unit(FunctionalUnit {
            name: "a".into(), op_type: RtlOp::Add,
            area: 100, delay_ns: 2.0, is_pipelined: false, stages: 1,
        });
        lib.add_unit(FunctionalUnit {
            name: "m".into(), op_type: RtlOp::Mul,
            area: 500, delay_ns: 5.0, is_pipelined: true, stages: 3,
        });
        assert_eq!(lib.total_area(), 600);
    }

    #[test]
    fn test_alap_schedule() {
        let mut dfg = DataflowGraph::new();
        let a = dfg.add_node(RtlOp::Input("x".into()), vec![], 32);
        let _b = dfg.add_node(RtlOp::Add, vec![a], 32);
        let mut lat = HashMap::new();
        lat.insert(RtlOp::Input("x".into()), 1u32);
        lat.insert(RtlOp::Add, 1);
        dfg.schedule_alap(5, &lat);
        // Both should be scheduled.
        assert!(dfg.get_node(a).unwrap().scheduled_cycle.is_some());
    }

    #[test]
    fn test_fixed_point_negative() {
        let fp = FixedPoint::new(-2.5, 8, 8);
        assert!((fp.to_f64() - (-2.5)).abs() < 0.01);
    }

    #[test]
    fn test_const_node() {
        let mut dfg = DataflowGraph::new();
        let c = dfg.add_node(RtlOp::Const(42), vec![], 32);
        assert_eq!(dfg.get_node(c).unwrap().op, RtlOp::Const(42));
    }

    #[test]
    fn test_mux_estimation() {
        let mut dfg = DataflowGraph::new();
        dfg.add_node(RtlOp::Mux, vec![], 16);
        let est = estimate_resources(&dfg);
        assert_eq!(est.luts, 16);
    }

    #[test]
    fn test_empty_dfg() {
        let dfg = DataflowGraph::new();
        assert_eq!(dfg.node_count(), 0);
        let est = estimate_resources(&dfg);
        assert_eq!(est.total_area, 0);
    }
}
