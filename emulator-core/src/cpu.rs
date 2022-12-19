pub mod arm;
pub mod thumb;

use std::fmt::Display;
use std::{fmt::Debug, ops::RangeInclusive};

use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::BitManipulation;

use self::arm::ArmInstruction;
use self::thumb::ThumbInstruction;

#[derive(Clone, Default)]
struct ModeRegisters {
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
    r12: u32,
    r13: u32, // SP
    r14: u32, // LR
    r15: u32, // PC
    spsr: u32,
}

#[derive(Clone)]
pub struct Cpu {
    current_registers: ModeRegisters,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
    r12: u32,
    r13: u32,
    r14: u32,
    r15: u32,
    r8_fiq: u32,
    r9_fiq: u32,
    r10_fiq: u32,
    r11_fiq: u32,
    r12_fiq: u32,
    r13_fiq: u32,
    r14_fiq: u32,
    spsr_fiq: u32,
    r13_svc: u32,
    r14_svc: u32,
    spsr_svc: u32,
    r13_abt: u32,
    r14_abt: u32,
    spsr_abt: u32,
    r13_irq: u32,
    r14_irq: u32,
    spsr_irq: u32,
    r13_und: u32,
    r14_und: u32,
    spsr_und: u32,
    cpsr: u32,
    cycle_count: u64,
    pub bus: Bus,
    prefetch_opcode: Option<u32>,
    pre_decode_arm: Option<ArmInstruction>,
    pre_decode_thumb: Option<ThumbInstruction>,
}

#[derive(Clone, Copy, Debug)]
struct InstructionCyclesInfo {
    i: u8, // internal cycle
    n: u8, // non-sequential cycle
    s: u8, // sequential cycle
}

#[derive(Clone, Copy, Debug)]
enum ExceptionType {
    Reset,
    Undefined,
    Swi,
    PrefetchAbort,
    DataAbort,
    AddressExceeds26Bit,
    InterruptRequest,
    FastInterruptRequest,
}

