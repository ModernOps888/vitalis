//! Bare-metal / embedded target support for Vitalis.
//!
//! - **no_std mode**: Compile without standard library
//! - **Interrupt vector table (IVT)**: Static interrupt registration
//! - **MMIO**: Memory-mapped I/O register abstraction
//! - **DMA**: DMA descriptor and channel configuration
//! - **Linker scripts**: Generate linker scripts for common MCUs
//! - **HAL traits**: Hardware abstraction layer interfaces
//! - **ARM / RISC-V targets**: Multi-architecture support

use std::collections::HashMap;

// ── Memory Map ──────────────────────────────────────────────────────

/// A memory region in the MCU address space.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub name: String,
    pub base_address: u32,
    pub size: u32,
    pub permissions: Permissions,
    pub region_type: RegionType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permissions {
    pub fn rwx() -> Self { Self { read: true, write: true, execute: true } }
    pub fn rw() -> Self { Self { read: true, write: true, execute: false } }
    pub fn ro() -> Self { Self { read: true, write: false, execute: false } }
    pub fn rx() -> Self { Self { read: true, write: false, execute: true } }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegionType {
    Flash,
    Ram,
    Peripheral,
    ExternalRam,
    Bootloader,
    Reserved,
}

/// MCU memory map.
pub struct MemoryMap {
    regions: Vec<MemoryRegion>,
}

impl MemoryMap {
    pub fn new() -> Self { Self { regions: Vec::new() } }

    pub fn add_region(&mut self, region: MemoryRegion) {
        self.regions.push(region);
    }

    pub fn region_at(&self, addr: u32) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| addr >= r.base_address && addr < r.base_address + r.size)
    }

    pub fn total_flash(&self) -> u32 {
        self.regions.iter().filter(|r| r.region_type == RegionType::Flash).map(|r| r.size).sum()
    }

    pub fn total_ram(&self) -> u32 {
        self.regions.iter().filter(|r| r.region_type == RegionType::Ram).map(|r| r.size).sum()
    }

    pub fn regions(&self) -> &[MemoryRegion] {
        &self.regions
    }
}

// ── MMIO ────────────────────────────────────────────────────────────

/// An MMIO register.
#[derive(Debug, Clone)]
pub struct MmioRegister {
    pub name: String,
    pub offset: u32,
    pub width: RegisterWidth,
    pub access: AccessMode,
    pub reset_value: u32,
    pub fields: Vec<BitField>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegisterWidth {
    Bits8,
    Bits16,
    Bits32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    ReadClear,
    ReadWriteOnce,
}

/// A bit field within a register.
#[derive(Debug, Clone)]
pub struct BitField {
    pub name: String,
    pub bit_offset: u8,
    pub bit_width: u8,
    pub description: String,
}

impl BitField {
    pub fn mask(&self) -> u32 {
        ((1u32 << self.bit_width) - 1) << self.bit_offset
    }

    pub fn extract(&self, value: u32) -> u32 {
        (value >> self.bit_offset) & ((1u32 << self.bit_width) - 1)
    }

    pub fn insert(&self, reg_value: u32, field_value: u32) -> u32 {
        let mask = self.mask();
        (reg_value & !mask) | ((field_value << self.bit_offset) & mask)
    }
}

/// A peripheral (group of MMIO registers).
#[derive(Debug, Clone)]
pub struct Peripheral {
    pub name: String,
    pub base_address: u32,
    pub registers: Vec<MmioRegister>,
}

impl Peripheral {
    pub fn register(&self, name: &str) -> Option<&MmioRegister> {
        self.registers.iter().find(|r| r.name == name)
    }

    pub fn register_address(&self, name: &str) -> Option<u32> {
        self.register(name).map(|r| self.base_address + r.offset)
    }
}

// ── Interrupt Vector Table ──────────────────────────────────────────

/// Interrupt priority.
pub type InterruptPriority = u8;

/// An interrupt handler entry.
#[derive(Debug, Clone)]
pub struct InterruptHandler {
    pub number: u32,
    pub name: String,
    pub handler_name: String,
    pub priority: InterruptPriority,
    pub enabled: bool,
}

/// Interrupt vector table.
pub struct InterruptVectorTable {
    handlers: Vec<InterruptHandler>,
    max_interrupts: u32,
}

impl InterruptVectorTable {
    pub fn new(max_interrupts: u32) -> Self {
        Self { handlers: Vec::new(), max_interrupts }
    }

