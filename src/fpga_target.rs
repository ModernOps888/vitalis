//! FPGA target backend for Vitalis.
//!
//! Generates FPGA-specific outputs:
//! - **Xilinx / Intel primitives**: LUTs, DSPs, BRAMs, IOBs
//! - **Clock domain crossing**: CDC synchronizers
//! - **Constraint generation**: Timing / placement constraints (XDC/SDC)
//! - **Resource estimation**: Utilization reports
//! - **Simulation testbench**: Generates testbench Verilog
//! - **Bitstream metadata**: Device targeting and configuration

use std::collections::HashMap;

// ── FPGA Device Model ───────────────────────────────────────────────

/// FPGA vendor.
#[derive(Debug, Clone, PartialEq)]
pub enum FpgaVendor {
    Xilinx,
    Intel,
    Lattice,
    Gowin,
}

/// FPGA device specification.
#[derive(Debug, Clone)]
pub struct FpgaDevice {
    pub name: String,
    pub vendor: FpgaVendor,
    pub family: String,
    pub lut_count: u32,
    pub register_count: u32,
    pub dsp_count: u32,
    pub bram_kb: u32,
    pub io_count: u32,
    pub max_freq_mhz: f64,
    pub speed_grade: i32,
}

impl FpgaDevice {
    pub fn artix7_35t() -> Self {
        Self {
            name: "xc7a35t".into(), vendor: FpgaVendor::Xilinx,
            family: "Artix-7".into(), lut_count: 20800, register_count: 41600,
            dsp_count: 90, bram_kb: 1800, io_count: 250, max_freq_mhz: 450.0,
            speed_grade: -1,
        }
    }

    pub fn cyclone_v() -> Self {
        Self {
            name: "5CEBA4F23C7".into(), vendor: FpgaVendor::Intel,
            family: "Cyclone V".into(), lut_count: 18480, register_count: 36960,
            dsp_count: 66, bram_kb: 3080, io_count: 224, max_freq_mhz: 500.0,
            speed_grade: 7,
        }
    }

    pub fn zynq_7020() -> Self {
        Self {
            name: "xc7z020".into(), vendor: FpgaVendor::Xilinx,
            family: "Zynq-7000".into(), lut_count: 53200, register_count: 106400,
            dsp_count: 220, bram_kb: 4480, io_count: 200, max_freq_mhz: 500.0,
            speed_grade: -1,
        }
    }
}

// ── FPGA Primitives ─────────────────────────────────────────────────

