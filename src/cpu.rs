mod arm;
mod thumb;

use std::collections::BTreeMap;
use std::fmt::Display;
use std::{fmt::Debug, ops::RangeInclusive};

use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::{cartridge, BitManipulation, DataAccess};

use crate::DEBUG_AND_PANIC_ON_LOOP;

pub use self::arm::ArmInstruction;
pub use self::thumb::ThumbInstruction;

type InstructionCache<T, U> = BTreeMap<T, U>;

pub struct Cpu {
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
    cpsr: u32,
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
    cycle_count: u64,
    pub bus: Bus,
    arm_cache: InstructionCache<u32, ArmInstruction>,
    thumb_cache: InstructionCache<u16, ThumbInstruction>,
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
        Self {
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
            cpsr: Self::SYSTEM_MODE_BITS,
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
            cycle_count: 0,
            bus: Bus::new(cartridge),
            arm_cache: Default::default(),
            thumb_cache: Default::default(),
        }
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

#[derive(Clone, Copy, Debug)]
pub enum CpuMode {
    User,
    Fiq,
    Irq,
    Supervisor,
    Abort,
    Undefined,
    System,
}

#[derive(Clone, Copy, Debug)]
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
            Self::Never => unreachable!("never branch condition"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum InstructionSet {
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
        match (self.get_cpu_mode(), register) {
            (_, Register::R0) => self.r0 = value,
            (_, Register::R1) => self.r1 = value,
            (_, Register::R2) => self.r2 = value,
            (_, Register::R3) => self.r3 = value,
            (_, Register::R4) => self.r4 = value,
            (_, Register::R5) => self.r5 = value,
            (_, Register::R6) => self.r6 = value,
            (_, Register::R7) => self.r7 = value,
            (CpuMode::Fiq, Register::R8) => self.r8_fiq = value,
            (_, Register::R8) => self.r8 = value,
            (CpuMode::Fiq, Register::R9) => self.r9_fiq = value,
            (_, Register::R9) => self.r9 = value,
            (CpuMode::Fiq, Register::R10) => self.r10_fiq = value,
            (_, Register::R10) => self.r10 = value,
            (CpuMode::Fiq, Register::R11) => self.r11_fiq = value,
            (_, Register::R11) => self.r11 = value,
            (CpuMode::Fiq, Register::R12) => self.r12_fiq = value,
            (_, Register::R12) => self.r12 = value,
            (CpuMode::User | CpuMode::System, Register::R13) => self.r13 = value,
            (CpuMode::Fiq, Register::R13) => self.r13_fiq = value,
            (CpuMode::Supervisor, Register::R13) => self.r13_svc = value,
            (CpuMode::Abort, Register::R13) => self.r13_abt = value,
            (CpuMode::Irq, Register::R13) => self.r13_irq = value,
            (CpuMode::Undefined, Register::R13) => self.r13_und = value,
            (CpuMode::User | CpuMode::System, Register::R14) => self.r14 = value,
            (CpuMode::Fiq, Register::R14) => self.r14_fiq = value,
            (CpuMode::Supervisor, Register::R14) => self.r14_svc = value,
            (CpuMode::Abort, Register::R14) => self.r14_abt = value,
            (CpuMode::Irq, Register::R14) => self.r14_irq = value,
            (CpuMode::Undefined, Register::R14) => self.r14_und = value,
            (_, Register::R15) => {
                if self.get_cpu_state_bit() {
                    if value & 0b1 != 0 {
                        println!(
                            "writing to Thumb PC with unaligned value: 0x{:08X}, force aligning",
                            value
                        );
                    }
                    self.r15 = value & !0b1;
                } else {
                    if value & 0b11 != 0 {
                        println!(
                            "writing to ARM PC with unaligned value: 0x{:08X}, force aligning",
                            value
                        );
                    }
                    self.r15 = value & !0b11;
                }
            }
            (_, Register::Cpsr) => self.cpsr = value,
            // if current mode has no spsr, value is written to cpsr instead
            (CpuMode::User | CpuMode::System, Register::Spsr) => self.cpsr = value,
            (CpuMode::Fiq, Register::Spsr) => self.spsr_fiq = value,
            (CpuMode::Supervisor, Register::Spsr) => self.spsr_svc = value,
            (CpuMode::Abort, Register::Spsr) => self.spsr_abt = value,
            (CpuMode::Irq, Register::Spsr) => self.spsr_irq = value,
            (CpuMode::Undefined, Register::Spsr) => self.spsr_und = value,
        }
    }

    fn read_register(&self, register: Register, pc_calculation: fn(u32) -> u32) -> u32 {
        match (self.get_cpu_mode(), register) {
            (_, Register::R0) => self.r0,
            (_, Register::R1) => self.r1,
            (_, Register::R2) => self.r2,
            (_, Register::R3) => self.r3,
            (_, Register::R4) => self.r4,
            (_, Register::R5) => self.r5,
            (_, Register::R6) => self.r6,
            (_, Register::R7) => self.r7,
            (CpuMode::Fiq, Register::R8) => self.r8_fiq,
            (_, Register::R8) => self.r8,
            (CpuMode::Fiq, Register::R9) => self.r9_fiq,
            (_, Register::R9) => self.r9,
            (CpuMode::Fiq, Register::R10) => self.r10_fiq,
            (_, Register::R10) => self.r10,
            (CpuMode::Fiq, Register::R11) => self.r11_fiq,
            (_, Register::R11) => self.r11,
            (CpuMode::Fiq, Register::R12) => self.r12_fiq,
            (_, Register::R12) => self.r12,
            (CpuMode::User | CpuMode::System, Register::R13) => self.r13,
            (CpuMode::Fiq, Register::R13) => self.r13_fiq,
            (CpuMode::Supervisor, Register::R13) => self.r13_svc,
            (CpuMode::Abort, Register::R13) => self.r13_abt,
            (CpuMode::Irq, Register::R13) => self.r13_irq,
            (CpuMode::Undefined, Register::R13) => self.r13_und,
            (CpuMode::User | CpuMode::System, Register::R14) => self.r14,
            (CpuMode::Fiq, Register::R14) => self.r14_fiq,
            (CpuMode::Supervisor, Register::R14) => self.r14_svc,
            (CpuMode::Abort, Register::R14) => self.r14_abt,
            (CpuMode::Irq, Register::R14) => self.r14_irq,
            (CpuMode::Undefined, Register::R14) => self.r14_und,
            (_, Register::R15) => pc_calculation(self.r15),
            (_, Register::Cpsr) => self.cpsr,
            // if current mode has no spsr, value read is cpsr
            (CpuMode::User | CpuMode::System, Register::Spsr) => self.cpsr,
            (CpuMode::Fiq, Register::Spsr) => self.spsr_fiq,
            (CpuMode::Supervisor, Register::Spsr) => self.spsr_svc,
            (CpuMode::Abort, Register::Spsr) => self.spsr_abt,
            (CpuMode::Irq, Register::Spsr) => self.spsr_irq,
            (CpuMode::Undefined, Register::Spsr) => self.spsr_und,
        }
    }

    fn read_register_double(
        &self,
        first_register: Register,
        pc_calculation: fn(u32) -> u32,
    ) -> (u32, u32) {
        let second_register = match first_register {
            Register::R0 => Register::R1,
            Register::R1 => Register::R2,
            Register::R2 => Register::R3,
            Register::R3 => Register::R4,
            Register::R4 => Register::R5,
            Register::R5 => Register::R6,
            Register::R6 => Register::R7,
            Register::R7 => Register::R8,
            Register::R8 => Register::R9,
            Register::R9 => Register::R10,
            Register::R10 => Register::R11,
            Register::R11 => Register::R12,
            Register::R12 => Register::R13,
            Register::R13 => Register::R14,
            Register::R14 => Register::R15,
            _ => unreachable!("double register write to {}", first_register),
        };

        let value_1 = self.read_register(first_register, pc_calculation);
        let value_2 = self.read_register(second_register, pc_calculation);

        (value_1, value_2)
    }
}

impl Cpu {
    pub fn fetch_decode_execute(&mut self, debug: bool) {
        if debug {
            let pc_offset = match self.get_instruction_mode() {
                InstructionSet::Arm => |pc| pc + 4,
                InstructionSet::Thumb => |pc| pc + 2,
            };
            print!("{:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} cpsr: {:08X} | ",
                self.read_register(Register::R0, |_| unreachable!()),
                self.read_register(Register::R1, |_| unreachable!()),
                self.read_register(Register::R2, |_| unreachable!()),
                self.read_register(Register::R3, |_| unreachable!()),
                self.read_register(Register::R4, |_| unreachable!()),
                self.read_register(Register::R5, |_| unreachable!()),
                self.read_register(Register::R6, |_| unreachable!()),
                self.read_register(Register::R7, |_| unreachable!()),
                self.read_register(Register::R8, |_| unreachable!()),
                self.read_register(Register::R9, |_| unreachable!()),
                self.read_register(Register::R10, |_| unreachable!()),
                self.read_register(Register::R11, |_| unreachable!()),
                self.read_register(Register::R12, |_| unreachable!()),
                self.read_register(Register::R13, |_| unreachable!()),
                self.read_register(Register::R14, |_| unreachable!()),
                self.read_register(Register::R15, pc_offset),
                self.read_register(Register::Cpsr, |_| unreachable!())
            );
        }
        self.bus.step();

        if !self.get_irq_disable() && self.bus.get_irq_pending() {
            self.handle_exception(ExceptionType::InterruptRequest);
        } else {
            let pc = self.read_register(Register::R15, |pc| pc);
            match self.get_instruction_mode() {
                InstructionSet::Arm => {
                    if pc % 4 != 0 {
                        unreachable!("unaligned ARM pc");
                    }

                    let opcode = self.bus.read_word_address(pc);

                    // let instruction = match self.arm_cache.get(&opcode) {
                    //     Some(&cached) => cached,
                    //     None => {
                    //         let decoded = arm::decode_arm(opcode);
                    //         self.arm_cache.insert(opcode, decoded);
                    //         decoded
                    //     }
                    // };
                    let instruction = arm::decode_arm(opcode);

                    self.write_register(pc + 4, Register::R15);
                    self.execute_arm(instruction);
                }
                InstructionSet::Thumb => {
                    if pc % 2 != 0 {
                        unreachable!("unaligned Thumb pc");
                    }

                    let opcode = self.bus.read_halfword_address(pc);

                    // let instruction = match self.thumb_cache.get(&opcode) {
                    //     Some(&cached) => cached,
                    //     None => {
                    //         let decoded = thumb::decode_thumb(opcode);
                    //         self.thumb_cache.insert(opcode, decoded);
                    //         decoded
                    //     }
                    // };
                    let instruction = thumb::decode_thumb(opcode);

                    self.write_register(pc + 2, Register::R15);
                    self.execute_thumb(instruction);
                }
            }
        }

        self.cycle_count += 1;
    }