impl Cpu {
    pub fn new(cartridge: Cartridge) -> Self {
        // treated as SPSR in system and user mode
        let cpsr = Self::SYSTEM_MODE_BITS;

        let current_registers = ModeRegisters::default();
        Self {
            current_registers,
            r0: 0,
            r1: 0,
            r2: 0,
            r3: 0,
            r4: 0,
            r5: 0,
            r6: 0,
            r7: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            r8_fiq: 0,
            r9_fiq: 0,
            r10_fiq: 0,
            r11_fiq: 0,
            r12_fiq: 0,
            r13_fiq: 0,
            r14_fiq: 0,
            spsr_fiq: 0,
            r13_svc: 0,
            r14_svc: 0,
            spsr_svc: 0,
            r13_abt: 0,
            r14_abt: 0,
            spsr_abt: 0,
            r13_irq: 0,
            r14_irq: 0,
            spsr_irq: 0,
            r13_und: 0,
            r14_und: 0,
            spsr_und: 0,
            cpsr,
            cycle_count: 0,
            bus: Bus::new(cartridge),
            pre_decode_arm: None,
            prefetch_opcode: None,
            pre_decode_thumb: None,
        }
    }

    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }
}

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r0 = self.read_register(Register::R0, |_| unreachable!());
        let r1 = self.read_register(Register::R1, |_| unreachable!());
        let r2 = self.read_register(Register::R2, |_| unreachable!());
        let r3 = self.read_register(Register::R3, |_| unreachable!());
        let r4 = self.read_register(Register::R4, |_| unreachable!());
        let r5 = self.read_register(Register::R5, |_| unreachable!());
        let r6 = self.read_register(Register::R6, |_| unreachable!());
        let r7 = self.read_register(Register::R7, |_| unreachable!());
        let r8 = self.read_register(Register::R8, |_| unreachable!());
        let r9 = self.read_register(Register::R9, |_| unreachable!());
        let r10 = self.read_register(Register::R10, |_| unreachable!());
        let r11 = self.read_register(Register::R11, |_| unreachable!());
        let r12 = self.read_register(Register::R12, |_| unreachable!());
        let r13 = self.read_register(Register::R13, |_| unreachable!());
        let r14 = self.read_register(Register::R14, |_| unreachable!());
        let r15 = self.read_register(Register::R15, |pc| pc);

        writeln!(
            f,
            " R0: 0x{:08x}  R1: 0x{:08x}  R2: 0x{:08x}  R3: 0x{:08x}",
            r0, r1, r2, r3
        )?;
        writeln!(
            f,
            " R4: 0x{:08x}  R5: 0x{:08x}  R6: 0x{:08x}  R7: 0x{:08x}",
            r4, r5, r6, r7
        )?;
        writeln!(
            f,
            " R8: 0x{:08x}  R9: 0x{:08x} R10: 0x{:08x} R11: 0x{:08x}",
            r8, r9, r10, r11
        )?;
        write!(
            f,
            "R12: 0x{:08x} R13: 0x{:08x} R14: 0x{:08x} R15: 0x{:08x}",
            r12, r13, r14, r15
        )?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuMode {
    User,
    Fiq,
    Irq,
    Supervisor,
    Abort,
    Undefined,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Register {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13, // SP
    R14, // LR
    R15, // PR
    Cpsr,
    Spsr,
}

impl Register {
    pub fn from_index(index: u32) -> Self {
        match index {
            0 => Register::R0,
            1 => Register::R1,
            2 => Register::R2,
            3 => Register::R3,
            4 => Register::R4,
            5 => Register::R5,
            6 => Register::R6,
            7 => Register::R7,
            8 => Register::R8,
            9 => Register::R9,
            10 => Register::R10,
            11 => Register::R11,
            12 => Register::R12,
            13 => Register::R13,
            14 => Register::R14,
            15 => Register::R15,
            _ => unreachable!(),
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::R0 => f.write_str("r0"),
            Self::R1 => f.write_str("r1"),
            Self::R2 => f.write_str("r2"),
            Self::R3 => f.write_str("r3"),
            Self::R4 => f.write_str("r4"),
            Self::R5 => f.write_str("r5"),
            Self::R6 => f.write_str("r6"),
            Self::R7 => f.write_str("r7"),
            Self::R8 => f.write_str("r8"),
            Self::R9 => f.write_str("r9"),
            Self::R10 => f.write_str("r10"),
            Self::R11 => f.write_str("r11"),
            Self::R12 => f.write_str("r12"),
            Self::R13 => f.write_str("sp"),
            Self::R14 => f.write_str("lr"),
            Self::R15 => f.write_str("pc"),
            Self::Cpsr => f.write_str("cpsr"),
            Self::Spsr => f.write_str("spsr"),
            _ => todo!("{:?}", self),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InstructionCondition {
    Equal,
    NotEqual,
    UnsignedHigherOrSame,
    UnsignedLower,
    SignedNegative,
    SignedPositiveOrZero,
    SignedOverflow,
    SignedNoOverflow,
    UnsignedHigher,
    UnsignedLowerOrSame,
    SignedGreaterOrEqual,
    SignedLessThan,
    SignedGreaterThan,
    SignedLessOrEqual,
    Always,
    Never,
}

impl Display for InstructionCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal => f.write_str("eq"),
            Self::NotEqual => f.write_str("ne"),
            Self::UnsignedHigherOrSame => f.write_str("cs"),
            Self::UnsignedLower => f.write_str("cc"),
            Self::SignedNegative => f.write_str("mi"),
            Self::SignedPositiveOrZero => f.write_str("pl"),
            Self::SignedOverflow => f.write_str("vs"),
            Self::SignedNoOverflow => f.write_str("vc"),
            Self::UnsignedHigher => f.write_str("hi"),
            Self::UnsignedLowerOrSame => f.write_str("ls"),
            Self::SignedGreaterOrEqual => f.write_str("ge"),
            Self::SignedLessThan => f.write_str("lt"),
            Self::SignedGreaterThan => f.write_str("gt"),
            Self::SignedLessOrEqual => f.write_str("le"),
            Self::Always => Ok(()),
            Self::Never => f.write_str("_NEVER"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InstructionSet {
    Arm,
    Thumb,
}

#[derive(Clone, Copy, Debug)]
pub enum ShiftType {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

impl ShiftType {
    fn evaluate(self, value: u32, shift: u32) -> u32 {
        assert!(shift < 32);

        match self {
            ShiftType::Lsl => value << shift,
            ShiftType::Lsr => value >> shift,
            ShiftType::Asr => ((value as i32) >> shift) as u32,
            ShiftType::Ror => value.rotate_right(shift),
        }
    }
}

impl Display for ShiftType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShiftType::Lsl => f.write_str("lsl"),
            ShiftType::Lsr => f.write_str("lsr"),
            ShiftType::Asr => f.write_str("asr"),
            ShiftType::Ror => f.write_str("ror"),
        }
    }
}

impl Cpu {
    fn write_register(&mut self, value: u32, register: Register) {
        let instruction_mode = self.get_instruction_mode();

        match register {
            Register::R0 => self.current_registers.r0 = value,
            Register::R1 => self.current_registers.r1 = value,
            Register::R2 => self.current_registers.r2 = value,
            Register::R3 => self.current_registers.r3 = value,
            Register::R4 => self.current_registers.r4 = value,
            Register::R5 => self.current_registers.r5 = value,
            Register::R6 => self.current_registers.r6 = value,
            Register::R7 => self.current_registers.r7 = value,
            Register::R8 => self.current_registers.r8 = value,
            Register::R9 => self.current_registers.r9 = value,
            Register::R10 => self.current_registers.r10 = value,
            Register::R11 => self.current_registers.r11 = value,
            Register::R12 => self.current_registers.r12 = value,
            Register::R13 => self.current_registers.r13 = value,
            Register::R14 => self.current_registers.r14 = value,
            Register::R15 => match instruction_mode {
                InstructionSet::Arm => {
                    if value & 0b11 != 0 {
                        log::warn!(
                            "writing to ARM PC with unaligned value: 0x{:08X}, force aligning",
                            value
                        );
                    }
                    self.current_registers.r15 = value & (!0b11);
                }
                InstructionSet::Thumb => {
                    if value & 0b1 != 0 {
                        log::warn!(
                            "writing to Thumb PC with unaligned value: 0x{:08X}, force aligning",
                            value
                        );
                    }
                    self.current_registers.r15 = value & (!0b1);
                }
            },
            Register::Spsr => self.current_registers.spsr = value,
            Register::Cpsr => {
                let old_mode = self.get_cpu_mode();
                self.cpsr = value;
                let new_mode = self.get_cpu_mode();

                // when a mode switch occurs:
                // - store all registers from current mode registers into mode-agnostic register storage.
                // - load all registers from new mode mode-agnostic register storage into current mode registers
                if old_mode != new_mode {
                    match old_mode {
                        CpuMode::System | CpuMode::User => {
                            self.r0 = self.current_registers.r0;
                            self.r1 = self.current_registers.r1;
                            self.r2 = self.current_registers.r2;
                            self.r3 = self.current_registers.r3;
                            self.r4 = self.current_registers.r4;
                            self.r5 = self.current_registers.r5;
                            self.r6 = self.current_registers.r6;
                            self.r7 = self.current_registers.r7;
                            self.r8 = self.current_registers.r8;
                            self.r9 = self.current_registers.r9;
                            self.r10 = self.current_registers.r10;
                            self.r11 = self.current_registers.r11;
                            self.r12 = self.current_registers.r12;
                            self.r13 = self.current_registers.r13;
                            self.r14 = self.current_registers.r14;
                            self.r15 = self.current_registers.r15;
                        }
                        CpuMode::Fiq => {
                            self.r0 = self.current_registers.r0;
                            self.r1 = self.current_registers.r1;
                            self.r2 = self.current_registers.r2;
                            self.r3 = self.current_registers.r3;
                            self.r4 = self.current_registers.r4;
                            self.r5 = self.current_registers.r5;
                            self.r6 = self.current_registers.r6;
                            self.r7 = self.current_registers.r7;
                            self.r8_fiq = self.current_registers.r8;
                            self.r9_fiq = self.current_registers.r9;
                            self.r10_fiq = self.current_registers.r10;
                            self.r11_fiq = self.current_registers.r11;
                            self.r12_fiq = self.current_registers.r12;
                            self.r13_fiq = self.current_registers.r13;
                            self.r14_fiq = self.current_registers.r14;
                            self.r15 = self.current_registers.r15;
                            self.spsr_fiq = self.current_registers.spsr;
                        }
                        CpuMode::Supervisor => {
                            self.r0 = self.current_registers.r0;
                            self.r1 = self.current_registers.r1;
                            self.r2 = self.current_registers.r2;
                            self.r3 = self.current_registers.r3;
                            self.r4 = self.current_registers.r4;
                            self.r5 = self.current_registers.r5;
                            self.r6 = self.current_registers.r6;
                            self.r7 = self.current_registers.r7;
                            self.r8 = self.current_registers.r8;
                            self.r9 = self.current_registers.r9;
                            self.r10 = self.current_registers.r10;
                            self.r11 = self.current_registers.r11;
                            self.r12 = self.current_registers.r12;
                            self.r13_svc = self.current_registers.r13;
                            self.r14_svc = self.current_registers.r14;
                            self.r15 = self.current_registers.r15;
                            self.spsr_svc = self.current_registers.spsr;
                        }
                        CpuMode::Abort => {
                            self.r0 = self.current_registers.r0;
                            self.r1 = self.current_registers.r1;
                            self.r2 = self.current_registers.r2;
                            self.r3 = self.current_registers.r3;
                            self.r4 = self.current_registers.r4;
                            self.r5 = self.current_registers.r5;
                            self.r6 = self.current_registers.r6;
                            self.r7 = self.current_registers.r7;
                            self.r8 = self.current_registers.r8;
                            self.r9 = self.current_registers.r9;
                            self.r10 = self.current_registers.r10;
                            self.r11 = self.current_registers.r11;
                            self.r12 = self.current_registers.r12;
                            self.r13_abt = self.current_registers.r13;
                            self.r14_abt = self.current_registers.r14;
                            self.r15 = self.current_registers.r15;
                            self.spsr_abt = self.current_registers.spsr;
                        }
                        CpuMode::Irq => {
                            self.r0 = self.current_registers.r0;
                            self.r1 = self.current_registers.r1;
                            self.r2 = self.current_registers.r2;
                            self.r3 = self.current_registers.r3;
                            self.r4 = self.current_registers.r4;
                            self.r5 = self.current_registers.r5;
                            self.r6 = self.current_registers.r6;
                            self.r7 = self.current_registers.r7;
                            self.r8 = self.current_registers.r8;
                            self.r9 = self.current_registers.r9;
                            self.r10 = self.current_registers.r10;
                            self.r11 = self.current_registers.r11;
                            self.r12 = self.current_registers.r12;
                            self.r13_irq = self.current_registers.r13;
                            self.r14_irq = self.current_registers.r14;
                            self.r15 = self.current_registers.r15;
                            self.spsr_irq = self.current_registers.spsr;
                        }
                        CpuMode::Undefined => {
                            self.r0 = self.current_registers.r0;
                            self.r1 = self.current_registers.r1;
                            self.r2 = self.current_registers.r2;
                            self.r3 = self.current_registers.r3;
                            self.r4 = self.current_registers.r4;
                            self.r5 = self.current_registers.r5;
                            self.r6 = self.current_registers.r6;
                            self.r7 = self.current_registers.r7;
                            self.r8 = self.current_registers.r8;
                            self.r9 = self.current_registers.r9;
                            self.r10 = self.current_registers.r10;
                            self.r11 = self.current_registers.r11;
                            self.r12 = self.current_registers.r12;
                            self.r13_und = self.current_registers.r13;
                            self.r14_und = self.current_registers.r14;
                            self.r15 = self.current_registers.r15;
                            self.spsr_und = self.current_registers.spsr;
                        }
                    }

                    match new_mode {
                        CpuMode::User | CpuMode::System => {
                            self.current_registers.r0 = self.r0;
                            self.current_registers.r1 = self.r1;
                            self.current_registers.r2 = self.r2;
                            self.current_registers.r3 = self.r3;
                            self.current_registers.r4 = self.r4;
                            self.current_registers.r5 = self.r5;
                            self.current_registers.r6 = self.r6;
                            self.current_registers.r7 = self.r7;
                            self.current_registers.r8 = self.r8;
                            self.current_registers.r9 = self.r9;
                            self.current_registers.r10 = self.r10;
                            self.current_registers.r11 = self.r11;
                            self.current_registers.r12 = self.r12;
                            self.current_registers.r13 = self.r13;
                            self.current_registers.r14 = self.r14;
                            self.current_registers.r15 = self.r15;
                        }
                        CpuMode::Fiq => {
                            self.current_registers.r0 = self.r0;
                            self.current_registers.r1 = self.r1;
                            self.current_registers.r2 = self.r2;
                            self.current_registers.r3 = self.r3;
                            self.current_registers.r4 = self.r4;
                            self.current_registers.r5 = self.r5;
                            self.current_registers.r6 = self.r6;
                            self.current_registers.r7 = self.r7;
                            self.current_registers.r8 = self.r8_fiq;
                            self.current_registers.r9 = self.r9_fiq;
                            self.current_registers.r10 = self.r10_fiq;
                            self.current_registers.r11 = self.r11_fiq;
                            self.current_registers.r12 = self.r12_fiq;
                            self.current_registers.r13 = self.r13_fiq;
                            self.current_registers.r14 = self.r14_fiq;
                            self.current_registers.r15 = self.r15;
                            self.current_registers.spsr = self.spsr_fiq;
                        }
                        CpuMode::Supervisor => {
                            self.current_registers.r0 = self.r0;
                            self.current_registers.r1 = self.r1;
                            self.current_registers.r2 = self.r2;
                            self.current_registers.r3 = self.r3;
                            self.current_registers.r4 = self.r4;
                            self.current_registers.r5 = self.r5;
                            self.current_registers.r6 = self.r6;
                            self.current_registers.r7 = self.r7;
                            self.current_registers.r8 = self.r8;
                            self.current_registers.r9 = self.r9;
                            self.current_registers.r10 = self.r10;
                            self.current_registers.r11 = self.r11;
                            self.current_registers.r12 = self.r12;
                            self.current_registers.r13 = self.r13_svc;
                            self.current_registers.r14 = self.r14_svc;
                            self.current_registers.r15 = self.r15;
                            self.current_registers.spsr = self.spsr_svc;
                        }
                        CpuMode::Abort => {
                            self.current_registers.r0 = self.r0;
                            self.current_registers.r1 = self.r1;
                            self.current_registers.r2 = self.r2;
                            self.current_registers.r3 = self.r3;
                            self.current_registers.r4 = self.r4;
                            self.current_registers.r5 = self.r5;
                            self.current_registers.r6 = self.r6;
                            self.current_registers.r7 = self.r7;
                            self.current_registers.r8 = self.r8;
                            self.current_registers.r9 = self.r9;
                            self.current_registers.r10 = self.r10;
                            self.current_registers.r11 = self.r11;
                            self.current_registers.r12 = self.r12;
                            self.current_registers.r13 = self.r13_abt;
                            self.current_registers.r14 = self.r14_abt;
                            self.current_registers.r15 = self.r15;
                            self.current_registers.spsr = self.spsr_abt;
                        }
                        CpuMode::Irq => {
                            self.current_registers.r0 = self.r0;
                            self.current_registers.r1 = self.r1;
                            self.current_registers.r2 = self.r2;
                            self.current_registers.r3 = self.r3;
                            self.current_registers.r4 = self.r4;
                            self.current_registers.r5 = self.r5;
                            self.current_registers.r6 = self.r6;
                            self.current_registers.r7 = self.r7;
                            self.current_registers.r8 = self.r8;
                            self.current_registers.r9 = self.r9;
                            self.current_registers.r10 = self.r10;
                            self.current_registers.r11 = self.r11;
                            self.current_registers.r12 = self.r12;
                            self.current_registers.r13 = self.r13_irq;
                            self.current_registers.r14 = self.r14_irq;
                            self.current_registers.r15 = self.r15;
                            self.current_registers.spsr = self.spsr_irq;
                        }
                        CpuMode::Undefined => {
                            self.current_registers.r0 = self.r0;
                            self.current_registers.r1 = self.r1;
                            self.current_registers.r2 = self.r2;
                            self.current_registers.r3 = self.r3;
                            self.current_registers.r4 = self.r4;
                            self.current_registers.r5 = self.r5;
                            self.current_registers.r6 = self.r6;
                            self.current_registers.r7 = self.r7;
                            self.current_registers.r8 = self.r8;
                            self.current_registers.r9 = self.r9;
                            self.current_registers.r10 = self.r10;
                            self.current_registers.r11 = self.r11;
                            self.current_registers.r12 = self.r12;
                            self.current_registers.r13 = self.r13_und;
                            self.current_registers.r14 = self.r14_und;
                            self.current_registers.r15 = self.r15;
                            self.current_registers.spsr = self.spsr_und;
                        }
                    }
                }
            }
        }
    }

    pub fn read_register(&self, register: Register, pc_calculation: fn(u32) -> u32) -> u32 {
        match register {
            Register::R0 => self.current_registers.r0,
            Register::R1 => self.current_registers.r1,
            Register::R2 => self.current_registers.r2,
            Register::R3 => self.current_registers.r3,
            Register::R4 => self.current_registers.r4,
            Register::R5 => self.current_registers.r5,
            Register::R6 => self.current_registers.r6,
            Register::R7 => self.current_registers.r7,
            Register::R8 => self.current_registers.r8,
            Register::R9 => self.current_registers.r9,
            Register::R10 => self.current_registers.r10,
            Register::R11 => self.current_registers.r11,
            Register::R12 => self.current_registers.r12,
            Register::R13 => self.current_registers.r13,
            Register::R14 => self.current_registers.r14,
            Register::R15 => pc_calculation(self.current_registers.r15),
            Register::Spsr => self.current_registers.spsr,
            Register::Cpsr => self.cpsr,
        }
    }

    fn read_user_register(&self, register: Register, pc_calculation: fn(u32) -> u32) -> u32 {
        match register {
            Register::R0 => self.r0,
            Register::R1 => self.r1,
            Register::R2 => self.r2,
            Register::R3 => self.r3,
            Register::R4 => self.r4,
            Register::R5 => self.r5,
            Register::R6 => self.r6,
            Register::R7 => self.r7,
            Register::R8 => self.r8,
            Register::R9 => self.r9,
            Register::R10 => self.r10,
            Register::R11 => self.r11,
            Register::R12 => self.r12,
            Register::R13 => self.r13,
            Register::R14 => self.r14,
            Register::R15 => pc_calculation(self.r15),
            Register::Spsr => unreachable!("no spsr in user mode"),
            Register::Cpsr => self.cpsr,
        }
    }

    fn pc(&self) -> u32 {
        self.read_register(Register::R15, |pc| pc)
    }
}

pub enum Instruction {
    ArmInstruction(ArmInstruction),
    ThumbInstruction(ThumbInstruction),
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::ArmInstruction(instruction) => Debug::fmt(&instruction, f),
            Instruction::ThumbInstruction(instruction) => Debug::fmt(&instruction, f),
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::ArmInstruction(instruction) => Display::fmt(&instruction, f),
            Instruction::ThumbInstruction(instruction) => Display::fmt(&instruction, f),
        }
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Self::ArmInstruction(arm::decode_arm(0x00000000))
    }
}

impl Cpu {
    pub fn fetch_decode_execute(&mut self) {
        let irq_wanted = !self.get_irq_disable() && self.bus.get_irq_pending();
        let pc = self.read_register(Register::R15, |pc| pc);

        let cycles_taken = match self.get_instruction_mode() {
            InstructionSet::Arm => {
                if pc % 4 != 0 {
                    unreachable!("unaligned ARM pc");
                }

                let decoded_instruction = self.pre_decode_arm;
                let prefetched_opcode = self.prefetch_opcode;

                self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(pc));
                self.pre_decode_arm = prefetched_opcode.map(arm::decode_arm);

                if let Some(decoded) = decoded_instruction {
                    // IRQ must only be dispatched when the pipeline is full.
                    //
                    // The return value we push in the IRQ handler is based on the current value of
                    // PC, which is in turned based on how saturated the prefetch pipeline is. As
                    // a result, if we attempt to dispatch an interrupt directly after the pipeline
                    // is flushed (for instance, by a branch), our IRQ handler will push the wrong
                    // return value.
                    //
                    // TODO: Evaluate whether it makes more sense to add custom logic to our
                    // IRQ handler to check what stage of the instruction pipeline we're in in
                    // order to calculate the proper return value to save. This may make more sense
                    // in the long run, but this works for now. This same logic applies to ARM,
                    // for the same reasons.
                    if irq_wanted {
                        self.handle_exception(ExceptionType::InterruptRequest);
                        1
                    } else {
                        self.execute_arm(decoded);
                        let cycle_info = decoded.instruction_type().cycles_info();

                        let result = cycle_info.i + cycle_info.n + cycle_info.s;
                        u8::max(result, 1)
                    }
                } else {
                    self.write_register(pc + 4, Register::R15);
                    1
                }
            }
            InstructionSet::Thumb => {
                if pc % 2 != 0 {
                    unreachable!("unaligned Thumb pc");
                }

                let decoded_instruction = self.pre_decode_thumb;
                let prefetched_opcode = self.prefetch_opcode;

                self.prefetch_opcode = Some(u32::from(self.bus.fetch_thumb_opcode(pc)));
                self.pre_decode_thumb =
                    prefetched_opcode.map(|prefetch| thumb::decode_thumb(prefetch as u16));

                if let Some(decoded) = decoded_instruction {
                    if irq_wanted {
                        self.handle_exception(ExceptionType::InterruptRequest);
                        1
                    } else {
                        self.execute_thumb(decoded);
                        let cycle_info = decoded.instruction_type().cycles_info();

                        let result = cycle_info.i + cycle_info.n + cycle_info.s;
                        u8::max(result, 1)
                    }
                } else {
                    self.write_register(pc + 2, Register::R15);
                    1
                }
            }
        };

        for _ in 0..cycles_taken {
            self.bus.step();
        }

        self.cycle_count += u64::from(cycles_taken);
    }

    fn flush_prefetch(&mut self) {
        self.pre_decode_arm = None;
        self.pre_decode_thumb = None;

        self.prefetch_opcode = None;
    }

    fn handle_exception(&mut self, exception_type: ExceptionType) {
        log::trace!("HANDLING EXCEPTION: {:?}", exception_type);

        let new_mode = match exception_type {
            ExceptionType::Reset => CpuMode::Supervisor,
            ExceptionType::Undefined => CpuMode::Undefined,
            ExceptionType::Swi => CpuMode::Supervisor,
            ExceptionType::PrefetchAbort => CpuMode::Abort,
            ExceptionType::DataAbort => CpuMode::Abort,
            ExceptionType::AddressExceeds26Bit => CpuMode::Supervisor,
            ExceptionType::InterruptRequest => CpuMode::Irq,
            ExceptionType::FastInterruptRequest => CpuMode::Fiq,
        };

        let pc_offset = match (exception_type, self.get_instruction_mode()) {
            // PC = $ + 8 for ARM
            // PC = $ + 4 for Thumb
            //
            // IRQ Exception
            //
            // Determine return information. SPSR is to be the current CPSR, and LR is to be the
            // current PC minus 0 for Thumb or 4 for ARM, to change the PC offsets of 4 or 8
            // respectively from the address of the current instruction into the required address
            // of the instruction boundary at which the interrupt occurred plus 4. For this
            // purpose, the PC and CPSR are considered to have already moved on to their values
            // for the instruction following that boundary.
            (ExceptionType::InterruptRequest, InstructionSet::Arm) => |pc| pc - 4,
            (ExceptionType::InterruptRequest, InstructionSet::Thumb) => |pc| pc,
            // SVC (SWI) Exception
            //
            // Determine return information. SPSR is to be the current CPSR, after changing the IT[]
            // bits to give them the correct values for the following instruction, and LR is to be
            // the current PC minus 2 for Thumb or 4 for ARM, to change the PC offsets of 4 or 8
            // respectively from the address of the current instruction into the required address of
            // the next instruction, the SVC instruction having size 2bytes for Thumb or 4 bytes for ARM.
            (ExceptionType::Swi, InstructionSet::Arm) => |pc| pc - 4,
            (ExceptionType::Swi, InstructionSet::Thumb) => |pc| pc - 2,
            (exception_type, mode) => todo!("{exception_type:?}, {mode:?}"),
        };

        let old_pc = self.read_register(Register::R15, pc_offset);
        let old_flags = self.read_register(Register::Cpsr, |_| unreachable!());

        self.set_cpu_state_bit(false);

        self.set_cpu_mode(new_mode);
        // save old pc in new mode lr, old cpsr in new mode spsr
        self.current_registers.r14 = old_pc;
        self.current_registers.spsr = old_flags;

        self.set_irq_disable(true);

        // fiq only disabled by reset and fiq
        if matches!(
            exception_type,
            ExceptionType::Reset | ExceptionType::FastInterruptRequest
        ) {
            self.set_fiq_disable(true);
        }

        let new_pc = Self::get_exception_vector_address(exception_type);
        self.write_register(new_pc, Register::R15);
        self.flush_prefetch();
    }

    fn get_exception_vector_address(exception_type: ExceptionType) -> u32 {
        const RESET_EXCEPTION_VECTOR: u32 = 0x00000000;
        const UNDEFINED_INSTRUCTION_VECTOR: u32 = 0x00000004;
        const SOFTWARE_INTERRUPT_VECTOR: u32 = 0x00000008;
        const PREFETCH_ABORT_VECTOR: u32 = 0x0000000C;
        const DATA_ABORT_VECTOR: u32 = 0x00000010;
        const ADDRESS_EXCEEDS_26_BIT_VECTOR: u32 = 0x00000014;
        const INTERRUPT_REQUEST_VECTOR: u32 = 0x00000018;
        const FAST_INTERRUPT_REQUEST_VECTOR: u32 = 0x0000001C;

        match exception_type {
            ExceptionType::Reset => RESET_EXCEPTION_VECTOR,
            ExceptionType::Undefined => UNDEFINED_INSTRUCTION_VECTOR,
            ExceptionType::Swi => SOFTWARE_INTERRUPT_VECTOR,
            ExceptionType::PrefetchAbort => PREFETCH_ABORT_VECTOR,
            ExceptionType::DataAbort => DATA_ABORT_VECTOR,
            ExceptionType::AddressExceeds26Bit => ADDRESS_EXCEEDS_26_BIT_VECTOR,
            ExceptionType::InterruptRequest => INTERRUPT_REQUEST_VECTOR,
            ExceptionType::FastInterruptRequest => FAST_INTERRUPT_REQUEST_VECTOR,
        }
    }
}