/// FPGA primitive type.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Lut4 { init: u16 },
    Lut6 { init: u64 },
    FlipFlop { has_reset: bool, has_enable: bool },
    Dsp48 { a_width: u8, b_width: u8, use_mult: bool },
    BramPort { addr_width: u8, data_width: u8, is_dual: bool },
    Iob { direction: IoDirection, standard: String },
    Bufg,
    Pll { input_freq: f64, output_freq: f64 },
    Mmcm { input_freq: f64, outputs: Vec<f64> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum IoDirection {
    Input,
    Output,
    Bidir,
}

/// An instantiated primitive.
#[derive(Debug, Clone)]
pub struct PrimitiveInstance {
    pub name: String,
    pub primitive: Primitive,
    pub connections: HashMap<String, String>,
    pub attributes: HashMap<String, String>,
}

// ── Clock Domain Crossing ───────────────────────────────────────────

/// CDC synchronizer type.
#[derive(Debug, Clone, PartialEq)]
pub enum CdcType {
    TwoFlop,
    ThreeFlop,
    AsyncFifo,
    GrayCode,
    Handshake,
    PulseSync,
}

/// A clock domain.
#[derive(Debug, Clone)]
pub struct ClockDomain {
    pub name: String,
    pub frequency_mhz: f64,
    pub source: String,
    pub is_primary: bool,
}

/// CDC crossing point.
#[derive(Debug, Clone)]
pub struct CdcCrossing {
    pub source_domain: String,
    pub dest_domain: String,
    pub signal_name: String,
    pub sync_type: CdcType,
    pub bit_width: u32,
}

/// Generate CDC synchronizer Verilog.
pub fn generate_cdc_sync(crossing: &CdcCrossing) -> String {
    match crossing.sync_type {
        CdcType::TwoFlop => {
            format!(
                "// CDC: {} -> {} for signal {}\n\
                 reg [{w}:0] sync_stage1_{name};\n\
                 reg [{w}:0] sync_stage2_{name};\n\
                 always @(posedge clk_{dest}) begin\n\
                 \tsync_stage1_{name} <= {name};\n\
                 \tsync_stage2_{name} <= sync_stage1_{name};\n\
                 end\n",
                crossing.source_domain, crossing.dest_domain,
                crossing.signal_name,
                w = crossing.bit_width.saturating_sub(1),
                name = crossing.signal_name,
                dest = crossing.dest_domain,
            )
        }
        CdcType::ThreeFlop => {
            format!(
                "// CDC 3-flop: {} -> {} for signal {}\n\
                 reg [{w}:0] sync_s1_{name}, sync_s2_{name}, sync_s3_{name};\n\
                 always @(posedge clk_{dest}) begin\n\
                 \tsync_s1_{name} <= {name};\n\
                 \tsync_s2_{name} <= sync_s1_{name};\n\
                 \tsync_s3_{name} <= sync_s2_{name};\n\
                 end\n",
                crossing.source_domain, crossing.dest_domain,
                crossing.signal_name,
                w = crossing.bit_width.saturating_sub(1),
                name = crossing.signal_name,
                dest = crossing.dest_domain,
            )
        }
        _ => format!("// CDC {} for {}\n", 
            match crossing.sync_type {
                CdcType::AsyncFifo => "async_fifo",
                CdcType::GrayCode => "gray_code",
                CdcType::Handshake => "handshake",
                CdcType::PulseSync => "pulse_sync",
                _ => "unknown",
            },
            crossing.signal_name),
    }
}

// ── Constraint Generation ───────────────────────────────────────────

/// Constraint type (XDC for Xilinx, SDC for Intel).
#[derive(Debug, Clone)]
pub enum Constraint {
    ClockPeriod { name: String, period_ns: f64, waveform: (f64, f64) },
    IoStandard { port: String, standard: String },
    PinLocation { port: String, pin: String },
    MaxDelay { from: String, to: String, delay_ns: f64 },
    FalsePathBetween { from: String, to: String },
    MulticyclePath { from: String, to: String, cycles: u32 },
    SetInputDelay { port: String, delay_ns: f64, clock: String },
    SetOutputDelay { port: String, delay_ns: f64, clock: String },
}

/// Generate XDC constraints (Xilinx).
pub fn generate_xdc(constraints: &[Constraint]) -> String {
    let mut out = String::new();
    out.push_str("# Vitalis FPGA Target — Generated Constraints (XDC)\n\n");
    for c in constraints {
        match c {
            Constraint::ClockPeriod { name, period_ns, waveform } => {
                out.push_str(&format!(
                    "create_clock -period {:.3} -waveform {{{:.3} {:.3}}} [get_ports {}]\n",
                    period_ns, waveform.0, waveform.1, name
                ));
            }
            Constraint::IoStandard { port, standard } => {
                out.push_str(&format!(
                    "set_property IOSTANDARD {} [get_ports {}]\n", standard, port
                ));
            }
            Constraint::PinLocation { port, pin } => {
                out.push_str(&format!(
                    "set_property PACKAGE_PIN {} [get_ports {}]\n", pin, port
                ));
            }
            Constraint::MaxDelay { from, to, delay_ns } => {
                out.push_str(&format!(
                    "set_max_delay {:.3} -from [get_cells {}] -to [get_cells {}]\n",
                    delay_ns, from, to
                ));
            }
            Constraint::FalsePathBetween { from, to } => {
                out.push_str(&format!(
                    "set_false_path -from [get_clocks {}] -to [get_clocks {}]\n", from, to
                ));
            }
            Constraint::MulticyclePath { from, to, cycles } => {
                out.push_str(&format!(
                    "set_multicycle_path {} -from [get_cells {}] -to [get_cells {}]\n",
                    cycles, from, to
                ));
            }
            Constraint::SetInputDelay { port, delay_ns, clock } => {
                out.push_str(&format!(
                    "set_input_delay -clock {} {:.3} [get_ports {}]\n", clock, delay_ns, port
                ));
            }
            Constraint::SetOutputDelay { port, delay_ns, clock } => {
                out.push_str(&format!(
                    "set_output_delay -clock {} {:.3} [get_ports {}]\n", clock, delay_ns, port
                ));
            }
        }
    }
    out
}

// ── Simulation Testbench ────────────────────────────────────────────

/// Testbench stimulus.
#[derive(Debug, Clone)]
pub struct TestStimulus {
    pub time_ns: u64,
    pub signal: String,
    pub value: String,
}

/// Generate a simulation testbench.
pub fn generate_testbench(module_name: &str, inputs: &[(&str, u32)], stimuli: &[TestStimulus]) -> String {
    let mut tb = String::new();
    tb.push_str(&format!("`timescale 1ns/1ps\n\n"));
    tb.push_str(&format!("module {}_tb;\n\n", module_name));

    // Clock generation.
    tb.push_str("    reg clk = 0;\n");
    tb.push_str("    always #5 clk = ~clk;\n\n");
    tb.push_str("    reg rst = 1;\n\n");

    // Input regs.
    for (name, width) in inputs {
        if *width > 1 {
            tb.push_str(&format!("    reg [{}:0] {};\n", width - 1, name));
        } else {
            tb.push_str(&format!("    reg {};\n", name));
        }
    }
    tb.push_str("\n");

    // DUT instantiation.
    tb.push_str(&format!("    {} dut (\n", module_name));
    tb.push_str("        .clk(clk),\n");
    tb.push_str("        .rst(rst)");
    for (name, _) in inputs {
        tb.push_str(&format!(",\n        .{}({})", name, name));
    }
    tb.push_str("\n    );\n\n");

    // Stimulus.
    tb.push_str("    initial begin\n");
    tb.push_str("        $dumpfile(\"dump.vcd\");\n");
    tb.push_str(&format!("        $dumpvars(0, {}_tb);\n\n", module_name));
    tb.push_str("        #20 rst = 0;\n\n");
    for stim in stimuli {
        tb.push_str(&format!("        #{} {} = {};\n", stim.time_ns, stim.signal, stim.value));
    }
    tb.push_str("\n        #100 $finish;\n");
    tb.push_str("    end\n\n");
    tb.push_str("endmodule\n");
    tb
}

// ── Utilization Report ──────────────────────────────────────────────

/// Utilization report.
#[derive(Debug, Clone)]
pub struct UtilizationReport {
    pub device: String,
    pub lut_used: u32,
    pub lut_available: u32,
    pub reg_used: u32,
    pub reg_available: u32,
    pub dsp_used: u32,
    pub dsp_available: u32,
    pub bram_used: u32,
    pub bram_available: u32,
}

impl UtilizationReport {
    pub fn lut_percent(&self) -> f64 {
        if self.lut_available == 0 { return 0.0; }
        self.lut_used as f64 / self.lut_available as f64 * 100.0
    }

    pub fn reg_percent(&self) -> f64 {
        if self.reg_available == 0 { return 0.0; }
        self.reg_used as f64 / self.reg_available as f64 * 100.0
    }

    pub fn dsp_percent(&self) -> f64 {
        if self.dsp_available == 0 { return 0.0; }
        self.dsp_used as f64 / self.dsp_available as f64 * 100.0
    }

    pub fn fits(&self) -> bool {
        self.lut_used <= self.lut_available
            && self.reg_used <= self.reg_available
            && self.dsp_used <= self.dsp_available
            && self.bram_used <= self.bram_available
    }

    pub fn summary(&self) -> String {
        format!(
            "Device: {}\n  LUTs: {}/{} ({:.1}%)\n  Regs: {}/{} ({:.1}%)\n  DSPs: {}/{} ({:.1}%)\n  BRAMs: {}/{}\n  Fits: {}",
            self.device,
            self.lut_used, self.lut_available, self.lut_percent(),
            self.reg_used, self.reg_available, self.reg_percent(),
            self.dsp_used, self.dsp_available, self.dsp_percent(),
            self.bram_used, self.bram_available,
            if self.fits() { "YES" } else { "NO" }
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_artix7() {
        let dev = FpgaDevice::artix7_35t();
        assert_eq!(dev.vendor, FpgaVendor::Xilinx);
        assert_eq!(dev.lut_count, 20800);
    }

    #[test]
    fn test_device_cyclone_v() {
        let dev = FpgaDevice::cyclone_v();
        assert_eq!(dev.vendor, FpgaVendor::Intel);
    }

    #[test]
    fn test_device_zynq() {
        let dev = FpgaDevice::zynq_7020();
        assert_eq!(dev.family, "Zynq-7000");
        assert_eq!(dev.dsp_count, 220);
    }

    #[test]
    fn test_cdc_two_flop() {
        let crossing = CdcCrossing {
            source_domain: "clk_a".into(),
            dest_domain: "clk_b".into(),
            signal_name: "data".into(),
            sync_type: CdcType::TwoFlop,
            bit_width: 8,
        };
        let verilog = generate_cdc_sync(&crossing);
        assert!(verilog.contains("sync_stage1_data"));
        assert!(verilog.contains("sync_stage2_data"));
    }

    #[test]
    fn test_cdc_three_flop() {
        let crossing = CdcCrossing {
            source_domain: "fast".into(),
            dest_domain: "slow".into(),
            signal_name: "valid".into(),
            sync_type: CdcType::ThreeFlop,
            bit_width: 1,
        };
        let verilog = generate_cdc_sync(&crossing);
        assert!(verilog.contains("sync_s3_valid"));
    }

    #[test]
    fn test_xdc_clock() {
        let constraints = vec![
            Constraint::ClockPeriod {
                name: "sys_clk".into(), period_ns: 10.0, waveform: (0.0, 5.0),
            },
        ];
        let xdc = generate_xdc(&constraints);
        assert!(xdc.contains("create_clock -period 10.000"));
    }

    #[test]
    fn test_xdc_pin() {
        let constraints = vec![
            Constraint::PinLocation { port: "led[0]".into(), pin: "H17".into() },
            Constraint::IoStandard { port: "led[0]".into(), standard: "LVCMOS33".into() },
        ];
        let xdc = generate_xdc(&constraints);
        assert!(xdc.contains("PACKAGE_PIN H17"));
        assert!(xdc.contains("IOSTANDARD LVCMOS33"));
    }

    #[test]
    fn test_testbench_generation() {
        let stimuli = vec![
            TestStimulus { time_ns: 10, signal: "din".into(), value: "8'hAA".into() },
            TestStimulus { time_ns: 20, signal: "din".into(), value: "8'h55".into() },
        ];
        let tb = generate_testbench("my_design", &[("din", 8)], &stimuli);
        assert!(tb.contains("module my_design_tb"));
        assert!(tb.contains("$dumpfile"));
        assert!(tb.contains("8'hAA"));
    }

    #[test]
    fn test_utilization_report() {
        let report = UtilizationReport {
            device: "xc7a35t".into(),
            lut_used: 5000, lut_available: 20800,
            reg_used: 3000, reg_available: 41600,
            dsp_used: 10, dsp_available: 90,
            bram_used: 5, bram_available: 50,
        };
        assert!(report.fits());
        assert!((report.lut_percent() - 24.04).abs() < 0.1);
    }

    #[test]
    fn test_utilization_exceeds() {
        let report = UtilizationReport {
            device: "small".into(),
            lut_used: 25000, lut_available: 20000,
            reg_used: 0, reg_available: 40000,
            dsp_used: 0, dsp_available: 100,
            bram_used: 0, bram_available: 50,
        };
        assert!(!report.fits());
    }

    #[test]
    fn test_utilization_summary() {
        let report = UtilizationReport {
            device: "test".into(),
            lut_used: 100, lut_available: 1000,
            reg_used: 50, reg_available: 2000,
            dsp_used: 2, dsp_available: 10,
            bram_used: 0, bram_available: 20,
        };
        let s = report.summary();
        assert!(s.contains("Fits: YES"));
    }

    #[test]
    fn test_primitive_instance() {
        let inst = PrimitiveInstance {
            name: "lut_0".into(),
            primitive: Primitive::Lut6 { init: 0xDEADBEEF },
            connections: HashMap::new(),
            attributes: HashMap::new(),
        };
        assert_eq!(inst.name, "lut_0");
    }

    #[test]
    fn test_clock_domain() {
        let d = ClockDomain {
            name: "sys_clk".into(), frequency_mhz: 100.0,
            source: "external".into(), is_primary: true,
        };
        assert!(d.is_primary);
    }

    #[test]
    fn test_false_path_constraint() {
        let constraints = vec![
            Constraint::FalsePathBetween { from: "clk_a".into(), to: "clk_b".into() },
        ];
        let xdc = generate_xdc(&constraints);
        assert!(xdc.contains("set_false_path"));
    }

    #[test]
    fn test_input_output_delay() {
        let constraints = vec![
            Constraint::SetInputDelay { port: "din".into(), delay_ns: 2.0, clock: "clk".into() },
            Constraint::SetOutputDelay { port: "dout".into(), delay_ns: 3.0, clock: "clk".into() },
        ];
        let xdc = generate_xdc(&constraints);
        assert!(xdc.contains("set_input_delay"));
        assert!(xdc.contains("set_output_delay"));
    }

    #[test]
    fn test_pll_primitive() {
        let pll = Primitive::Pll { input_freq: 100.0, output_freq: 200.0 };
        match pll {
            Primitive::Pll { input_freq, output_freq } => {
                assert!((output_freq / input_freq - 2.0).abs() < 0.001);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_multicycle_constraint() {
        let constraints = vec![
            Constraint::MulticyclePath { from: "a".into(), to: "b".into(), cycles: 3 },
        ];
        let xdc = generate_xdc(&constraints);
        assert!(xdc.contains("set_multicycle_path 3"));
    }

    #[test]
    fn test_io_direction() {
        assert_ne!(IoDirection::Input, IoDirection::Output);
        assert_ne!(IoDirection::Bidir, IoDirection::Input);
    }
}