    fn handle_exception(&mut self, exception_type: ExceptionType) {
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

        let pc_offset = match exception_type {
            ExceptionType::InterruptRequest => |pc| pc + 4,
            ExceptionType::Swi => |pc| pc, // PC has already been incremented by decoder
            _ => todo!("{:?}", exception_type),
        };

        let old_pc = self.read_register(Register::R15, pc_offset);

        // save old pc in new mode lr
        match new_mode {
            CpuMode::Abort => self.r14_abt = old_pc,
            CpuMode::Fiq => self.r14_fiq = old_pc,
            CpuMode::Irq => self.r14_irq = old_pc,
            CpuMode::Supervisor => self.r14_svc = old_pc,
            CpuMode::Undefined => self.r14_und = old_pc,
            _ => unreachable!(),
        };

        let old_flags = self.read_register(Register::Cpsr, |_| unreachable!());

        // save old cpsr in new mode spsr
        match new_mode {
            CpuMode::Abort => self.spsr_abt = old_flags,
            CpuMode::Fiq => self.spsr_fiq = old_flags,
            CpuMode::Irq => self.spsr_irq = old_flags,
            CpuMode::Supervisor => self.spsr_svc = old_flags,
            CpuMode::Undefined => self.spsr_und = old_flags,
            _ => unreachable!(),
        };

        self.set_cpu_state_bit(false);
        self.set_cpu_mode(new_mode);
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

    fn get_sign_flag(&self) -> bool {
        self.cpsr.get_bit(Self::SIGN_FLAG_BIT_INDEX)
    }

    fn set_sign_flag(&mut self, set: bool) {
        self.cpsr = self.cpsr.set_bit(Self::SIGN_FLAG_BIT_INDEX, set);
    }

    fn get_zero_flag(&self) -> bool {
        self.cpsr.get_bit(Self::ZERO_FLAG_BIT_INDEX)
    }

    fn set_zero_flag(&mut self, set: bool) {
        self.cpsr = self.cpsr.set_bit(Self::ZERO_FLAG_BIT_INDEX, set);
    }

    fn get_carry_flag(&self) -> bool {
        self.cpsr.get_bit(Self::CARRY_FLAG_BIT_INDEX)
    }

    fn set_carry_flag(&mut self, set: bool) {
        self.cpsr = self.cpsr.set_bit(Self::CARRY_FLAG_BIT_INDEX, set);
    }

    fn get_overflow_flag(&self) -> bool {
        self.cpsr.get_bit(Self::OVERFLOW_FLAG_BIT_INDEX)
    }

    fn set_overflow_flag(&mut self, set: bool) {
        self.cpsr = self.cpsr.set_bit(Self::OVERFLOW_FLAG_BIT_INDEX, set);
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

    fn get_irq_disable(&self) -> bool {
        self.cpsr.get_bit(Self::IRQ_DISABLE_BIT_OFFSET)
    }

    fn set_irq_disable(&mut self, set: bool) {
        self.cpsr = self.cpsr.set_bit(Self::IRQ_DISABLE_BIT_OFFSET, set);
    }

    fn get_fiq_disable(&self) -> bool {
        self.cpsr.get_bit(Self::FIQ_DISABLE_BIT_OFFSET)
    }

    fn set_fiq_disable(&mut self, set: bool) {
        self.cpsr = self.cpsr.set_bit(Self::FIQ_DISABLE_BIT_OFFSET, set);
    }

    fn get_instruction_mode(&self) -> InstructionSet {
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
        self.cpsr = self.cpsr.set_bit(Self::STATE_BIT_OFFSET, set);
    }

    fn get_cpu_mode(&self) -> CpuMode {
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

        self.cpsr = self
            .cpsr
            .set_bit_range(new_mode_bits, Self::MODE_BITS_RANGE);
    }
}