impl Cpu {
    fn evaluate_instruction_condition(&self, condition: InstructionCondition) -> bool {
        if matches!(condition, InstructionCondition::Always) {
            true
        } else {
            match condition {
                InstructionCondition::Equal => self.get_zero_flag(),
                InstructionCondition::NotEqual => !self.get_zero_flag(),
                InstructionCondition::UnsignedHigherOrSame => self.get_carry_flag(),
                InstructionCondition::UnsignedLower => !self.get_carry_flag(),
                InstructionCondition::SignedNegative => self.get_sign_flag(),
                InstructionCondition::SignedPositiveOrZero => !self.get_sign_flag(),
                InstructionCondition::SignedOverflow => self.get_overflow_flag(),
                InstructionCondition::SignedNoOverflow => !self.get_overflow_flag(),
                InstructionCondition::UnsignedHigher => {
                    self.get_carry_flag() && (!self.get_zero_flag())
                }
                InstructionCondition::UnsignedLowerOrSame => {
                    (!self.get_carry_flag()) || self.get_zero_flag()
                }
                InstructionCondition::SignedGreaterOrEqual => {
                    self.get_sign_flag() == self.get_overflow_flag()
                }
                InstructionCondition::SignedLessThan => {
                    self.get_sign_flag() != self.get_overflow_flag()
                }
                InstructionCondition::SignedGreaterThan => {
                    (!self.get_zero_flag()) && (self.get_sign_flag() == self.get_overflow_flag())
                }
                InstructionCondition::SignedLessOrEqual => {
                    self.get_zero_flag() || (self.get_sign_flag() != self.get_overflow_flag())
                }
                InstructionCondition::Never => false,
                InstructionCondition::Always => unreachable!(),
            }
        }
    }
}