    pub fn register(&mut self, handler: InterruptHandler) -> Result<(), String> {
        if handler.number >= self.max_interrupts {
            return Err(format!("IRQ {} exceeds max {}", handler.number, self.max_interrupts));
        }
        if self.handlers.iter().any(|h| h.number == handler.number) {
            return Err(format!("IRQ {} already registered", handler.number));
        }
        self.handlers.push(handler);
        Ok(())
    }

    pub fn handler_for(&self, irq: u32) -> Option<&InterruptHandler> {
        self.handlers.iter().find(|h| h.number == irq)
    }

    pub fn enabled_handlers(&self) -> Vec<&InterruptHandler> {
        self.handlers.iter().filter(|h| h.enabled).collect()
    }

    pub fn count(&self) -> usize {
        self.handlers.len()
    }
}

// ── DMA ─────────────────────────────────────────────────────────────

/// DMA transfer direction.
#[derive(Debug, Clone, PartialEq)]
pub enum DmaDirection {
    MemoryToMemory,
    MemoryToPeripheral,
    PeripheralToMemory,
}

/// DMA transfer width.
#[derive(Debug, Clone, PartialEq)]
pub enum DmaWidth {
    Byte,
    HalfWord,
    Word,
}

/// DMA channel configuration.
#[derive(Debug, Clone)]
pub struct DmaConfig {
    pub channel: u8,
    pub direction: DmaDirection,
    pub source_addr: u32,
    pub dest_addr: u32,
    pub transfer_count: u32,
    pub data_width: DmaWidth,
    pub source_increment: bool,
    pub dest_increment: bool,
    pub circular: bool,
    pub priority: u8,
}

/// DMA descriptor for scatter-gather.
#[derive(Debug, Clone)]
pub struct DmaDescriptor {
    pub source: u32,
    pub dest: u32,
    pub count: u32,
    pub next: Option<u32>,
    pub config: u32,
}

impl DmaDescriptor {
    pub fn is_last(&self) -> bool {
        self.next.is_none()
    }
}

// ── Linker Script Generation ────────────────────────────────────────

/// Target architecture.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetArch {
    ArmCortexM0,
    ArmCortexM3,
    ArmCortexM4,
    ArmCortexM7,
    RiscV32,
    RiscV64,
}

impl TargetArch {
    pub fn arch_name(&self) -> &str {
        match self {
            TargetArch::ArmCortexM0 => "armv6-m",
            TargetArch::ArmCortexM3 => "armv7-m",
            TargetArch::ArmCortexM4 => "armv7e-m",
            TargetArch::ArmCortexM7 => "armv7e-m",
            TargetArch::RiscV32 => "riscv32",
            TargetArch::RiscV64 => "riscv64",
        }
    }

    pub fn is_arm(&self) -> bool {
        matches!(self, TargetArch::ArmCortexM0 | TargetArch::ArmCortexM3 |
                       TargetArch::ArmCortexM4 | TargetArch::ArmCortexM7)
    }
}

/// Generate a linker script for the given memory map.
pub fn generate_linker_script(memory_map: &MemoryMap, arch: &TargetArch, stack_size: u32) -> String {
    let mut ld = String::new();
    ld.push_str("/* Generated by Vitalis Embedded Target */\n\n");

    // Entry.
    if arch.is_arm() {
        ld.push_str("ENTRY(Reset_Handler)\n\n");
    } else {
        ld.push_str("ENTRY(_start)\n\n");
    }

    // Stack.
    ld.push_str(&format!("_stack_size = 0x{:X};\n\n", stack_size));

    // Memory regions.
    ld.push_str("MEMORY\n{\n");
    for region in memory_map.regions() {
        let attr = match region.region_type {
            RegionType::Flash => "rx",
            RegionType::Ram => "rwx",
            _ => "rw",
        };
        ld.push_str(&format!("    {} ({}) : ORIGIN = 0x{:08X}, LENGTH = 0x{:X}\n",
            region.name, attr, region.base_address, region.size));
    }
    ld.push_str("}\n\n");

    // Sections.
    ld.push_str("SECTIONS\n{\n");
    ld.push_str("    .text : { *(.text*) } > FLASH\n");
    ld.push_str("    .rodata : { *(.rodata*) } > FLASH\n");
    ld.push_str("    .data : { *(.data*) } > RAM AT > FLASH\n");
    ld.push_str("    .bss : { *(.bss*) } > RAM\n");
    ld.push_str("    ._stack (NOLOAD) : {\n");
    ld.push_str("        . = ALIGN(8);\n");
    ld.push_str("        . = . + _stack_size;\n");
    ld.push_str("        _stack_top = .;\n");
    ld.push_str("    } > RAM\n");
    ld.push_str("}\n");
    ld
}