impl Cpu {
    const SIGN_FLAG_BIT_INDEX: usize = 31;
    const ZERO_FLAG_BIT_INDEX: usize = 30;
    const CARRY_FLAG_BIT_INDEX: usize = 29;
    const OVERFLOW_FLAG_BIT_INDEX: usize = 28;

    pub fn get_sign_flag(&self) -> bool {
        self.cpsr.get_bit(Self::SIGN_FLAG_BIT_INDEX)
    }

    fn set_sign_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.set_bit(Self::SIGN_FLAG_BIT_INDEX, set);
        self.cpsr = new_cpsr;
    }

    pub fn get_zero_flag(&self) -> bool {
        self.cpsr.get_bit(Self::ZERO_FLAG_BIT_INDEX)
    }

    fn set_zero_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.set_bit(Self::ZERO_FLAG_BIT_INDEX, set);
        self.cpsr = new_cpsr;
    }

    pub fn get_carry_flag(&self) -> bool {
        self.cpsr.get_bit(Self::CARRY_FLAG_BIT_INDEX)
    }

    fn set_carry_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.set_bit(Self::CARRY_FLAG_BIT_INDEX, set);
        self.cpsr = new_cpsr;
    }

    pub fn get_overflow_flag(&self) -> bool {
        self.cpsr.get_bit(Self::OVERFLOW_FLAG_BIT_INDEX)
    }

    fn set_overflow_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.set_bit(Self::OVERFLOW_FLAG_BIT_INDEX, set);
        self.cpsr = new_cpsr;
    }

    const IRQ_DISABLE_BIT_OFFSET: usize = 7;
    const FIQ_DISABLE_BIT_OFFSET: usize = 6;
    const STATE_BIT_OFFSET: usize = 5;
    const MODE_BITS_RANGE: RangeInclusive<usize> = 0..=4;
    const USER_MODE_BITS: u32 = 0b10000;
    const FIQ_MODE_BITS: u32 = 0b10001;
    const IRQ_MODE_BITS: u32 = 0b10010;
    const SUPERVISOR_MODE_BITS: u32 = 0b10011;
    const ABORT_MODE_BITS: u32 = 0b10111;
    const UNDEFINED_MODE_BITS: u32 = 0b11011;
    const SYSTEM_MODE_BITS: u32 = 0b11111;

    pub fn get_irq_disable(&self) -> bool {
        self.cpsr.get_bit(Self::IRQ_DISABLE_BIT_OFFSET)
    }

    fn set_irq_disable(&mut self, set: bool) {
        let new_cpsr = self.cpsr.set_bit(Self::IRQ_DISABLE_BIT_OFFSET, set);
        self.cpsr = new_cpsr;
    }

    pub fn get_fiq_disable(&self) -> bool {
        self.cpsr.get_bit(Self::FIQ_DISABLE_BIT_OFFSET)
    }

    fn set_fiq_disable(&mut self, set: bool) {
        let new_cpsr = self.cpsr.set_bit(Self::FIQ_DISABLE_BIT_OFFSET, set);
        self.cpsr = new_cpsr;
    }

    pub fn get_instruction_mode(&self) -> InstructionSet {
        if self.get_cpu_state_bit() {
            InstructionSet::Thumb
        } else {
            InstructionSet::Arm
        }
    }

    fn get_cpu_state_bit(&self) -> bool {
        self.cpsr.get_bit(Self::STATE_BIT_OFFSET)
    }

    fn set_cpu_state_bit(&mut self, set: bool) {
        let new_cpsr = self.cpsr.set_bit(Self::STATE_BIT_OFFSET, set);
        self.cpsr = new_cpsr;
    }

    pub fn get_cpu_mode(&self) -> CpuMode {
        match self.cpsr.get_bit_range(Self::MODE_BITS_RANGE) {
            Self::USER_MODE_BITS => CpuMode::User,
            Self::FIQ_MODE_BITS => CpuMode::Fiq,
            Self::IRQ_MODE_BITS => CpuMode::Irq,
            Self::SUPERVISOR_MODE_BITS => CpuMode::Supervisor,
            Self::ABORT_MODE_BITS => CpuMode::Abort,
            Self::UNDEFINED_MODE_BITS => CpuMode::Undefined,
            Self::SYSTEM_MODE_BITS => CpuMode::System,
            other => unreachable!("0b{:05b}", other),
        }
    }

    fn set_cpu_mode(&mut self, mode: CpuMode) {
        let new_mode_bits = match mode {
            CpuMode::User => Self::USER_MODE_BITS,
            CpuMode::Fiq => Self::FIQ_MODE_BITS,
            CpuMode::Irq => Self::IRQ_MODE_BITS,
            CpuMode::Supervisor => Self::SUPERVISOR_MODE_BITS,
            CpuMode::Abort => Self::ABORT_MODE_BITS,
            CpuMode::Undefined => Self::UNDEFINED_MODE_BITS,
            CpuMode::System => Self::SYSTEM_MODE_BITS,
        };

        let new_cpsr = self
            .read_register(Register::Cpsr, |_| unreachable!())
            .set_bit_range(new_mode_bits, Self::MODE_BITS_RANGE);

        self.write_register(new_cpsr, Register::Cpsr);
    }
}

// Methods intended for external introspection
impl Cpu {
    pub fn disassemble(&mut self, address: u32) -> Instruction {
        match self.get_instruction_mode() {
            InstructionSet::Arm => {
                let opcode = self.bus.read_word_address(address);
                let instruction = arm::decode_arm(opcode);
                Instruction::ArmInstruction(instruction)
            }
            InstructionSet::Thumb => {
                let opcode = self.bus.read_halfword_address(address) as u16;
                let instruction = thumb::decode_thumb(opcode);
                Instruction::ThumbInstruction(instruction)
            }
        }
    }

    pub fn get_instruction_width(&self) -> u32 {
        match self.get_instruction_mode() {
            InstructionSet::Arm => 4,
            InstructionSet::Thumb => 2,
        }
    }

    pub fn get_executing_pc(&self) -> u32 {
        let r15 = self.read_register(Register::R15, std::convert::identity);
        let prefetch_saturated = self.prefetch_opcode.is_some();
        let decode_saturated = match self.get_instruction_mode() {
            InstructionSet::Arm => self.pre_decode_arm.is_some(),
            InstructionSet::Thumb => self.pre_decode_thumb.is_some(),
        };

        let instructions_behind = match (prefetch_saturated, decode_saturated) {
            (false, false) => 0,
            (true, false) => 1,
            (false, true) => {
                unreachable!("prefetch empty and decode saturated shouldn't be possible")
            }
            (true, true) => 2,
        };

        let bytes_behind = instructions_behind * self.get_instruction_width();

        r15 - bytes_behind
    }
}