// ── HAL Traits ──────────────────────────────────────────────────────

/// GPIO pin state.
#[derive(Debug, Clone, PartialEq)]
pub enum PinState {
    High,
    Low,
}

/// GPIO pin mode.
#[derive(Debug, Clone, PartialEq)]
pub enum PinMode {
    Input,
    Output,
    AlternateFunction(u8),
    Analog,
}

/// A HAL trait descriptor (for codegen).
#[derive(Debug, Clone)]
pub struct HalTrait {
    pub name: String,
    pub methods: Vec<HalMethod>,
    pub description: String,
}

/// A method in a HAL trait.
#[derive(Debug, Clone)]
pub struct HalMethod {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub return_type: String,
    pub is_fallible: bool,
}

/// Standard HAL traits.
pub fn standard_hal_traits() -> Vec<HalTrait> {
    vec![
        HalTrait {
            name: "DigitalOutput".into(),
            description: "GPIO output pin control".into(),
            methods: vec![
                HalMethod { name: "set_high".into(), params: vec![], return_type: "()".into(), is_fallible: false },
                HalMethod { name: "set_low".into(), params: vec![], return_type: "()".into(), is_fallible: false },
                HalMethod { name: "toggle".into(), params: vec![], return_type: "()".into(), is_fallible: false },
            ],
        },
        HalTrait {
            name: "DigitalInput".into(),
            description: "GPIO input pin reading".into(),
            methods: vec![
                HalMethod { name: "is_high".into(), params: vec![], return_type: "bool".into(), is_fallible: false },
                HalMethod { name: "is_low".into(), params: vec![], return_type: "bool".into(), is_fallible: false },
            ],
        },
        HalTrait {
            name: "Serial".into(),
            description: "UART serial communication".into(),
            methods: vec![
                HalMethod { name: "write_byte".into(), params: vec![("byte".into(), "u8".into())], return_type: "Result<(), Error>".into(), is_fallible: true },
                HalMethod { name: "read_byte".into(), params: vec![], return_type: "Result<u8, Error>".into(), is_fallible: true },
                HalMethod { name: "flush".into(), params: vec![], return_type: "Result<(), Error>".into(), is_fallible: true },
            ],
        },
        HalTrait {
            name: "Spi".into(),
            description: "SPI bus communication".into(),
            methods: vec![
                HalMethod { name: "transfer".into(), params: vec![("data".into(), "&[u8]".into())], return_type: "Result<Vec<u8>, Error>".into(), is_fallible: true },
            ],
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_map() {
        let mut mm = MemoryMap::new();
        mm.add_region(MemoryRegion {
            name: "FLASH".into(), base_address: 0x0800_0000,
            size: 256 * 1024, permissions: Permissions::rx(),
            region_type: RegionType::Flash,
        });
        mm.add_region(MemoryRegion {
            name: "RAM".into(), base_address: 0x2000_0000,
            size: 64 * 1024, permissions: Permissions::rwx(),
            region_type: RegionType::Ram,
        });
        assert_eq!(mm.total_flash(), 256 * 1024);
        assert_eq!(mm.total_ram(), 64 * 1024);
    }

    #[test]
    fn test_region_at() {
        let mut mm = MemoryMap::new();
        mm.add_region(MemoryRegion {
            name: "FLASH".into(), base_address: 0x0800_0000,
            size: 256 * 1024, permissions: Permissions::rx(),
            region_type: RegionType::Flash,
        });
        assert!(mm.region_at(0x0800_0100).is_some());
        assert!(mm.region_at(0x0000_0000).is_none());
    }

    #[test]
    fn test_bit_field_mask() {
        let field = BitField { name: "EN".into(), bit_offset: 3, bit_width: 1, description: "Enable".into() };
        assert_eq!(field.mask(), 0b1000);
    }

    #[test]
    fn test_bit_field_extract() {
        let field = BitField { name: "PRIO".into(), bit_offset: 4, bit_width: 4, description: "Priority".into() };
        assert_eq!(field.extract(0xA0), 0xA);
    }

    #[test]
    fn test_bit_field_insert() {
        let field = BitField { name: "VAL".into(), bit_offset: 0, bit_width: 8, description: "Value".into() };
        assert_eq!(field.insert(0xFF00, 0x42), 0xFF42);
    }

    #[test]
    fn test_peripheral() {
        let periph = Peripheral {
            name: "GPIOA".into(), base_address: 0x4002_0000,
            registers: vec![
                MmioRegister {
                    name: "ODR".into(), offset: 0x14,
                    width: RegisterWidth::Bits32, access: AccessMode::ReadWrite,
                    reset_value: 0, fields: vec![],
                },
            ],
        };
        assert_eq!(periph.register_address("ODR"), Some(0x4002_0014));
    }

    #[test]
    fn test_ivt_register() {
        let mut ivt = InterruptVectorTable::new(64);
        ivt.register(InterruptHandler {
            number: 0, name: "SysTick".into(),
            handler_name: "systick_handler".into(),
            priority: 0, enabled: true,
        }).unwrap();
        assert_eq!(ivt.count(), 1);
        assert!(ivt.handler_for(0).is_some());
    }

    #[test]
    fn test_ivt_duplicate() {
        let mut ivt = InterruptVectorTable::new(64);
        ivt.register(InterruptHandler {
            number: 5, name: "TIM2".into(),
            handler_name: "tim2".into(), priority: 3, enabled: true,
        }).unwrap();
        let result = ivt.register(InterruptHandler {
            number: 5, name: "TIM2_dup".into(),
            handler_name: "tim2_dup".into(), priority: 3, enabled: true,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_ivt_overflow() {
        let mut ivt = InterruptVectorTable::new(4);
        let result = ivt.register(InterruptHandler {
            number: 10, name: "X".into(),
            handler_name: "x".into(), priority: 0, enabled: true,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_dma_config() {
        let config = DmaConfig {
            channel: 0, direction: DmaDirection::MemoryToPeripheral,
            source_addr: 0x2000_0000, dest_addr: 0x4001_3004,
            transfer_count: 256, data_width: DmaWidth::Byte,
            source_increment: true, dest_increment: false,
            circular: false, priority: 1,
        };
        assert_eq!(config.direction, DmaDirection::MemoryToPeripheral);
    }

    #[test]
    fn test_dma_descriptor_last() {
        let desc = DmaDescriptor {
            source: 0x2000_0000, dest: 0x4000_0000,
            count: 128, next: None, config: 0,
        };
        assert!(desc.is_last());
    }

    #[test]
    fn test_linker_script() {
        let mut mm = MemoryMap::new();
        mm.add_region(MemoryRegion {
            name: "FLASH".into(), base_address: 0x0800_0000,
            size: 256 * 1024, permissions: Permissions::rx(),
            region_type: RegionType::Flash,
        });
        mm.add_region(MemoryRegion {
            name: "RAM".into(), base_address: 0x2000_0000,
            size: 64 * 1024, permissions: Permissions::rwx(),
            region_type: RegionType::Ram,
        });
        let ld = generate_linker_script(&mm, &TargetArch::ArmCortexM4, 0x2000);
        assert!(ld.contains("MEMORY"));
        assert!(ld.contains("FLASH"));
        assert!(ld.contains("Reset_Handler"));
    }

    #[test]
    fn test_target_arch() {
        assert!(TargetArch::ArmCortexM4.is_arm());
        assert!(!TargetArch::RiscV32.is_arm());
        assert_eq!(TargetArch::RiscV64.arch_name(), "riscv64");
    }

    #[test]
    fn test_hal_traits() {
        let traits = standard_hal_traits();
        assert!(traits.len() >= 4);
        assert!(traits.iter().any(|t| t.name == "DigitalOutput"));
        assert!(traits.iter().any(|t| t.name == "Serial"));
    }

    #[test]
    fn test_permissions() {
        let p = Permissions::rwx();
        assert!(p.read && p.write && p.execute);
        let ro = Permissions::ro();
        assert!(ro.read && !ro.write && !ro.execute);
    }

    #[test]
    fn test_pin_state() {
        assert_ne!(PinState::High, PinState::Low);
    }

    #[test]
    fn test_pin_mode() {
        let mode = PinMode::AlternateFunction(7);
        match mode {
            PinMode::AlternateFunction(n) => assert_eq!(n, 7),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_access_mode() {
        assert_ne!(AccessMode::ReadOnly, AccessMode::WriteOnly);
        assert_ne!(AccessMode::ReadWrite, AccessMode::ReadClear);
    }

    #[test]
    fn test_enabled_handlers() {
        let mut ivt = InterruptVectorTable::new(64);
        ivt.register(InterruptHandler {
            number: 0, name: "A".into(), handler_name: "a".into(),
            priority: 0, enabled: true,
        }).unwrap();
        ivt.register(InterruptHandler {
            number: 1, name: "B".into(), handler_name: "b".into(),
            priority: 0, enabled: false,
        }).unwrap();
        assert_eq!(ivt.enabled_handlers().len(), 1);
    }
}
