mod display;

use std::collections::btree_map::Range;
use std::fmt::Display;
use std::{fmt::Debug, ops::RangeInclusive};

use crate::bit_manipulation::BitManipulation;
use crate::bus::{Bus, DataAccess};
use crate::DEBUG_AND_PANIC_ON_LOOP;

#[derive(Debug)]
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

impl Default for Cpu {
    fn default() -> Self {
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
            bus: Bus::default(),
        }
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

#[derive(Clone, Copy, Debug)]
enum OffsetModifierType {
    AddToBase,
    SubtractFromBase,
}

#[derive(Clone, Copy, Debug)]
pub enum SingleDataMemoryAccessSize {
    Byte,
    HalfWord,
    Word,
    DoubleWord,
}

#[derive(Clone, Copy, Debug)]
enum ArmInstructionType {
    B {
        offset: i32,
    },
    Bl {
        offset: i32,
    },
    Bx {
        operand: Register,
    },
    Blx {
        operand: Register,
    },
    Ldr {
        index_type: SingleDataTransferIndexType,
        base_register: Register,
        destination_register: Register,
        offset_info: SingleDataTransferOffsetInfo,
        access_size: SingleDataMemoryAccessSize,
        sign_extend: bool,
    },
    Str {
        index_type: SingleDataTransferIndexType,
        base_register: Register,
        source_register: Register,
        offset_info: SingleDataTransferOffsetInfo,
        access_size: SingleDataMemoryAccessSize,
    },
    Ldm {
        index_type: BlockDataTransferIndexType,
        offset_modifier: OffsetModifierType,
        write_back: bool,
        base_register: Register,
        register_bit_list: [bool; 16],
    },
    Stm {
        index_type: BlockDataTransferIndexType,
        offset_modifier: OffsetModifierType,
        write_back: bool,
        base_register: Register,
        register_bit_list: [bool; 16],
    },
    Mrs {
        source_psr: PsrTransferPsr,
        destination_register: Register,
    },
    Msr {
        destination_psr: PsrTransferPsr,
        write_flags_field: bool,
        write_status_field: bool,
        write_extension_field: bool,
        write_control_field: bool,
        source_info: MsrSourceInfo,
    },
    Alu {
        operation: AluOperation,
        set_conditions: bool,
        first_operand: Register,
        second_operand: AluSecondOperandInfo,
        destination_operand: Register,
    },
    Mul {
        operation: MultiplyOperation,
        set_conditions: bool,
        destination_register: Register,
        accumulate_register: Register,
        operand_register_rs: Register,
        operand_register_rm: Register,
    },
    Swi {
        comment: u32,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbInstructionType {
    Ldr {
        base_register: Register,
        offset: ThumbRegisterOrImmediate,
        destination_register: Register,
        size: ThumbLoadStoreDataSize,
        sign_extend: bool,
    },
    Str {
        base_register: Register,
        offset: ThumbRegisterOrImmediate,
        source_register: Register,
        size: ThumbLoadStoreDataSize,
    },
    Register {
        operation: ThumbRegisterOperation,
        destination_register: Register,
        source: Register,
        second_operand: ThumbRegisterOrImmediate,
    },
    HighRegister {
        operation: ThumbHighRegisterOperation,
        destination_register: Register,
        source: Register,
    },
    B {
        condition: InstructionCondition,
        offset: i16,
    },
    BlPartOne {
        offset: i32,
    },
    BlPartTwo {
        offset: u16,
    },
    Bx {
        operand: Register,
    },
    Push {
        register_bit_list: [bool; 8],
        push_lr: bool,
    },
    Pop {
        register_bit_list: [bool; 8],
        pop_pc: bool,
    },
    AddSpecial {
        source_register: Register,
        dest_register: Register,
        unsigned_offset: u16,
        sign_bit: bool,
    },
    StmiaWriteBack {
        base_register: Register,
        register_bit_list: [bool; 8],
    },
    LdmiaWriteBack {
        base_register: Register,
        register_bit_list: [bool; 8],
    },
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbRegisterOperation {
    Lsl,
    Lsr,
    Asr,
    Add,
    Sub,
    Mov,
    Cmp,
    And,
    Eor,
    Adc,
    Sbc,
    Ror,
    Tst,
    Neg,
    Cmn,
    Orr,
    Mul,
    Bic,
    Mvn,
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbHighRegisterOperation {
    Add,
    Cmp,
    Mov,
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbRegisterOrImmediate {
    Immediate(u32),
    Register(Register),
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbLoadStoreDataSize {
    Byte,
    HalfWord,
    Word,
}

#[derive(Clone, Copy, Debug)]
pub struct ArmInstruction {
    instruction_type: ArmInstructionType,
    condition: InstructionCondition,
    address: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ThumbInstruction {
    instruction_type: ThumbInstructionType,
    address: u32,
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

#[derive(Clone, Copy, Debug)]
pub enum SingleDataTransferIndexType {
    PostIndex { non_privileged: bool },
    PreIndex { write_back: bool },
}

#[derive(Clone, Copy, Debug)]
pub enum BlockDataTransferIndexType {
    PostIndex,
    PreIndex,
}

#[derive(Clone, Copy, Debug)]
enum SingleDataTransferType {
    Ldr,
    Str,
}

#[derive(Clone, Copy, Debug)]
pub enum BlockDataTransferType {
    Ldm,
    Stm,
}

#[derive(Clone, Copy, Debug)]
pub enum PsrTransferType {
    Mrs,
    Msr,
}

#[derive(Clone, Copy, Debug)]
pub enum PsrTransferPsr {
    Cpsr,
    Spsr,
}

#[derive(Clone, Copy, Debug)]
pub struct SingleDataTransferOffsetInfo {
    value: SingleDataTransferOffsetValue,
    sign: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum SingleDataTransferOffsetValue {
    Immediate {
        offset: u32,
    },
    RegisterImmediate {
        shift_amount: u32,
        shift_type: ShiftType,
        offset_register: Register,
    },
    Register {
        offset_register: Register,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum AluSecondOperandInfo {
    Register {
        shift_info: ArmRegisterOrImmediate,
        shift_type: ShiftType,
        register: Register,
    },
    Immediate {
        base: u32,
        shift: u32,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum ArmRegisterOrImmediate {
    Immediate(u32),
    Register(Register),
}

#[derive(Clone, Copy, Debug)]
pub enum AluOperation {
    And,
    Eor,
    Sub,
    Rsb,
    Add,
    Adc,
    Sbc,
    Rsc,
    Tst,
    Teq,
    Cmp,
    Cmn,
    Orr,
    Mov,
    Bic,
    Mvn,
}

#[derive(Clone, Copy, Debug)]
pub enum MultiplyOperation {
    Mul,
    Mla,
    Umaal,
    Umull,
    Umlal,
    Smull,
    Smlal,
}

#[derive(Clone, Copy, Debug)]
pub enum MsrSourceInfo {
    Register(Register),
    Immediate { value: u32 },
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
                    self.r15 = value & !1;
                } else {
                    self.r15 = value & !3;
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

    fn write_register_double(&mut self, values: (u32, u32), first_register: Register) {
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

        self.write_register(values.0, first_register);
        self.write_register(values.1, second_register);
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
        let pc_offset = if self.get_cpu_state_bit() {
            |pc| pc + 2
        } else {
            |pc| pc + 4
        };

        if debug {
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
                    if debug {
                        print!("{:08X}: ", opcode);
                    }

                    let instruction = Self::decode_arm(opcode, pc);
                    if debug {
                        println!("{}", instruction);
                    }

                    self.write_register(pc + 4, Register::R15);
                    self.execute_arm(instruction);
                }
                InstructionSet::Thumb => {
                    if pc % 2 != 0 {
                        unreachable!("unaligned Thumb pc");
                    }

                    let opcode = self.bus.read_halfword_address(pc);
                    if debug {
                        print!("    {:04X}: ", opcode);
                    }

                    let instruction = Self::decode_thumb(opcode, pc);
                    if debug {
                        println!("{}", instruction);
                    }
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

        let old_pc = self.read_register(Register::R15, |pc| pc);

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
    pub fn decode_arm(opcode: u32, address: u32) -> ArmInstruction {
        let condition = opcode.get_condition();

        let maybe_instruction_type = None
            .or_else(|| Self::try_decode_arm_branch(opcode))
            .or_else(|| Self::try_decode_arm_data_process(opcode))
            .or_else(|| Self::try_decode_arm_multiply(opcode))
            .or_else(|| Self::try_decode_arm_psr_transfer(opcode))
            .or_else(|| Self::try_decode_arm_single_data_transfer(opcode))
            .or_else(|| Self::try_decode_arm_block_data_transfer(opcode));

        let instruction_type = if let Some(instruction_type) = maybe_instruction_type {
            instruction_type
        } else {
            todo!("unrecognized ARM opcode 0x{:08X}", opcode)
        };

        ArmInstruction {
            condition,
            instruction_type,
            address,
        }
    }

    pub fn decode_thumb(opcode: u16, address: u32) -> ThumbInstruction {
        let maybe_instruction_type = None
            .or_else(|| Self::try_decode_thumb_register_operation(opcode))
            .or_else(|| Self::try_decode_thumb_memory_load_store(opcode))
            .or_else(|| Self::try_decode_thumb_memory_addressing(opcode))
            .or_else(|| Self::try_decode_thumb_memory_multiple_load_store(opcode))
            .or_else(|| Self::try_decode_thumb_jump_call(opcode));

        let instruction_type = if let Some(instruction_type) = maybe_instruction_type {
            instruction_type
        } else {
            todo!("unrecognized Thumb opcode")
        };

        ThumbInstruction {
            instruction_type,
            address,
        }
    }
}

impl Cpu {
    fn try_decode_arm_branch(opcode: u32) -> Option<ArmInstructionType> {
        None.or_else(|| Self::try_decode_arm_branch_basic(opcode))
            .or_else(|| Self::try_decode_arm_branch_exchange(opcode))
            .or_else(|| Self::try_decode_arm_swi(opcode))
    }

    fn try_decode_arm_branch_basic(opcode: u32) -> Option<ArmInstructionType> {
        const BRANCH_MASK: u32 = 0b00001110_00000000_00000000_00000000;
        const BRANCH_MASK_RESULT: u32 = 0b00001010_00000000_00000000_00000000;

        opcode.match_mask(BRANCH_MASK, BRANCH_MASK_RESULT).then(|| {
            const OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=23;
            const BRANCH_TYPE_BIT_INDEX: usize = 24;

            // 24-bit sign extension, by left shifting until effective sign bit is in MSB, then ASR
            // an equal amount back over.
            let offset = (((opcode.get_bit_range(OFFSET_BIT_RANGE) as i32) << 8) >> 8) * 4;

            let branch_type_bit = opcode.get_bit(BRANCH_TYPE_BIT_INDEX);
            if branch_type_bit {
                ArmInstructionType::Bl { offset }
            } else {
                ArmInstructionType::B { offset }
            }
        })
    }

    fn try_decode_arm_branch_exchange(opcode: u32) -> Option<ArmInstructionType> {
        const BRANCH_EXCHANGE_MASK: u32 = 0b00001111_11111111_11111111_00000000;
        const BRANCH_EXCHANGE_MASK_RESULT: u32 = 0b00000001_00101111_11111111_00000000;

        opcode
            .match_mask(BRANCH_EXCHANGE_MASK, BRANCH_EXCHANGE_MASK_RESULT)
            .then(|| {
                const OPCODE_BIT_RANGE: RangeInclusive<usize> = 4..=7;

                const OPERAND_REGISTER_OFFSET: usize = 0;

                let operand = opcode.get_register_at_offset(OPERAND_REGISTER_OFFSET);

                match opcode.get_bit_range(OPCODE_BIT_RANGE) {
                    0b0001 => ArmInstructionType::Bx { operand },
                    0b0010 => todo!("Jazelle bytecode"),
                    0b0011 => ArmInstructionType::Blx { operand },
                    _ => unreachable!(),
                }
            })
    }

    fn try_decode_arm_swi(opcode: u32) -> Option<ArmInstructionType> {
        const MUST_BE_1111_BIT_RANGE: RangeInclusive<usize> = 24..=27;
        const COMMENT_FIELD_BIT_RANGE: RangeInclusive<usize> = 0..=23;

        if opcode.get_bit_range(MUST_BE_1111_BIT_RANGE) != 0b1111 {
            return None;
        }

        let comment = opcode.get_bit_range(COMMENT_FIELD_BIT_RANGE);

        Some(ArmInstructionType::Swi { comment })
    }

    fn try_decode_arm_data_process(opcode: u32) -> Option<ArmInstructionType> {
        const DATA_PROCESS_MASK: u32 = 0b00001100_00000000_00000000_00000000;
        const DATA_PROCESS_MASK_RESULT: u32 = 0b00000000_00000000_00000000_00000000;

        fn lookup_alu_opcode(opcode_value: u32) -> AluOperation {
            match opcode_value {
                0x0 => AluOperation::And,
                0x1 => AluOperation::Eor,
                0x2 => AluOperation::Sub,
                0x3 => AluOperation::Rsb,
                0x4 => AluOperation::Add,
                0x5 => AluOperation::Adc,
                0x6 => AluOperation::Sbc,
                0x7 => AluOperation::Rsc,
                0x8 => AluOperation::Tst,
                0x9 => AluOperation::Teq,
                0xA => AluOperation::Cmp,
                0xB => AluOperation::Cmn,
                0xC => AluOperation::Orr,
                0xD => AluOperation::Mov,
                0xE => AluOperation::Bic,
                0xF => AluOperation::Mvn,
                _ => unreachable!(),
            }
        }

        if opcode.match_mask(DATA_PROCESS_MASK, DATA_PROCESS_MASK_RESULT) {
            const IMMEDIATE_OPERAND_BIT_INDEX: usize = 25;
            const ALU_OPCODE_BIT_RANGE: RangeInclusive<usize> = 21..=24;
            const SET_CONDITION_CODES_BIT_INDEX: usize = 20;

            const FIRST_OPERATION_REGISTER_OFFSET: usize = 16;
            const DESTINATION_OPERATION_REGISTER_OFFSET: usize = 12;

            let opcode_value = opcode.get_bit_range(ALU_OPCODE_BIT_RANGE);
            let set_condition_codes_bit = opcode.get_bit(SET_CONDITION_CODES_BIT_INDEX);
            let first_operand = opcode.get_register_at_offset(FIRST_OPERATION_REGISTER_OFFSET);
            let destination_operand =
                opcode.get_register_at_offset(DESTINATION_OPERATION_REGISTER_OFFSET);

            // set condition code "Must be 1 for opcode 8-B".
            if (0x8..=0xB).contains(&opcode_value) && !set_condition_codes_bit {
                return None;
            }

            let alu_operation = lookup_alu_opcode(opcode_value);

            // first operation register "Must be 0000b for MOV/MVN.".
            if matches!(alu_operation, AluOperation::Mov | AluOperation::Mvn)
                && !matches!(first_operand, Register::R0)
            {
                return None;
            }

            // destination register "Must be 0000b (or 1111b) for CMP/CMN/TST/TEQ{P}."
            if matches!(
                alu_operation,
                AluOperation::Cmp | AluOperation::Cmn | AluOperation::Tst | AluOperation::Teq
            ) && !matches!(destination_operand, Register::R0 | Register::R15)
            {
                return None;
            }

            let immediate_operand_bit = opcode.get_bit(IMMEDIATE_OPERAND_BIT_INDEX);

            let second_operand = if immediate_operand_bit {
                // Immediate as 2nd operand
                const LITERAL_SHIFT_BIT_RANGE: RangeInclusive<usize> = 8..=11;
                const SECOND_OPERAND_IMMEDIATE_BIT_RANGE: RangeInclusive<usize> = 0..=7;

                let shift = opcode.get_bit_range(LITERAL_SHIFT_BIT_RANGE) * 2;
                let base_value = opcode.get_bit_range(SECOND_OPERAND_IMMEDIATE_BIT_RANGE);

                AluSecondOperandInfo::Immediate {
                    base: base_value,
                    shift,
                }
            } else {
                // Register as 2nd operand
                const SHIFT_BY_REGISTER_BIT_INDEX: usize = 4;
                const SECOND_OPERAND_REGISTER_OFFSET: usize = 0;

                let shift_type = opcode.get_shift_type();
                let shift_by_register_bit = opcode.get_bit(SHIFT_BY_REGISTER_BIT_INDEX);
                let second_operand_register =
                    opcode.get_register_at_offset(SECOND_OPERAND_REGISTER_OFFSET);

                let shift_info = if shift_by_register_bit {
                    // Shift by Register
                    const SHIFT_REGISTER_OFFSET: usize = 8;
                    const MUST_BE_0_BIT_RANGE: RangeInclusive<usize> = 7..=7;

                    let shift_register = opcode.get_register_at_offset(SHIFT_REGISTER_OFFSET);

                    // This bit "must be zero  (otherwise multiply or LDREX or undefined)".
                    if opcode.get_bit_range(MUST_BE_0_BIT_RANGE) != 0 {
                        return None;
                    }

                    ArmRegisterOrImmediate::Register(shift_register)
                } else {
                    // Shift by Immediate
                    const SHIFT_AMOUNT_BIT_RANGE: RangeInclusive<usize> = 7..=11;

                    let shift_amount = opcode.get_bit_range(SHIFT_AMOUNT_BIT_RANGE);

                    ArmRegisterOrImmediate::Immediate(shift_amount)
                };
                AluSecondOperandInfo::Register {
                    register: second_operand_register,
                    shift_info,
                    shift_type,
                }
            };

            Some(ArmInstructionType::Alu {
                operation: alu_operation,
                set_conditions: set_condition_codes_bit,
                first_operand,
                second_operand,
                destination_operand,
            })
        } else {
            None
        }
    }

    fn try_decode_arm_multiply(opcode: u32) -> Option<ArmInstructionType> {
        const MUST_BE_000_BIT_RANGE: RangeInclusive<usize> = 25..=27;
        const MUL_OPCODE_BIT_RANGE: RangeInclusive<usize> = 21..=24;
        const SET_CONDITION_CODES_BIT_INDEX: usize = 20;
        const DESTINATION_REGISTER_OFFSET: usize = 16;
        const ACCUMULATE_REGISTER_OFFSET: usize = 12;
        const OPERAND_REGISTER_RS_OFFSET: usize = 8;
        const MUST_BE_1001_BIT_RANGE: RangeInclusive<usize> = 4..=7;
        const OPERAND_REGISTER_RM_OFFSET: usize = 0;

        fn lookup_mul_opcode(opcode_value: u32) -> MultiplyOperation {
            match opcode_value {
                0b0000 => MultiplyOperation::Mul,
                0b0001 => MultiplyOperation::Mla,
                0b0010 => MultiplyOperation::Umaal,
                0b0100 => MultiplyOperation::Umull,
                0b0101 => MultiplyOperation::Umlal,
                0b0110 => MultiplyOperation::Smull,
                0b0111 => MultiplyOperation::Smlal,
                _ => unreachable!(),
            }
        }

        if opcode.get_bit_range(MUST_BE_000_BIT_RANGE) != 0b000 {
            return None;
        }

        if opcode.get_bit_range(MUST_BE_1001_BIT_RANGE) != 0b1001 {
            return None;
        }

        let mul_opcode_value = opcode.get_bit_range(MUL_OPCODE_BIT_RANGE);
        let set_condition_codes_bit = opcode.get_bit(SET_CONDITION_CODES_BIT_INDEX);
        let destination_register = opcode.get_register_at_offset(DESTINATION_REGISTER_OFFSET);
        let accumulate_register = opcode.get_register_at_offset(ACCUMULATE_REGISTER_OFFSET);
        let operand_rs = opcode.get_register_at_offset(OPERAND_REGISTER_RS_OFFSET);
        let operand_rm = opcode.get_register_at_offset(OPERAND_REGISTER_RM_OFFSET);

        let operation_type = lookup_mul_opcode(mul_opcode_value);

        Some(ArmInstructionType::Mul {
            operation: operation_type,
            set_conditions: set_condition_codes_bit,
            accumulate_register,
            destination_register,
            operand_register_rm: operand_rm,
            operand_register_rs: operand_rs,
        })
    }

    fn try_decode_arm_psr_transfer(opcode: u32) -> Option<ArmInstructionType> {
        None.or_else(|| Self::try_decode_arm_mrs(opcode))
            .or_else(|| Self::try_decode_arm_msr(opcode))
    }

    fn try_decode_arm_mrs(opcode: u32) -> Option<ArmInstructionType> {
        const MUST_BE_00_BIT_RANGE: RangeInclusive<usize> = 26..=27;
        const MUST_BE_0_BIT_INDEX_1: usize = 25;
        const MUST_BE_10_BIT_RANGE: RangeInclusive<usize> = 23..=24;
        const SOURCE_DEST_PSR_BIT_INDEX: usize = 22;
        const OPCODE_VALUE_BIT_INDEX: usize = 21;
        const MUST_BE_0_BIT_INDEX_2: usize = 20;
        const MUST_BE_1111_BIT_RANGE: RangeInclusive<usize> = 16..=19;
        const DEST_REGISTER_OFFSET: usize = 12;
        const MUST_BE_0000_0000_0000_BIT_RANGE: RangeInclusive<usize> = 0..=11;

        if opcode.get_bit_range(MUST_BE_00_BIT_RANGE) != 0b00 {
            return None;
        }

        if opcode.get_bit(MUST_BE_0_BIT_INDEX_1) {
            return None;
        }

        if opcode.get_bit_range(MUST_BE_10_BIT_RANGE) != 0b10 {
            return None;
        }

        if opcode.get_bit(MUST_BE_0_BIT_INDEX_2) {
            return None;
        }

        if opcode.get_bit_range(MUST_BE_1111_BIT_RANGE) != 0b1111 {
            return None;
        }

        if opcode.get_bit_range(MUST_BE_0000_0000_0000_BIT_RANGE) != 0b0000_0000_0000 {
            return None;
        }

        // Opcode
        //  0: MRS{cond} Rd,Psr          ;Rd = Psr
        //  1: MSR{cond} Psr{_field},Op  ;Psr[field] = Op
        if opcode.get_bit(OPCODE_VALUE_BIT_INDEX) {
            return None;
        }

        let source_dest_psr_bit = opcode.get_bit(SOURCE_DEST_PSR_BIT_INDEX);

        let source_dest_psr = if source_dest_psr_bit {
            // SPSR
            PsrTransferPsr::Spsr
        } else {
            // CPSR
            PsrTransferPsr::Cpsr
        };

        let destination_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);

        Some(ArmInstructionType::Mrs {
            source_psr: source_dest_psr,
            destination_register,
        })
    }

    fn try_decode_arm_msr(opcode: u32) -> Option<ArmInstructionType> {
        const MUST_BE_00_BIT_RANGE: RangeInclusive<usize> = 26..=27;
        const IMMEDIATE_OFFSET_BIT_INDEX: usize = 25;
        const MUST_BE_10_BIT_RANGE: RangeInclusive<usize> = 23..=24;
        const SOURCE_DEST_PSR_BIT_INDEX: usize = 22;
        const OPCODE_VALUE_BIT_INDEX: usize = 21;
        const MUST_BE_0_BIT_RANGE: RangeInclusive<usize> = 20..=20;

        if opcode.get_bit_range(MUST_BE_00_BIT_RANGE) != 0b00 {
            return None;
        }

        if opcode.get_bit_range(MUST_BE_10_BIT_RANGE) != 0b10 {
            return None;
        }

        if opcode.get_bit_range(MUST_BE_0_BIT_RANGE) != 0b0 {
            return None;
        }

        // Opcode
        //  0: MRS{cond} Rd,Psr          ;Rd = Psr
        //  1: MSR{cond} Psr{_field},Op  ;Psr[field] = Op
        if !opcode.get_bit(OPCODE_VALUE_BIT_INDEX) {
            return None;
        }

        let immediate_operand_flag_bit = opcode.get_bit(IMMEDIATE_OFFSET_BIT_INDEX);
        let source_dest_psr_bit = opcode.get_bit(SOURCE_DEST_PSR_BIT_INDEX);

        let source_dest_psr = if source_dest_psr_bit {
            // SPSR
            PsrTransferPsr::Spsr
        } else {
            // CPSR
            PsrTransferPsr::Cpsr
        };

        const WRITE_FLAGS_FIELD_BIT_INDEX: usize = 19;
        const WRITE_STATUS_FIELD_BIT_INDEX: usize = 18;
        const WRITE_EXTENSION_FIELD_BIT_INDEX: usize = 17;
        const WRITE_CONTROL_FIELD_BIT_INDEX: usize = 16;

        let write_flags_field = opcode.get_bit(WRITE_FLAGS_FIELD_BIT_INDEX);
        let write_status_field = opcode.get_bit(WRITE_STATUS_FIELD_BIT_INDEX);
        let write_extension_field = opcode.get_bit(WRITE_EXTENSION_FIELD_BIT_INDEX);
        let write_control_field = opcode.get_bit(WRITE_CONTROL_FIELD_BIT_INDEX);

        let source_info = if immediate_operand_flag_bit {
            // MSR Psr,Imm

            const APPLIED_SHIFT_BIT_RANGE: RangeInclusive<usize> = 8..=11;
            const IMMEDIATE_BIT_RANGE: RangeInclusive<usize> = 0..=7;

            let shift = opcode.get_bit_range(APPLIED_SHIFT_BIT_RANGE) * 2;
            let immediate = opcode.get_bit_range(IMMEDIATE_BIT_RANGE);

            let value = immediate.rotate_right(shift);
            MsrSourceInfo::Immediate { value }
        } else {
            // MSR Psr,Rm

            const SHOULD_BE_00000000_BIT_RANGE: RangeInclusive<usize> = 4..=11;
            const SOURCE_REGISTER_OFFSET: usize = 0;

            if opcode.get_bit_range(SHOULD_BE_00000000_BIT_RANGE) != 0b00000000 {
                return None;
            }

            let source_register = opcode.get_register_at_offset(SOURCE_REGISTER_OFFSET);

            MsrSourceInfo::Register(source_register)
        };

        Some(ArmInstructionType::Msr {
            destination_psr: source_dest_psr,
            write_flags_field,
            write_status_field,
            write_extension_field,
            write_control_field,
            source_info,
        })
    }

    fn try_decode_arm_single_data_transfer(opcode: u32) -> Option<ArmInstructionType> {
        None.or_else(|| Self::try_decode_arm_basic_single_data_transfer(opcode))
            .or_else(|| Self::try_decode_arm_special_single_data_transfer(opcode))
    }

    fn try_decode_arm_basic_single_data_transfer(opcode: u32) -> Option<ArmInstructionType> {
        const SINGLE_DATA_TRANSFER_MASK: u32 = 0b00001100_00000000_00000000_00000000;
        const SINGLE_DATA_TRANSFER_MASK_RESULT: u32 = 0b00000100_00000000_00000000_00000000;

        if opcode.match_mask(SINGLE_DATA_TRANSFER_MASK, SINGLE_DATA_TRANSFER_MASK_RESULT) {
            const IMMEDIATE_OFFSET_BIT_INDEX: usize = 25;
            const PRE_POST_BIT_INDEX: usize = 24;
            const UP_DOWN_BIT_INDEX: usize = 23;
            const BYTE_WORD_BIT_INDEX: usize = 22;
            const INDEXING_CONFIG_BIT_INDEX: usize = 21;
            const LOAD_STORE_BIT_INDEX: usize = 20;
            const BASE_REGISTER_OFFSET: usize = 16;
            const SOURCE_DEST_REGISTER_OFFSET: usize = 12;

            let immediate_offset_bit = opcode.get_bit(IMMEDIATE_OFFSET_BIT_INDEX);
            let pre_post_bit = opcode.get_bit(PRE_POST_BIT_INDEX);
            let up_down_bit = opcode.get_bit(UP_DOWN_BIT_INDEX);
            let byte_word_bit = opcode.get_bit(BYTE_WORD_BIT_INDEX);
            let indexing_config_bit = opcode.get_bit(INDEXING_CONFIG_BIT_INDEX);
            let load_store_bit = opcode.get_bit(LOAD_STORE_BIT_INDEX);

            let index_type = if pre_post_bit {
                // pre-index
                SingleDataTransferIndexType::PreIndex {
                    write_back: indexing_config_bit,
                }
            } else {
                // post-indexing
                SingleDataTransferIndexType::PostIndex {
                    non_privileged: indexing_config_bit,
                }
            };

            let access_type = if load_store_bit {
                // ldr
                SingleDataTransferType::Ldr
            } else {
                // str
                SingleDataTransferType::Str
            };

            let access_size = if byte_word_bit {
                SingleDataMemoryAccessSize::Byte
            } else {
                SingleDataMemoryAccessSize::Word
            };

            let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);
            let source_destination_register =
                opcode.get_register_at_offset(SOURCE_DEST_REGISTER_OFFSET);

            let offset_value = if immediate_offset_bit {
                // register shifted by immediate
                const SHIFT_AMOUNT_BIT_RANGE: RangeInclusive<usize> = 7..=11;
                const MUST_BE_ZERO_BIT_RANGE: RangeInclusive<usize> = 4..=4;
                const OFFSET_REGISTER_OFFSET: usize = 0;

                // This bit "Must be 0 (Reserved, see The Undefined Instruction)".
                if opcode.get_bit_range(MUST_BE_ZERO_BIT_RANGE) != 0b0 {
                    return None;
                }

                let shift_amount = opcode.get_bit_range(SHIFT_AMOUNT_BIT_RANGE);
                let shift_type = opcode.get_shift_type();
                let offset_register = opcode.get_register_at_offset(OFFSET_REGISTER_OFFSET);

                SingleDataTransferOffsetValue::RegisterImmediate {
                    shift_amount,
                    shift_type,
                    offset_register,
                }
            } else {
                const IMMEDIATE_OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=11;

                let offset = opcode.get_bit_range(IMMEDIATE_OFFSET_BIT_RANGE);
                SingleDataTransferOffsetValue::Immediate { offset }
            };

            let offset_info = SingleDataTransferOffsetInfo {
                value: offset_value,
                sign: !up_down_bit,
            };

            Some(match access_type {
                SingleDataTransferType::Ldr => ArmInstructionType::Ldr {
                    index_type,
                    base_register,
                    destination_register: source_destination_register,
                    offset_info,
                    access_size,
                    sign_extend: false,
                },
                SingleDataTransferType::Str => ArmInstructionType::Str {
                    index_type,
                    base_register,
                    source_register: source_destination_register,
                    offset_info,
                    access_size,
                },
            })
        } else {
            None
        }
    }

    fn try_decode_arm_special_single_data_transfer(opcode: u32) -> Option<ArmInstructionType> {
        const MUST_BE_000_BIT_RANGE: RangeInclusive<usize> = 25..=27;
        const PRE_POST_BIT_INDEX: usize = 24;
        const UP_DOWN_BIT_INDEX: usize = 23;
        const IMMEDIATE_OFFSET_FLAG_INDEX: usize = 22;
        const WRITE_BACK_MEMORY_MANAGEMENT_BIT_INDEX: usize = 21;
        const LOAD_STORE_BIT_INDEX: usize = 20;
        const BASE_REGISTER_OFFSET: usize = 16;
        const SOURCE_DEST_REGISTER_OFFSET: usize = 12;
        const OFFSET_UPPER_4_BITS: RangeInclusive<usize> = 8..=11;
        const MUST_BE_1_BIT_INDEX_1: usize = 7;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 5..=6;
        const MUST_BE_1_BIT_INDEX_2: usize = 4;
        const OFFSET_LOWER_4_BITS: RangeInclusive<usize> = 0..=3;

        if opcode.get_bit_range(MUST_BE_000_BIT_RANGE) != 0b000 {
            return None;
        }

        if !opcode.get_bit(MUST_BE_1_BIT_INDEX_1) {
            return None;
        }

        if !opcode.get_bit(MUST_BE_1_BIT_INDEX_2) {
            return None;
        }

        let pre_post_bit_flag = opcode.get_bit(PRE_POST_BIT_INDEX);
        let up_down_bit_flag = opcode.get_bit(UP_DOWN_BIT_INDEX);
        let immediate_offset_flag = opcode.get_bit(IMMEDIATE_OFFSET_FLAG_INDEX);
        let write_back_memory_management_flag =
            opcode.get_bit(WRITE_BACK_MEMORY_MANAGEMENT_BIT_INDEX);
        let load_store_flag = opcode.get_bit(LOAD_STORE_BIT_INDEX);
        let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);
        let source_dest_register = opcode.get_register_at_offset(SOURCE_DEST_REGISTER_OFFSET);
        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);
        let offset_upper_4_bits = opcode.get_bit_range(OFFSET_UPPER_4_BITS);
        let offset_lower_4_bits = opcode.get_bit_range(OFFSET_LOWER_4_BITS);

        let offset_value = if immediate_offset_flag {
            let immediate_offset = (offset_upper_4_bits << 4) | offset_lower_4_bits;
            SingleDataTransferOffsetValue::Immediate {
                offset: immediate_offset,
            }
        } else {
            // When above Bit 22 I=0 (Register as Offset):
            //   Not used. Must be 0000b
            // Referring to offset upper.
            if offset_upper_4_bits != 0b0000 {
                return None;
            }

            let offset_register = Register::from_index(offset_lower_4_bits);
            SingleDataTransferOffsetValue::Register { offset_register }
        };

        let offset_info = SingleDataTransferOffsetInfo {
            value: offset_value,
            sign: !up_down_bit_flag,
        };

        // Pre/Post (0=post; add offset after transfer, 1=pre; before trans.)
        let index_type = if pre_post_bit_flag {
            SingleDataTransferIndexType::PreIndex {
                write_back: write_back_memory_management_flag,
            }
        } else {
            SingleDataTransferIndexType::PostIndex {
                non_privileged: write_back_memory_management_flag,
            }
        };

        Some(if load_store_flag {
            // When Bit 20 L=1 (Load):
            // 0: Reserved.
            // 1: LDR{cond}H  Rd,<Address>  ;Load Unsigned halfword (zero-extended)
            // 2: LDR{cond}SB Rd,<Address>  ;Load Signed byte (sign extended)
            // 3: LDR{cond}SH Rd,<Address>  ;Load Signed halfword (sign extended)
            match opcode_value {
                0 => return None,
                1 => ArmInstructionType::Ldr {
                    access_size: SingleDataMemoryAccessSize::HalfWord,
                    base_register,
                    destination_register: source_dest_register,
                    index_type,
                    offset_info,
                    sign_extend: false,
                },
                2 => ArmInstructionType::Ldr {
                    access_size: SingleDataMemoryAccessSize::Byte,
                    base_register,
                    destination_register: source_dest_register,
                    index_type,
                    offset_info,
                    sign_extend: true,
                },
                3 => ArmInstructionType::Ldr {
                    access_size: SingleDataMemoryAccessSize::HalfWord,
                    base_register,
                    destination_register: source_dest_register,
                    index_type,
                    offset_info,
                    sign_extend: true,
                },
                _ => unreachable!(),
            }
        } else {
            // When Bit 20 L=0 (Store) (and Doubleword Load/Store):
            // 0: Reserved for SWP instruction
            // 1: STR{cond}H  Rd,<Address>  ;Store halfword   [a]=Rd
            // 2: LDR{cond}D  Rd,<Address>  ;Load Doubleword  R(d)=[a], R(d+1)=[a+4]
            // 3: STR{cond}D  Rd,<Address>  ;Store Doubleword [a]=R(d), [a+4]=R(d+1)
            match opcode_value {
                0 => return None,
                1 => ArmInstructionType::Str {
                    access_size: SingleDataMemoryAccessSize::HalfWord,
                    base_register,
                    index_type,
                    offset_info,
                    source_register: source_dest_register,
                },
                2 => ArmInstructionType::Ldr {
                    access_size: SingleDataMemoryAccessSize::DoubleWord,
                    base_register,
                    index_type,
                    offset_info,
                    destination_register: source_dest_register,
                    sign_extend: false,
                },
                3 => ArmInstructionType::Str {
                    access_size: SingleDataMemoryAccessSize::DoubleWord,
                    base_register,
                    index_type,
                    offset_info,
                    source_register: source_dest_register,
                },
                _ => unreachable!(),
            }
        })
    }

    fn try_decode_arm_block_data_transfer(opcode: u32) -> Option<ArmInstructionType> {
        const BLOCK_DATA_TRANSFER_MASK: u32 = 0b00001110_00000000_00000000_00000000;
        const BLOCK_DATA_TRANSFER_MASK_RESULT: u32 = 0b00001000_00000000_00000000_00000000;

        if opcode.match_mask(BLOCK_DATA_TRANSFER_MASK, BLOCK_DATA_TRANSFER_MASK_RESULT) {
            const PRE_POST_BIT_INDEX: usize = 24;
            const UP_DOWN_BIT_INDEX: usize = 23;
            const PSR_FORCE_USER_BIT_INDEX: usize = 22;
            const WRITE_BACK_BIT_INDEX: usize = 21;
            const LOAD_STORE_BIT_INDEX: usize = 20;
            const BASE_REGISTER_OFFSET: usize = 16;
            const REGISTER_LIST_BIT_RANGE: RangeInclusive<usize> = 0..=15;

            let pre_post_bit = opcode.get_bit(PRE_POST_BIT_INDEX);
            let up_down_bit = opcode.get_bit(UP_DOWN_BIT_INDEX);
            let psr_force_user_bit = opcode.get_bit(PSR_FORCE_USER_BIT_INDEX);
            let write_back_bit = opcode.get_bit(WRITE_BACK_BIT_INDEX);
            let load_store_bit = opcode.get_bit(LOAD_STORE_BIT_INDEX);

            let index_type = if pre_post_bit {
                // pre-index
                BlockDataTransferIndexType::PreIndex
            } else {
                // post-index
                BlockDataTransferIndexType::PostIndex
            };

            let offset_modifier = if up_down_bit {
                // add offset to base
                OffsetModifierType::AddToBase
            } else {
                // subtract offset from base
                OffsetModifierType::SubtractFromBase
            };

            let write_back = write_back_bit;

            let access_type = if load_store_bit {
                // LDM
                BlockDataTransferType::Ldm
            } else {
                // STM
                BlockDataTransferType::Stm
            };

            let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);

            let register_list_raw = opcode.get_bit_range(REGISTER_LIST_BIT_RANGE);

            let mut register_bit_list = [false; 16];
            for (register_idx, register_bit) in register_bit_list.iter_mut().enumerate() {
                let register_mask = 1 << register_idx;
                let register_used = (register_list_raw & register_mask) == register_mask;
                *register_bit = register_used;
            }

            assert!(!psr_force_user_bit);
            Some(match access_type {
                BlockDataTransferType::Stm => ArmInstructionType::Stm {
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                },
                BlockDataTransferType::Ldm => ArmInstructionType::Ldm {
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                },
            })
        } else {
            None
        }
    }
}

impl Cpu {
    fn try_decode_thumb_register_operation(opcode: u16) -> Option<ThumbInstructionType> {
        None.or_else(|| Self::try_decode_thumb_move_shifted_register(opcode))
            .or_else(|| Self::try_decode_thumb_add_subtract(opcode))
            .or_else(|| Self::try_decode_thumb_move_compare_add_subtract_immediate(opcode))
            .or_else(|| Self::try_decode_thumb_alu_operations(opcode))
            .or_else(|| Self::try_decode_thumb_high_register_operations_branch_exchange(opcode))
    }

    fn try_decode_thumb_move_shifted_register(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_000_BIT_RANGE: RangeInclusive<usize> = 13..=15;
        const SHIFT_OPCODE_BIT_RANGE: RangeInclusive<usize> = 11..=12;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 6..=10;
        const SOURCE_REGISTER_OFFSET: usize = 3;
        const DEST_REGISTER_OFFSET: usize = 0;

        if opcode.get_bit_range(MUST_BE_000_BIT_RANGE) != 0b000 {
            return None;
        }

        let operation_type = match opcode.get_bit_range(SHIFT_OPCODE_BIT_RANGE) {
            0b00 => ThumbRegisterOperation::Lsl,
            0b01 => ThumbRegisterOperation::Lsr,
            0b10 => ThumbRegisterOperation::Asr,
            0b11 => return None,
            _ => unreachable!(),
        };

        let offset = u32::from(opcode.get_bit_range(OFFSET_BIT_RANGE));

        let source_register = opcode.get_register_at_offset(SOURCE_REGISTER_OFFSET);

        let dest_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);

        Some(ThumbInstructionType::Register {
            operation: operation_type,
            destination_register: dest_register,
            source: source_register,
            second_operand: ThumbRegisterOrImmediate::Immediate(offset),
        })
    }

    fn try_decode_thumb_add_subtract(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_00011_BIT_RANGE: RangeInclusive<usize> = 11..=15;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 9..=10;
        const IMMEDIATE_OPERAND_BIT_RANGE: RangeInclusive<usize> = 6..=8;
        const REGISTER_OPERAND_OFFSET: usize = 6;
        const SOURCE_REGISTER_OFFSET: usize = 3;
        const DEST_REGISTER_OFFSET: usize = 0;

        const ADD_REGISTER_OPCODE_VALUE: u16 = 0;
        const SUB_REGISTER_OPCODE_VALUE: u16 = 1;
        const ADD_IMMEDIATE_OPCODE_VALUE: u16 = 2;
        const SUB_IMMEDIATE_OPCODE_VALUE: u16 = 3;

        if opcode.get_bit_range(MUST_BE_00011_BIT_RANGE) != 0b00011 {
            return None;
        }

        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);
        let source_register = opcode.get_register_at_offset(SOURCE_REGISTER_OFFSET);
        let dest_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);

        Some(match opcode_value {
            ADD_REGISTER_OPCODE_VALUE => {
                let register_operand = opcode.get_register_at_offset(REGISTER_OPERAND_OFFSET);
                let second_operand = ThumbRegisterOrImmediate::Register(register_operand);
                ThumbInstructionType::Register {
                    destination_register: dest_register,
                    operation: ThumbRegisterOperation::Add,
                    source: source_register,
                    second_operand: second_operand,
                }
            }
            SUB_REGISTER_OPCODE_VALUE => {
                let register_operand = opcode.get_register_at_offset(REGISTER_OPERAND_OFFSET);
                let second_operand = ThumbRegisterOrImmediate::Register(register_operand);
                ThumbInstructionType::Register {
                    destination_register: dest_register,
                    operation: ThumbRegisterOperation::Sub,
                    source: source_register,
                    second_operand: second_operand,
                }
            }
            ADD_IMMEDIATE_OPCODE_VALUE => {
                let immediate_operand =
                    u32::from(opcode.get_bit_range(IMMEDIATE_OPERAND_BIT_RANGE));
                let second_operand = ThumbRegisterOrImmediate::Immediate(immediate_operand);
                ThumbInstructionType::Register {
                    destination_register: dest_register,
                    operation: ThumbRegisterOperation::Add,
                    source: source_register,
                    second_operand: second_operand,
                }
            }
            SUB_IMMEDIATE_OPCODE_VALUE => {
                let immediate_operand =
                    u32::from(opcode.get_bit_range(IMMEDIATE_OPERAND_BIT_RANGE));
                let second_operand = ThumbRegisterOrImmediate::Immediate(immediate_operand);
                ThumbInstructionType::Register {
                    destination_register: dest_register,
                    operation: ThumbRegisterOperation::Sub,
                    source: source_register,
                    second_operand: second_operand,
                }
            }
            _ => unreachable!(),
        })
    }

    fn try_decode_thumb_move_compare_add_subtract_immediate(
        opcode: u16,
    ) -> Option<ThumbInstructionType> {
        const MUST_BE_001_BIT_RANGE: RangeInclusive<usize> = 13..=15;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 11..=12;
        const DEST_REGISTER_OFFSET: usize = 8;
        const IMMEDIATE_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        if opcode.get_bit_range(MUST_BE_001_BIT_RANGE) != 0b001 {
            return None;
        }

        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);
        let dest_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);
        let immediate = u32::from(opcode.get_bit_range(IMMEDIATE_BIT_RANGE));

        let operation = match opcode_value {
            0b00 => ThumbRegisterOperation::Mov,
            0b01 => ThumbRegisterOperation::Cmp,
            0b10 => ThumbRegisterOperation::Add,
            0b11 => ThumbRegisterOperation::Sub,
            _ => unreachable!(),
        };

        Some(ThumbInstructionType::Register {
            operation,
            destination_register: dest_register,
            source: dest_register,
            second_operand: ThumbRegisterOrImmediate::Immediate(immediate),
        })
    }

    fn try_decode_thumb_alu_operations(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_010000_BIT_RANGE: RangeInclusive<usize> = 10..=15;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 6..=9;
        const SOURCE_REGISTER_OFFSET: usize = 3;
        const DEST_REGISTER_OFFSET: usize = 0;

        if opcode.get_bit_range(MUST_BE_010000_BIT_RANGE) != 0b010000 {
            return None;
        }

        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);
        let source_register = opcode.get_register_at_offset(SOURCE_REGISTER_OFFSET);
        let dest_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);

        let operation_type = match opcode_value {
            0x0 => ThumbRegisterOperation::And,
            0x1 => ThumbRegisterOperation::Eor,
            0x2 => ThumbRegisterOperation::Lsl,
            0x3 => ThumbRegisterOperation::Lsr,
            0x4 => ThumbRegisterOperation::Asr,
            0x5 => ThumbRegisterOperation::Adc,
            0x6 => ThumbRegisterOperation::Sbc,
            0x7 => ThumbRegisterOperation::Ror,
            0x8 => ThumbRegisterOperation::Tst,
            0x9 => ThumbRegisterOperation::Neg,
            0xA => ThumbRegisterOperation::Cmp,
            0xB => ThumbRegisterOperation::Cmn,
            0xC => ThumbRegisterOperation::Orr,
            0xD => ThumbRegisterOperation::Mul,
            0xE => ThumbRegisterOperation::Bic,
            0xF => ThumbRegisterOperation::Mvn,
            _ => unreachable!(),
        };

        Some(ThumbInstructionType::Register {
            operation: operation_type,
            destination_register: dest_register,
            source: dest_register,
            second_operand: ThumbRegisterOrImmediate::Register(source_register),
        })
    }

    fn try_decode_thumb_high_register_operations_branch_exchange(
        opcode: u16,
    ) -> Option<ThumbInstructionType> {
        const MUST_BE_010001_BIT_RANGE: RangeInclusive<usize> = 10..=15;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 8..=9;
        const DEST_REGISTER_MSB_BL_FLAG_INDEX: usize = 7;
        const SOURCE_REGISTER_BIT_RANGE: RangeInclusive<usize> = 3..=6;
        const DEST_REGISTER_LOW_BIT_RANGE: RangeInclusive<usize> = 0..=2;

        const DEST_REGISTER_MSB_SHIFT: usize = 3;

        if opcode.get_bit_range(MUST_BE_010001_BIT_RANGE) != 0b010001 {
            return None;
        }

        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);

        let dest_register_msb_bl_flag = opcode.get_bit(DEST_REGISTER_MSB_BL_FLAG_INDEX);
        let dest_register_index = if dest_register_msb_bl_flag {
            opcode.get_bit_range(DEST_REGISTER_LOW_BIT_RANGE) | (1 << DEST_REGISTER_MSB_SHIFT)
        } else {
            opcode.get_bit_range(DEST_REGISTER_LOW_BIT_RANGE)
        };
        let dest_register = Register::from_index(u32::from(dest_register_index));

        let source_register_index = opcode.get_bit_range(SOURCE_REGISTER_BIT_RANGE);
        let source_register = Register::from_index(u32::from(source_register_index));

        Some(match opcode_value {
            0 => ThumbInstructionType::HighRegister {
                destination_register: dest_register,
                source: source_register,
                operation: ThumbHighRegisterOperation::Add,
            },
            1 => ThumbInstructionType::HighRegister {
                destination_register: dest_register,
                source: source_register,
                operation: ThumbHighRegisterOperation::Cmp,
            },
            2 => ThumbInstructionType::HighRegister {
                destination_register: dest_register,
                source: source_register,
                operation: ThumbHighRegisterOperation::Mov,
            },
            3 => {
                if dest_register_msb_bl_flag {
                    // blx
                    todo!()
                } else {
                    ThumbInstructionType::Bx {
                        operand: source_register,
                    }
                }
            }
            _ => unreachable!(),
        })
    }

    fn try_decode_thumb_memory_load_store(opcode: u16) -> Option<ThumbInstructionType> {
        None.or_else(|| Self::try_decode_thumb_load_pc_relative(opcode))
            .or_else(|| Self::try_decode_thumb_load_store_register_offset(opcode))
            .or_else(|| Self::try_decode_thumb_load_store_sign_extended_byte_halfword(opcode))
            .or_else(|| Self::try_decode_thumb_load_store_immediate_offset(opcode))
            .or_else(|| Self::try_decode_thumb_load_store_halfword(opcode))
            .or_else(|| Self::try_decode_thumb_load_store_sp_relative(opcode))
    }

    fn try_decode_thumb_load_pc_relative(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_01001_BIT_RANGE: RangeInclusive<usize> = 11..=15;
        const DEST_REGISTER_OFFSET: usize = 8;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        if opcode.get_bit_range(MUST_BE_01001_BIT_RANGE) != 0b01001 {
            return None;
        }

        let dest_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);
        let offset = u32::from(opcode.get_bit_range(OFFSET_BIT_RANGE)) * 4;

        Some(ThumbInstructionType::Ldr {
            base_register: Register::R15,
            destination_register: dest_register,
            offset: ThumbRegisterOrImmediate::Immediate(offset),
            sign_extend: false,
            size: ThumbLoadStoreDataSize::Word,
        })
    }

    fn try_decode_thumb_load_store_register_offset(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_0101_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 10..=11;
        const MUST_BE_0_BIT_INDEX: usize = 9;
        const OFFSET_REGISTER_OFFSET: usize = 6;
        const BASE_REGISTER_OFFSET: usize = 3;
        const SOURCE_DEST_REGISTER_OFFSET: usize = 0;

        const STR_WORD_OPCODE_VALUE: u16 = 0b00;
        const STR_BYTE_OPCODE_VALUE: u16 = 0b01;
        const LDR_WORD_OPCODE_VALUE: u16 = 0b10;
        const LDR_BYTE_OPCODE_VALUE: u16 = 0b11;

        if opcode.get_bit_range(MUST_BE_0101_BIT_RANGE) != 0b0101 {
            return None;
        }

        if opcode.get_bit(MUST_BE_0_BIT_INDEX) {
            return None;
        }

        let offset_register = opcode.get_register_at_offset(OFFSET_REGISTER_OFFSET);
        let offset = ThumbRegisterOrImmediate::Register(offset_register);
        let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);
        let source_dest_register = opcode.get_register_at_offset(SOURCE_DEST_REGISTER_OFFSET);

        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);
        Some(match opcode_value {
            STR_WORD_OPCODE_VALUE => ThumbInstructionType::Str {
                base_register,
                offset,
                size: ThumbLoadStoreDataSize::Word,
                source_register: source_dest_register,
            },
            STR_BYTE_OPCODE_VALUE => ThumbInstructionType::Str {
                base_register,
                offset,
                size: ThumbLoadStoreDataSize::Byte,
                source_register: source_dest_register,
            },
            LDR_WORD_OPCODE_VALUE => ThumbInstructionType::Ldr {
                base_register,
                offset,
                size: ThumbLoadStoreDataSize::Word,
                destination_register: source_dest_register,
                sign_extend: false,
            },
            LDR_BYTE_OPCODE_VALUE => ThumbInstructionType::Ldr {
                base_register,
                offset,
                size: ThumbLoadStoreDataSize::Byte,
                destination_register: source_dest_register,
                sign_extend: false,
            },
            _ => unreachable!(),
        })
    }

    fn try_decode_thumb_load_store_sign_extended_byte_halfword(
        opcode: u16,
    ) -> Option<ThumbInstructionType> {
        const MUST_BE_0101_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 10..=11;
        const MUST_BE_1_BIT_INDEX: usize = 9;
        const OFFSET_REGISTER_OFFSET: usize = 6;
        const BASE_REGISTER_OFFSET: usize = 3;
        const SOURCE_DEST_REGISTER_OFFSET: usize = 0;

        const STRH_OPCODE_VALUE: u16 = 0;
        const LDSB_OPCODE_VALUE: u16 = 1;
        const LDRH_OPCODE_VALUE: u16 = 2;
        const LDSH_OPCODE_VALUE: u16 = 3;

        if opcode.get_bit_range(MUST_BE_0101_BIT_RANGE) != 0b0101 {
            return None;
        }

        if !opcode.get_bit(MUST_BE_1_BIT_INDEX) {
            return None;
        }

        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);
        let offset_register = opcode.get_register_at_offset(OFFSET_REGISTER_OFFSET);
        let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);
        let source_dest_register = opcode.get_register_at_offset(SOURCE_DEST_REGISTER_OFFSET);

        Some(match opcode_value {
            // STRH Rd,[Rb,Ro]  ;store 16bit data          HALFWORD[Rb+Ro] = Rd
            STRH_OPCODE_VALUE => ThumbInstructionType::Str {
                base_register,
                offset: ThumbRegisterOrImmediate::Register(offset_register),
                size: ThumbLoadStoreDataSize::HalfWord,
                source_register: source_dest_register,
            },
            // LDSB Rd,[Rb,Ro]  ;load sign-extended 8bit   Rd = BYTE[Rb+Ro]
            LDSB_OPCODE_VALUE => ThumbInstructionType::Ldr {
                base_register,
                offset: ThumbRegisterOrImmediate::Register(offset_register),
                sign_extend: true,
                destination_register: source_dest_register,
                size: ThumbLoadStoreDataSize::Byte,
            },
            // LDRH Rd,[Rb,Ro]  ;load zero-extended 16bit  Rd = HALFWORD[Rb+Ro]
            LDRH_OPCODE_VALUE => ThumbInstructionType::Ldr {
                base_register,
                offset: ThumbRegisterOrImmediate::Register(offset_register),
                sign_extend: false,
                destination_register: source_dest_register,
                size: ThumbLoadStoreDataSize::HalfWord,
            },
            // LDSH Rd,[Rb,Ro]  ;load sign-extended 16bit  Rd = HALFWORD[Rb+Ro]
            LDSH_OPCODE_VALUE => ThumbInstructionType::Ldr {
                base_register,
                offset: ThumbRegisterOrImmediate::Register(offset_register),
                sign_extend: true,
                destination_register: source_dest_register,
                size: ThumbLoadStoreDataSize::HalfWord,
            },
            _ => unreachable!(),
        })
    }

    fn try_decode_thumb_load_store_immediate_offset(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_011_BIT_RANGE: RangeInclusive<usize> = 13..=15;
        const ACCESS_SIZE_BIT_INDEX: usize = 12;
        const OPERATION_TYPE_BIT_INDEX: usize = 11;
        const OFFSET_VALUE_BIT_RANGE: RangeInclusive<usize> = 6..=10;
        const BASE_REGISTER_OFFSET: usize = 3;
        const SOURCE_DEST_REGISTER_OFFSET: usize = 0;

        if opcode.get_bit_range(MUST_BE_011_BIT_RANGE) != 0b011 {
            return None;
        }

        let access_size_bit = opcode.get_bit(ACCESS_SIZE_BIT_INDEX);
        let operation_type_bit = opcode.get_bit(OPERATION_TYPE_BIT_INDEX);
        let raw_offset = opcode.get_bit_range(OFFSET_VALUE_BIT_RANGE);
        let (access_size, offset) = if access_size_bit {
            // byte access
            (ThumbLoadStoreDataSize::Byte, u32::from(raw_offset))
        } else {
            // word access
            (ThumbLoadStoreDataSize::Word, u32::from(raw_offset * 4))
        };
        let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);
        let source_dest_register = opcode.get_register_at_offset(SOURCE_DEST_REGISTER_OFFSET);

        Some(if operation_type_bit {
            // ldr
            ThumbInstructionType::Ldr {
                base_register,
                offset: ThumbRegisterOrImmediate::Immediate(offset),
                destination_register: source_dest_register,
                sign_extend: false,
                size: access_size,
            }
        } else {
            // str
            ThumbInstructionType::Str {
                base_register,
                offset: ThumbRegisterOrImmediate::Immediate(offset),
                size: access_size,
                source_register: source_dest_register,
            }
        })
    }

    fn try_decode_thumb_load_store_halfword(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_1000_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_INDEX: usize = 11;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 6..=10;
        const BASE_REGISTER_OFFSET: usize = 3;
        const SOURCE_DEST_REGISTER_OFFSET: usize = 0;

        if opcode.get_bit_range(MUST_BE_1000_BIT_RANGE) != 0b1000 {
            return None;
        }

        let opcode_value_bit = opcode.get_bit(OPCODE_VALUE_BIT_INDEX);
        let offset = opcode.get_bit_range(OFFSET_BIT_RANGE) * 2;
        let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);
        let source_dest_register = opcode.get_register_at_offset(SOURCE_DEST_REGISTER_OFFSET);

        Some(if opcode_value_bit {
            // LDRH Rd,[Rb,#nn]  ;load  16bit data   Rd = HALFWORD[Rb+nn]
            ThumbInstructionType::Ldr {
                base_register,
                destination_register: source_dest_register,
                offset: ThumbRegisterOrImmediate::Immediate(u32::from(offset)),
                sign_extend: false,
                size: ThumbLoadStoreDataSize::HalfWord,
            }
        } else {
            // STRH Rd,[Rb,#nn]  ;store 16bit data   HALFWORD[Rb+nn] = Rd
            ThumbInstructionType::Str {
                base_register,
                source_register: source_dest_register,
                offset: ThumbRegisterOrImmediate::Immediate(u32::from(offset)),
                size: ThumbLoadStoreDataSize::HalfWord,
            }
        })
    }

    fn try_decode_thumb_load_store_sp_relative(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_1001_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_INDEX: usize = 11;
        const SOURCE_DEST_REGISTER_OFFSET: usize = 8;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        if opcode.get_bit_range(MUST_BE_1001_BIT_RANGE) != 0b1001 {
            return None;
        }

        let opcode_value_bit = opcode.get_bit(OPCODE_VALUE_BIT_INDEX);

        let source_dest_register = opcode.get_register_at_offset(SOURCE_DEST_REGISTER_OFFSET);
        let offset = u32::from(opcode.get_bit_range(OFFSET_BIT_RANGE) * 4);

        Some(if opcode_value_bit {
            // LDR  Rd,[SP,#nn]
            ThumbInstructionType::Ldr {
                base_register: Register::R13,
                destination_register: source_dest_register,
                offset: ThumbRegisterOrImmediate::Immediate(offset),
                sign_extend: false,
                size: ThumbLoadStoreDataSize::Word,
            }
        } else {
            // STR  Rd,[SP,#nn]
            ThumbInstructionType::Str {
                base_register: Register::R13,
                offset: ThumbRegisterOrImmediate::Immediate(offset),
                size: ThumbLoadStoreDataSize::Word,
                source_register: source_dest_register,
            }
        })
    }

    fn try_decode_thumb_memory_addressing(opcode: u16) -> Option<ThumbInstructionType> {
        None.or_else(|| Self::try_decode_thumb_get_relative_address(opcode))
            .or_else(|| Self::try_decode_thumb_add_offset_stack_pointer(opcode))
    }

    fn try_decode_thumb_get_relative_address(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_1010_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_INDEX: usize = 11;
        const DEST_REGISTER_OFFSET: usize = 8;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        if opcode.get_bit_range(MUST_BE_1010_BIT_RANGE) != 0b1010 {
            return None;
        }

        let opcode_value_bit = opcode.get_bit(OPCODE_VALUE_BIT_INDEX);
        let dest_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);
        let offset = opcode.get_bit_range(OFFSET_BIT_RANGE) * 4;

        Some(if opcode_value_bit {
            // 1: ADD  Rd,SP,#nn    ;Rd = SP + nn
            ThumbInstructionType::AddSpecial {
                dest_register,
                source_register: Register::R13,
                sign_bit: false,
                unsigned_offset: offset,
            }
        } else {
            // 0: ADD  Rd,PC,#nn    ;Rd = (($+4) AND NOT 2) + nn
            ThumbInstructionType::AddSpecial {
                dest_register,
                source_register: Register::R15,
                sign_bit: false,
                unsigned_offset: offset,
            }
        })
    }

    fn try_decode_thumb_add_offset_stack_pointer(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_10110000_BIT_RANGE: RangeInclusive<usize> = 8..=15;
        const OPCODE_VALUE_BIT_INDEX: usize = 7;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=6;

        if opcode.get_bit_range(MUST_BE_10110000_BIT_RANGE) != 0b10110000 {
            return None;
        }

        let opcode_sign_bit = opcode.get_bit(OPCODE_VALUE_BIT_INDEX);
        let unsigned_offset = opcode.get_bit_range(OFFSET_BIT_RANGE) * 4;

        Some(ThumbInstructionType::AddSpecial {
            dest_register: Register::R13,
            source_register: Register::R13,
            unsigned_offset,
            sign_bit: opcode_sign_bit,
        })
    }

    fn try_decode_thumb_memory_multiple_load_store(opcode: u16) -> Option<ThumbInstructionType> {
        None.or_else(|| Self::try_decode_thumb_push_pop_regs(opcode))
            .or_else(|| Self::try_decode_thumb_multiple_load_store(opcode))
    }

    fn try_decode_thumb_push_pop_regs(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_1011_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_INDEX: usize = 11;
        const MUST_BE_10_BIT_RANGE: RangeInclusive<usize> = 9..=10;
        const PC_LR_BIT_INDEX: usize = 8;
        const REGISTER_LIST_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        if opcode.get_bit_range(MUST_BE_1011_BIT_RANGE) != 0b1011 {
            return None;
        }

        if opcode.get_bit_range(MUST_BE_10_BIT_RANGE) != 0b10 {
            return None;
        }

        let opcode_value_bit = opcode.get_bit(OPCODE_VALUE_BIT_INDEX);
        let pc_lr_bit = opcode.get_bit(PC_LR_BIT_INDEX);
        let register_list_raw = opcode.get_bit_range(REGISTER_LIST_BIT_RANGE);

        let mut register_bit_list = [false; 8];
        for (register_index, register_used_bit) in register_bit_list.iter_mut().enumerate() {
            let register_used = register_list_raw.get_bit(register_index);
            *register_used_bit = register_used;
        }

        Some(if opcode_value_bit {
            // 1: POP  {Rlist}{PC}   ;load from memory, increments SP (R13)
            ThumbInstructionType::Pop {
                register_bit_list,
                pop_pc: pc_lr_bit,
            }
        } else {
            // 0: PUSH {Rlist}{LR}   ;store in memory, decrements SP (R13)
            ThumbInstructionType::Push {
                register_bit_list,
                push_lr: pc_lr_bit,
            }
        })
    }

    fn try_decode_thumb_multiple_load_store(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_1100_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_INDEX: usize = 11;
        const BASE_REGISTER_OFFSET: usize = 8;
        const REGISTER_LIST_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        if opcode.get_bit_range(MUST_BE_1100_BIT_RANGE) != 0b1100 {
            return None;
        }

        let opcode_value_bit = opcode.get_bit(OPCODE_VALUE_BIT_INDEX);
        let base_register = opcode.get_register_at_offset(BASE_REGISTER_OFFSET);
        let register_bit_list_raw = opcode.get_bit_range(REGISTER_LIST_BIT_RANGE);

        let mut register_bit_list = [false; 8];
        for (bit_list_index, register_used_bit) in register_bit_list.iter_mut().enumerate() {
            let register_used = register_bit_list_raw.get_bit(bit_list_index);
            *register_used_bit = register_used;
        }

        Some(if opcode_value_bit {
            // LDMIA
            ThumbInstructionType::LdmiaWriteBack {
                base_register,
                register_bit_list,
            }
        } else {
            // STMIA
            ThumbInstructionType::StmiaWriteBack {
                base_register,
                register_bit_list,
            }
        })
    }

    fn try_decode_thumb_jump_call(opcode: u16) -> Option<ThumbInstructionType> {
        None.or_else(|| Self::try_decode_thumb_conditional_branch(opcode))
            .or_else(|| Self::try_decode_thumb_unconditional_branch(opcode))
            .or_else(|| Self::try_decode_thumb_long_branch_link_1(opcode))
            .or_else(|| Self::try_decode_thumb_long_branch_link_2(opcode))
    }

    fn try_decode_thumb_conditional_branch(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_1101_BIT_RANGE: RangeInclusive<usize> = 12..=15;
        const OPCODE_VALUE_BIT_RANGE: RangeInclusive<usize> = 8..=11;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        if opcode.get_bit_range(MUST_BE_1101_BIT_RANGE) != 0b1101 {
            return None;
        }

        let opcode_value = opcode.get_bit_range(OPCODE_VALUE_BIT_RANGE);

        let branch_condition = match opcode_value {
            0x0 => InstructionCondition::Equal,
            0x1 => InstructionCondition::NotEqual,
            0x2 => InstructionCondition::UnsignedHigherOrSame,
            0x3 => InstructionCondition::UnsignedLower,
            0x4 => InstructionCondition::SignedNegative,
            0x5 => InstructionCondition::SignedPositiveOrZero,
            0x6 => InstructionCondition::SignedOverflow,
            0x7 => InstructionCondition::SignedNoOverflow,
            0x8 => InstructionCondition::UnsignedHigher,
            0x9 => InstructionCondition::UnsignedLowerOrSame,
            0xA => InstructionCondition::SignedGreaterOrEqual,
            0xB => InstructionCondition::SignedLessThan,
            0xC => InstructionCondition::SignedGreaterThan,
            0xD => InstructionCondition::SignedLessOrEqual,
            0xE => unreachable!("Undefined"),
            0xF => todo!("Swi"),
            _ => unreachable!(),
        };

        let jump_offset = (opcode.get_bit_range(OFFSET_BIT_RANGE) as u8 as i8 as i16) * 2;

        Some(ThumbInstructionType::B {
            condition: branch_condition,
            offset: jump_offset,
        })
    }

    fn try_decode_thumb_unconditional_branch(opcode: u16) -> Option<ThumbInstructionType> {
        const MUST_BE_11100_BIT_RANGE: RangeInclusive<usize> = 11..=15;
        const OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=10;

        if opcode.get_bit_range(MUST_BE_11100_BIT_RANGE) != 0b11100 {
            return None;
        }

        let unsigned_offset = opcode.get_bit_range(OFFSET_BIT_RANGE);

        // 11-bit sign extension, by left shifting until effective sign bit is in MSB, then ASR
        // an equal amount back over.
        let offset = (((unsigned_offset as i16) << 5) >> 5) * 2;

        Some(ThumbInstructionType::B {
            condition: InstructionCondition::Always,
            offset,
        })
    }

    // First Instruction - LR = PC+4+(nn SHL 12)
    // 15-11  Must be 11110b for BL/BLX type of instructions
    // 10-0   nn - Upper 11 bits of Target Address
    fn try_decode_thumb_long_branch_link_1(opcode: u16) -> Option<ThumbInstructionType> {
        const OPCODE_MUST_BE_11110_BIT_RANGE: RangeInclusive<usize> = 11..=15;
        const OPCODE_TARGET_ADDRESS_UPPER_11_BITS_RANGE: RangeInclusive<usize> = 0..=10;

        if opcode.get_bit_range(OPCODE_MUST_BE_11110_BIT_RANGE) != 0b11110 {
            return None;
        }

        let offset_unsigned =
            u32::from(opcode.get_bit_range(OPCODE_TARGET_ADDRESS_UPPER_11_BITS_RANGE)) << 12;

        // 23-bit sign extension, by left shifting until effective sign bit is in MSB, then ASR
        // an equal amount back over.
        let offset = ((offset_unsigned as i32) << 9) >> 9;

        Some(ThumbInstructionType::BlPartOne { offset })
    }

    // Second Instruction - PC = LR + (nn SHL 1), and LR = PC+2 OR 1 (and BLX: T=0)
    // 15-11  Opcode
    //      11111b: BL label   ;branch long with link
    //      11101b: BLX label  ;branch long with link switch to ARM mode (ARM9) (UNUSED)
    // 10-0   nn - Lower 11 bits of Target Address (BLX: Bit0 Must be zero)
    fn try_decode_thumb_long_branch_link_2(opcode: u16) -> Option<ThumbInstructionType> {
        const OPCODE_MUST_BE_11111_BIT_RANGE: RangeInclusive<usize> = 11..=15;
        const OPCODE_TARGET_ADDRESS_LOWER_11_BITS_RANGE: RangeInclusive<usize> = 0..=10;

        if opcode.get_bit_range(OPCODE_MUST_BE_11111_BIT_RANGE) != 0b11111 {
            return None;
        }

        let offset = opcode.get_bit_range(OPCODE_TARGET_ADDRESS_LOWER_11_BITS_RANGE) << 1;

        Some(ThumbInstructionType::BlPartTwo { offset })
    }
}

impl Cpu {
    fn execute_arm(&mut self, instruction: ArmInstruction) {
        if self.evaluate_instruction_condition(instruction.condition) {
            match instruction.instruction_type {
                ArmInstructionType::Alu {
                    operation,
                    first_operand,
                    second_operand,
                    destination_operand,
                    set_conditions,
                } => self.execute_arm_alu(
                    operation,
                    first_operand,
                    second_operand,
                    destination_operand,
                    set_conditions,
                ),
                ArmInstructionType::B { offset } => self.execute_arm_b(offset),
                ArmInstructionType::Bl { offset } => self.execute_arm_bl(offset),
                ArmInstructionType::Bx { operand } => self.execute_arm_bx(operand),
                ArmInstructionType::Msr {
                    destination_psr,
                    source_info,
                    write_control_field,
                    write_extension_field,
                    write_flags_field,
                    write_status_field,
                } => self.execute_arm_msr(
                    destination_psr,
                    source_info,
                    write_control_field,
                    write_extension_field,
                    write_flags_field,
                    write_status_field,
                ),
                ArmInstructionType::Mrs {
                    destination_register,
                    source_psr,
                } => self.execute_arm_mrs(destination_register, source_psr),
                ArmInstructionType::Ldr {
                    access_size,
                    base_register,
                    destination_register,
                    index_type,
                    offset_info,
                    sign_extend,
                } => self.execute_arm_ldr(
                    access_size,
                    base_register,
                    destination_register,
                    index_type,
                    offset_info,
                    sign_extend,
                ),
                ArmInstructionType::Str {
                    access_size,
                    base_register,
                    index_type,
                    offset_info,
                    source_register,
                } => self.execute_arm_str(
                    access_size,
                    base_register,
                    index_type,
                    offset_info,
                    source_register,
                ),
                ArmInstructionType::Ldm {
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                } => self.execute_arm_ldm(
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                ),
                ArmInstructionType::Stm {
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                } => self.execute_arm_stm(
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                ),
                ArmInstructionType::Mul {
                    operation,
                    set_conditions,
                    destination_register,
                    accumulate_register,
                    operand_register_rm,
                    operand_register_rs,
                } => self.execute_arm_mul(
                    operation,
                    set_conditions,
                    destination_register,
                    accumulate_register,
                    operand_register_rm,
                    operand_register_rs,
                ),
                ArmInstructionType::Swi { comment: _ } => self.handle_exception(ExceptionType::Swi),
                _ => todo!("{:#08x?}", instruction),
            }
        }
    }

    fn execute_thumb(&mut self, instruction: ThumbInstruction) {
        match instruction.instruction_type {
            ThumbInstructionType::Register {
                operation,
                destination_register,
                source,
                second_operand,
            } => self.execute_thumb_register_operation(
                operation,
                destination_register,
                source,
                second_operand,
            ),
            ThumbInstructionType::HighRegister {
                destination_register,
                operation,
                source,
            } => {
                self.execute_thumb_high_register_operation(destination_register, operation, source)
            }
            ThumbInstructionType::Ldr {
                base_register,
                offset,
                destination_register,
                size,
                sign_extend,
            } => self.execute_thumb_ldr(
                base_register,
                offset,
                destination_register,
                size,
                sign_extend,
            ),
            ThumbInstructionType::Str {
                base_register,
                offset,
                source_register,
                size,
            } => self.execute_thumb_str(base_register, offset, source_register, size),
            ThumbInstructionType::B { condition, offset } => {
                self.execute_thumb_b(condition, offset)
            }
            ThumbInstructionType::BlPartOne { offset } => self.execute_thumb_bl_part_1(offset),
            ThumbInstructionType::BlPartTwo { offset } => self.execute_thumb_bl_part_2(offset),
            ThumbInstructionType::Bx { operand } => self.execute_thumb_bx(operand),
            ThumbInstructionType::Push {
                register_bit_list,
                push_lr,
            } => self.execute_thumb_push(register_bit_list, push_lr),
            ThumbInstructionType::Pop {
                register_bit_list,
                pop_pc,
            } => self.execute_thumb_pop(register_bit_list, pop_pc),
            ThumbInstructionType::StmiaWriteBack {
                base_register,
                register_bit_list,
            } => self.execute_thumb_stmia_write_back(base_register, register_bit_list),
            ThumbInstructionType::LdmiaWriteBack {
                base_register,
                register_bit_list,
            } => self.execute_thumb_ldmia_write_back(base_register, register_bit_list),
            ThumbInstructionType::AddSpecial {
                source_register,
                dest_register,
                sign_bit,
                unsigned_offset,
            } => self.execute_thumb_add_special(
                source_register,
                dest_register,
                sign_bit,
                unsigned_offset,
            ),
            _ => todo!("{:#016x?}", instruction),
        }
    }
}

impl Cpu {
    fn execute_arm_alu(
        &mut self,
        operation: AluOperation,
        first_operand: Register,
        second_operand: AluSecondOperandInfo,
        destination_operand: Register,
        set_conditions: bool,
    ) {
        // When using R15 as operand (Rm or Rn), the returned value depends on the instruction:
        //   - $+12 if I=0,R=1 (shift by register)
        //   - otherwise, $+8 (shift by immediate).
        //
        // Note that that pc = $ + 4 due to decoding step.
        let pc_operand_calculation = match second_operand {
            AluSecondOperandInfo::Register {
                shift_info: ArmRegisterOrImmediate::Register(_),
                ..
            } => |pc| pc + 8,
            _ => |pc| pc + 4,
        };

        let first_operand_value = self.read_register(first_operand, pc_operand_calculation);
        let (second_operand_value, shifter_carry_out) =
            self.evaluate_alu_second_operand(second_operand);
        let old_overflow = self.get_overflow_flag();

        let (unsigned_result, carry_flag, signed_result, overflow_flag) = match operation {
            AluOperation::And => {
                let unsigned_result = first_operand_value & second_operand_value;
                let signed_result = unsigned_result as i32;

                (
                    unsigned_result,
                    shifter_carry_out,
                    signed_result,
                    old_overflow,
                )
            }
            AluOperation::Add => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_add(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32);

                (unsigned_result, carry, signed_result, overflow)
            }
            AluOperation::Adc => {
                let (unsigned_result, carry) = if self.get_carry_flag() {
                    let (intermediate_unsigned_result, carry_1) =
                        first_operand_value.overflowing_add(second_operand_value);
                    let (final_unsigned_result, carry_2) =
                        intermediate_unsigned_result.overflowing_add(1);
                    (final_unsigned_result, carry_1 | carry_2)
                } else {
                    first_operand_value.overflowing_add(second_operand_value)
                };

                let (signed_result, overflow) = if self.get_carry_flag() {
                    let (intermediate_signed_result, carry_1) =
                        (first_operand_value as i32).overflowing_add(second_operand_value as i32);
                    let (final_signed_result, carry_2) =
                        intermediate_signed_result.overflowing_add(1);
                    (final_signed_result, carry_1 | carry_2)
                } else {
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32)
                };

                (unsigned_result, carry, signed_result, overflow)
            }
            AluOperation::Sub => {
                let (unsigned_result, borrow) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);

                (unsigned_result, !borrow, signed_result, overflow)
            }
            AluOperation::Sbc => {
                let borrow_in = !self.get_carry_flag();

                let (unsigned_result, borrow) = if borrow_in {
                    let (result_1, borrow_1) =
                        first_operand_value.overflowing_sub(second_operand_value);
                    let (unsigned_result, borrow_2) = result_1.overflowing_sub(1);
                    (unsigned_result, borrow_1 | borrow_2)
                } else {
                    first_operand_value.overflowing_sub(second_operand_value)
                };

                let (signed_result, overflow) = if borrow_in {
                    let (result_1, overflow_1) =
                        (first_operand_value as i32).overflowing_sub(second_operand_value as i32);
                    let (signed_result, overflow_2) = result_1.overflowing_sub(1);
                    (signed_result, overflow_1 | overflow_2)
                } else {
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32)
                };

                (unsigned_result, !borrow, signed_result, overflow)
            }
            AluOperation::Rsc => {
                let borrow_in = !self.get_carry_flag();

                let (unsigned_result, borrow) = if borrow_in {
                    let (result_1, borrow_1) =
                        second_operand_value.overflowing_sub(first_operand_value);
                    let (unsigned_result, borrow_2) = result_1.overflowing_sub(1);
                    (unsigned_result, borrow_1 | borrow_2)
                } else {
                    second_operand_value.overflowing_sub(first_operand_value)
                };

                let (signed_result, overflow) = if borrow_in {
                    let (result_1, overflow_1) =
                        (second_operand_value as i32).overflowing_sub(first_operand_value as i32);
                    let (signed_result, overflow_2) = result_1.overflowing_sub(1);
                    (signed_result, overflow_1 | overflow_2)
                } else {
                    (second_operand_value as i32).overflowing_sub(first_operand_value as i32)
                };

                (unsigned_result, !borrow, signed_result, overflow)
            }
            AluOperation::Rsb => {
                let (unsigned_result, borrow) =
                    second_operand_value.overflowing_sub(first_operand_value);
                let (signed_result, overflow) =
                    (second_operand_value as i32).overflowing_sub(first_operand_value as i32);

                (unsigned_result, !borrow, signed_result, overflow)
            }
            AluOperation::Teq => {
                let unsigned_result = first_operand_value ^ second_operand_value;
                let signed_result = unsigned_result as i32;

                (
                    unsigned_result,
                    shifter_carry_out,
                    signed_result,
                    old_overflow,
                )
            }
            AluOperation::Cmp => {
                let (unsigned_result, borrow) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);

                (unsigned_result, !borrow, signed_result, overflow)
            }
            AluOperation::Cmn => {
                let (unsigned_result, borrow) =
                    first_operand_value.overflowing_add(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32);

                (unsigned_result, !borrow, signed_result, overflow)
            }
            AluOperation::Mov => (
                second_operand_value,
                shifter_carry_out,
                second_operand_value as i32,
                old_overflow,
            ),
            AluOperation::Bic => {
                let result = first_operand_value & (!second_operand_value);
                (result, shifter_carry_out, result as i32, old_overflow)
            }
            AluOperation::Tst => {
                let result = first_operand_value & second_operand_value;
                (result, shifter_carry_out, result as i32, old_overflow)
            }
            AluOperation::Orr => {
                let result = first_operand_value | second_operand_value;
                (result, shifter_carry_out, result as i32, old_overflow)
            }
            AluOperation::Eor => {
                let result = first_operand_value ^ second_operand_value;
                (result, shifter_carry_out, result as i32, old_overflow)
            }
            AluOperation::Mvn => {
                let result = !second_operand_value;
                (result, shifter_carry_out, result as i32, old_overflow)
            }
            _ => todo!("ARM ALU: {:?}", operation),
        };

        if set_conditions {
            self.set_sign_flag(signed_result < 0);
            self.set_zero_flag(unsigned_result == 0);
            self.set_carry_flag(carry_flag);
            self.set_overflow_flag(overflow_flag);

            // If S=1, Rd=R15; should not be used in user mode:
            //  CPSR = SPSR_<current mode>
            //  PC = result
            //  For example: MOVS PC,R14  ;return from SWI (PC=R14_svc, CPSR=SPSR_svc).

            if matches!(destination_operand, Register::R15) {
                let saved_cpsr = self.read_register(Register::Spsr, |_| unreachable!());
                self.cpsr = saved_cpsr;
            }
        }

        if matches!(
            operation,
            AluOperation::And
                | AluOperation::Eor
                | AluOperation::Sub
                | AluOperation::Rsb
                | AluOperation::Add
                | AluOperation::Adc
                | AluOperation::Sbc
                | AluOperation::Rsc
                | AluOperation::Orr
                | AluOperation::Mov
                | AluOperation::Bic
                | AluOperation::Mvn
        ) {
            self.write_register(unsigned_result, destination_operand);
        }
    }

    // pc is already at $ + 4 because of decoding step.
    // documentation specifies that branch is to ($ + offset + 8).
    fn execute_arm_b(&mut self, offset: i32) {
        let old_pc = self.read_register(Register::R15, |pc| pc);
        let new_pc = old_pc.wrapping_add(offset as u32).wrapping_add(4);
        if DEBUG_AND_PANIC_ON_LOOP && (old_pc - 4) == new_pc {
            panic!("infinite loop");
        }
        self.write_register(new_pc, Register::R15);
    }

    // PC is already at $ + 4 because of decoding step.
    // documentation specifies that branch is to ($ + offset + 8).
    // save ($ + 4) in lr.
    fn execute_arm_bl(&mut self, offset: i32) {
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.write_register(old_pc, Register::R14);

        let new_pc = old_pc.wrapping_add(offset as u32).wrapping_add(4);
        self.write_register(new_pc, Register::R15);
    }

    // PC = operand, T = Rn.0
    fn execute_arm_bx(&mut self, operand: Register) {
        const NEW_STATE_BIT_INDEX: usize = 0;

        let operand_value = self.read_register(operand, |_| todo!());

        let new_state_bit = operand_value.get_bit(NEW_STATE_BIT_INDEX);
        self.set_cpu_state_bit(new_state_bit);

        let new_pc = operand_value & (!1);

        self.write_register(new_pc, Register::R15);
    }

    fn execute_arm_msr(
        &mut self,
        destination_psr: PsrTransferPsr,
        source_info: MsrSourceInfo,
        write_control_field: bool,
        write_extension_field: bool,
        write_flags_field: bool,
        write_status_field: bool,
    ) {
        let original_mode = self.get_cpu_mode();
        const FLAGS_FIELD_MASK: u32 = 0b11111111_00000000_00000000_00000000;
        const STATUS_FIELD_MASK: u32 = 0b00000000_11111111_00000000_00000000;
        const EXTENSION_FIELD_MASK: u32 = 0b00000000_00000000_11111111_00000000;
        const CONTROL_FIELD_MASK: u32 = 0b00000000_00000000_00000000_11111111;

        let source_value = match source_info {
            MsrSourceInfo::Immediate { value } => value,
            MsrSourceInfo::Register(register) => self.read_register(register, |pc| pc),
        };

        let mut write_mask = 0;
        if write_flags_field {
            write_mask |= FLAGS_FIELD_MASK;
        }

        if write_status_field {
            write_mask |= STATUS_FIELD_MASK;
        }

        if write_extension_field {
            write_mask |= EXTENSION_FIELD_MASK;
        }

        if write_control_field {
            write_mask |= CONTROL_FIELD_MASK;
        }

        let psr_register = match destination_psr {
            PsrTransferPsr::Cpsr => Register::Cpsr,
            PsrTransferPsr::Spsr => Register::Spsr,
        };

        let original_psr_value = self.read_register(psr_register, |pc| pc);
        let new_psr_value = (source_value & write_mask) | (original_psr_value & (!write_mask));

        self.write_register(new_psr_value, psr_register);
    }

    fn execute_arm_mrs(&mut self, destination_register: Register, source_psr: PsrTransferPsr) {
        let source_psr_value = match source_psr {
            PsrTransferPsr::Cpsr => self.read_register(Register::Cpsr, |_| unreachable!()),
            PsrTransferPsr::Spsr => self.read_register(Register::Spsr, |_| unreachable!()),
        };

        self.write_register(source_psr_value, destination_register);
    }

    fn execute_arm_str(
        &mut self,
        access_size: SingleDataMemoryAccessSize,
        base_register: Register,
        index_type: SingleDataTransferIndexType,
        offset_info: SingleDataTransferOffsetInfo,
        source_register: Register,
    ) {
        // "including R15=PC+8".
        let base_address = self.read_register(base_register, |pc| pc + 4);

        let offset_amount = match offset_info.value {
            SingleDataTransferOffsetValue::Immediate { offset } => offset,
            SingleDataTransferOffsetValue::Register { offset_register } => {
                self.read_register(offset_register, |pc| pc)
            }
            SingleDataTransferOffsetValue::RegisterImmediate {
                offset_register,
                shift_amount,
                shift_type,
            } => {
                assert!(!matches!(offset_register, Register::R15));

                let offset_register_value = self.read_register(offset_register, |pc| pc);
                shift_type.evaluate(offset_register_value, shift_amount)
            }
        };

        let offset_address = if offset_info.sign {
            base_address - offset_amount
        } else {
            base_address + offset_amount
        };

        let actual_address = match index_type {
            SingleDataTransferIndexType::PostIndex { .. } => {
                // post index always has write-back
                self.write_register(offset_address, base_register);
                base_address
            }
            SingleDataTransferIndexType::PreIndex { write_back } => {
                if write_back {
                    self.write_register(offset_address, base_register);
                }

                offset_address
            }
        };

        // "including R15=PC+12"
        let value = self.read_register(source_register, |pc| pc + 4);
        match access_size {
            SingleDataMemoryAccessSize::Byte => {
                self.bus.write_byte_address(value as u8, actual_address)
            }
            SingleDataMemoryAccessSize::Word => self.bus.write_word_address(value, actual_address),
            SingleDataMemoryAccessSize::HalfWord => self
                .bus
                .write_halfword_address(value as u16, actual_address),
            _ => todo!("{:?}", access_size),
        };
    }

    fn execute_arm_ldr(
        &mut self,
        access_size: SingleDataMemoryAccessSize,
        base_register: Register,
        destination_register: Register,
        index_type: SingleDataTransferIndexType,
        offset_info: SingleDataTransferOffsetInfo,
        sign_extend: bool,
    ) {
        // "including R15=PC+8"
        let base_address = self.read_register(base_register, |pc| pc + 4);

        let offset_amount = match offset_info.value {
            SingleDataTransferOffsetValue::Immediate { offset } => offset,
            SingleDataTransferOffsetValue::Register { offset_register } => {
                self.read_register(offset_register, |pc| pc)
            }
            SingleDataTransferOffsetValue::RegisterImmediate {
                offset_register,
                shift_amount,
                shift_type,
            } => {
                let offset_register_value = self.read_register(offset_register, |_| unreachable!());
                shift_type.evaluate(offset_register_value, shift_amount)
            }
        };

        let offset_address = if offset_info.sign {
            base_address - offset_amount
        } else {
            base_address + offset_amount
        };

        let data_read_address = match index_type {
            SingleDataTransferIndexType::PostIndex { .. } => {
                // post index always has write-back
                self.write_register(offset_address, base_register);
                base_address
            }
            SingleDataTransferIndexType::PreIndex { write_back } => {
                if write_back {
                    self.write_register(offset_address, base_register);
                }

                offset_address
            }
        };

        let value = match (access_size, sign_extend) {
            (SingleDataMemoryAccessSize::Byte, false) => {
                self.bus.read_byte_address(data_read_address) as u32
            }
            (SingleDataMemoryAccessSize::Byte, true) => {
                self.bus.read_byte_address(data_read_address) as i8 as i32 as u32
            }
            (SingleDataMemoryAccessSize::HalfWord, false) => {
                self.bus.read_halfword_address(data_read_address) as u32
            }
            (SingleDataMemoryAccessSize::HalfWord, true) => {
                self.bus.read_halfword_address(data_read_address) as i16 as i32 as u32
            }
            (SingleDataMemoryAccessSize::Word, false) => {
                self.bus.read_word_address(data_read_address)
            }
            (SingleDataMemoryAccessSize::Word, true) => unreachable!(),
            _ => todo!("{:?} sign extend: {}", access_size, sign_extend),
        };

        self.write_register(value, destination_register);
    }

    fn execute_arm_ldm(
        &mut self,
        index_type: BlockDataTransferIndexType,
        offset_modifier: OffsetModifierType,
        write_back: bool,
        base_register: Register,
        register_bit_list: [bool; 16],
    ) {
        // "not including R15".
        let mut current_address = self.read_register(base_register, |_| unreachable!());

        match offset_modifier {
            OffsetModifierType::AddToBase => {
                for (register_idx, register_loaded) in register_bit_list.into_iter().enumerate() {
                    if register_loaded {
                        if matches!(index_type, BlockDataTransferIndexType::PreIndex) {
                            current_address += 4;
                        }

                        let value = self.bus.read_word_address(current_address);
                        let register = Register::from_index(register_idx as u32);
                        self.write_register(value, register);

                        if matches!(index_type, BlockDataTransferIndexType::PostIndex) {
                            current_address += 4;
                        }
                    }
                }
            }
            OffsetModifierType::SubtractFromBase => {
                // Lowest register index goes at lowest address. When decrementing after load, lowest register index needs to be considered last.
                //  In order to achieve this, iterate in reverse order.
                for (register_idx, register_loaded) in
                    register_bit_list.into_iter().enumerate().rev()
                {
                    if register_loaded {
                        if matches!(index_type, BlockDataTransferIndexType::PreIndex) {
                            current_address -= 4;
                        }

                        let value = self.bus.read_word_address(current_address);
                        let register = Register::from_index(register_idx as u32);
                        self.write_register(value, register);

                        if matches!(index_type, BlockDataTransferIndexType::PostIndex) {
                            current_address -= 4;
                        }
                    }
                }
            }
        }

        if write_back {
            self.write_register(current_address, base_register);
        }
    }

    fn execute_arm_stm(
        &mut self,
        index_type: BlockDataTransferIndexType,
        offset_modifier: OffsetModifierType,
        write_back: bool,
        base_register: Register,
        register_bit_list: [bool; 16],
    ) {
        // "not including R15".
        let mut current_address = self.read_register(base_register, |_| unreachable!());

        match offset_modifier {
            OffsetModifierType::AddToBase => {
                for (register_idx, register_loaded) in register_bit_list.into_iter().enumerate() {
                    if register_loaded {
                        if matches!(index_type, BlockDataTransferIndexType::PreIndex) {
                            current_address += 4;
                        }

                        let register = Register::from_index(register_idx as u32);
                        let register_value = self.read_register(register, |_| unreachable!());
                        self.bus.write_word_address(register_value, current_address);

                        if matches!(index_type, BlockDataTransferIndexType::PostIndex) {
                            current_address += 4;
                        }
                    }
                }
            }
            OffsetModifierType::SubtractFromBase => {
                // Lowest register index goes at lowest address. When decrementing after store, lowest register index needs to be considered last.
                //  In order to achieve this, iterate in reverse order.
                for (register_idx, register_loaded) in
                    register_bit_list.into_iter().enumerate().rev()
                {
                    if register_loaded {
                        if matches!(index_type, BlockDataTransferIndexType::PreIndex) {
                            current_address -= 4;
                        }

                        let register = Register::from_index(register_idx as u32);
                        let register_value = self.read_register(register, |_| unreachable!());
                        self.bus.write_word_address(register_value, current_address);

                        if matches!(index_type, BlockDataTransferIndexType::PostIndex) {
                            current_address -= 4;
                        }
                    }
                }
            }
        }

        if write_back {
            self.write_register(current_address, base_register);
        }
    }

    fn execute_arm_mul(
        &mut self,
        operation: MultiplyOperation,
        set_conditions: bool,
        destination_register_rdhi: Register,
        accumulate_register_rdlo: Register,
        operand_register_rm: Register,
        operand_register_rs: Register,
    ) {
        let accumulate_rdlo_value =
            self.read_register(accumulate_register_rdlo, |_| unreachable!());
        let destination_rdhi_value = self.read_register(destination_register_rdhi, |_| todo!());
        let rm_value = self.read_register(operand_register_rm, |_| unreachable!());
        let rs_value = self.read_register(operand_register_rs, |_| unreachable!());

        match operation {
            MultiplyOperation::Mul => {
                let result = rm_value.wrapping_mul(rs_value);
                if set_conditions {
                    self.set_zero_flag(result == 0);
                    self.set_sign_flag((result as i32) < 0);
                }

                self.write_register(result, destination_register_rdhi);
            }
            MultiplyOperation::Mla => {
                let result = rm_value
                    .wrapping_mul(rs_value)
                    .wrapping_add(accumulate_rdlo_value);
                if set_conditions {
                    self.set_zero_flag(result == 0);
                    self.set_sign_flag((result as i32) < 0);
                }

                self.write_register(result, destination_register_rdhi);
            }
            MultiplyOperation::Umull => {
                let result = u64::from(rm_value).wrapping_mul(u64::from(rs_value));
                if set_conditions {
                    self.set_zero_flag(result == 0);
                    self.set_sign_flag((result as i64) < 0);
                }

                let low_word = result.get_data(0);
                let high_word = result.get_data(1);

                self.write_register(low_word, accumulate_register_rdlo);
                self.write_register(high_word, destination_register_rdhi);
            }
            MultiplyOperation::Umlal => {
                let accumulate_value =
                    u64::from(accumulate_rdlo_value) | (u64::from(destination_rdhi_value) << 32);
                let result = u64::from(rm_value)
                    .wrapping_mul(u64::from(rs_value))
                    .wrapping_add(accumulate_value);
                if set_conditions {
                    self.set_zero_flag(result == 0);
                    self.set_sign_flag((result as i64) < 0);
                }

                let low_word = result.get_data(0);
                let high_word = result.get_data(1);

                self.write_register(low_word, accumulate_register_rdlo);
                self.write_register(high_word, destination_register_rdhi);
            }
            MultiplyOperation::Smull => {
                let signed_result =
                    i64::from(rm_value as i32).wrapping_mul(i64::from(rs_value as i32));
                let result = signed_result as u64;

                if set_conditions {
                    self.set_zero_flag(result == 0);
                    self.set_sign_flag((result as i64) < 0);
                }

                let low_word = result.get_data(0);
                let high_word = result.get_data(1);

                self.write_register(low_word, accumulate_register_rdlo);
                self.write_register(high_word, destination_register_rdhi);
            }
            MultiplyOperation::Smlal => {
                let accumulate_value = (u64::from(accumulate_rdlo_value)
                    | (u64::from(destination_rdhi_value) << 32))
                    as i64;

                let signed_result = i64::from(rm_value as i32)
                    .wrapping_mul(i64::from(rs_value as i32))
                    .wrapping_add(accumulate_value);
                let result = signed_result as u64;

                if set_conditions {
                    self.set_zero_flag(result == 0);
                    self.set_sign_flag((result as i64) < 0);
                }

                let low_word = result.get_data(0);
                let high_word = result.get_data(1);

                self.write_register(low_word, accumulate_register_rdlo);
                self.write_register(high_word, destination_register_rdhi);
            }
            _ => todo!("multiply impl for {:?}", operation),
        }
    }
}

impl Cpu {
    fn execute_thumb_register_operation(
        &mut self,
        operation: ThumbRegisterOperation,
        destination_register: Register,
        source: Register,
        second_operand: ThumbRegisterOrImmediate,
    ) {
        let first_operand_value = self.read_register(source, |pc| pc + 2);
        let second_operand_value =
            self.evaluate_thumb_register_or_immedate(second_operand, |_| unreachable!());

        let (unsigned_result, carry_flag, signed_result, overflow_flag) = match operation {
            ThumbRegisterOperation::Add => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_add(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32);

                (unsigned_result, Some(carry), signed_result, Some(overflow))
            }
            ThumbRegisterOperation::Adc => {
                let (unsigned_result, carry) = if self.get_carry_flag() {
                    let (first_result, carry_1) =
                        first_operand_value.overflowing_add(second_operand_value);
                    let (final_result, carry_2) = first_result.overflowing_add(1);
                    (final_result, carry_1 | carry_2)
                } else {
                    first_operand_value.overflowing_add(second_operand_value)
                };

                let (signed_result, overflow) = if self.get_carry_flag() {
                    let (first_result, carry_1) =
                        (first_operand_value as i32).overflowing_add(second_operand_value as i32);
                    let (final_result, carry_2) = first_result.overflowing_add(1);
                    (final_result, carry_1 | carry_2)
                } else {
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32)
                };

                (unsigned_result, Some(carry), signed_result, Some(overflow))
            }
            ThumbRegisterOperation::Sub => {
                let (unsigned_result, borrow) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);
                (
                    unsigned_result,
                    Some(!borrow),
                    signed_result,
                    Some(overflow),
                )
            }
            ThumbRegisterOperation::Sbc => {
                let borrow_in = !self.get_carry_flag();

                let (unsigned_result, borrow) = if borrow_in {
                    let (result_1, borrow_1) =
                        first_operand_value.overflowing_sub(second_operand_value);
                    let (unsigned_result, borrow_2) = result_1.overflowing_sub(1);
                    (unsigned_result, borrow_1 | borrow_2)
                } else {
                    first_operand_value.overflowing_sub(second_operand_value)
                };

                let (signed_result, overflow) = if borrow_in {
                    let (result_1, overflow_1) =
                        (first_operand_value as i32).overflowing_sub(second_operand_value as i32);
                    let (signed_result, overflow_2) = result_1.overflowing_sub(1);
                    (signed_result, overflow_1 | overflow_2)
                } else {
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32)
                };

                (
                    unsigned_result,
                    Some(!borrow),
                    signed_result,
                    Some(overflow),
                )
            }
            ThumbRegisterOperation::Neg => {
                let (unsigned_result, borrow) = 0u32.overflowing_sub(second_operand_value);
                let (signed_result, overflow) = 0i32.overflowing_sub(second_operand_value as i32);
                (
                    unsigned_result,
                    Some(!borrow),
                    signed_result,
                    Some(overflow),
                )
            }
            ThumbRegisterOperation::Cmp => {
                let (unsigned_result, borrow) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);
                (
                    unsigned_result,
                    Some(!borrow),
                    signed_result,
                    Some(overflow),
                )
            }
            ThumbRegisterOperation::Cmn => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_add(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32);

                (unsigned_result, Some(carry), signed_result, Some(overflow))
            }
            ThumbRegisterOperation::Mov => (
                second_operand_value,
                None,
                second_operand_value as i32,
                None,
            ),
            ThumbRegisterOperation::Mvn => {
                let result = !second_operand_value;
                (result, None, result as i32, None)
            }
            ThumbRegisterOperation::Lsl => {
                let (result, carry_out) = match second_operand {
                    ThumbRegisterOrImmediate::Immediate(shift) => {
                        if second_operand_value == 0 {
                            (first_operand_value, self.get_carry_flag())
                        } else {
                            let result = ShiftType::Lsl.evaluate(first_operand_value, shift);
                            let carry = first_operand_value.get_bit((32 - shift) as usize);
                            (result, carry)
                        }
                    }
                    ThumbRegisterOrImmediate::Register(_) => {
                        let shift = second_operand_value;

                        if shift == 0 {
                            (first_operand_value, self.get_carry_flag())
                        } else if shift < 32 {
                            let result = ShiftType::Lsl.evaluate(first_operand_value, shift);
                            let carry = first_operand_value.get_bit((32 - shift) as usize);

                            (result, carry)
                        } else if shift == 32 {
                            let carry = first_operand_value.get_bit(0);
                            (0, carry)
                        } else {
                            (0, false)
                        }
                    }
                };

                (result, Some(carry_out), result as i32, None)
            }
            ThumbRegisterOperation::Lsr => {
                let (result, carry_out) = match second_operand {
                    ThumbRegisterOrImmediate::Immediate(shift) => {
                        if second_operand_value == 0 {
                            (0, first_operand_value.get_bit(31))
                        } else {
                            let result = ShiftType::Lsr.evaluate(first_operand_value, shift);
                            let carry = first_operand_value.get_bit((shift - 1) as usize);
                            (result, carry)
                        }
                    }
                    ThumbRegisterOrImmediate::Register(_) => {
                        let shift = second_operand_value;

                        if shift == 0 {
                            (first_operand_value, self.get_carry_flag())
                        } else if shift < 32 {
                            let result = ShiftType::Lsr.evaluate(first_operand_value, shift);
                            let carry = first_operand_value.get_bit((shift - 1) as usize);

                            (result, carry)
                        } else if shift == 32 {
                            let carry = first_operand_value.get_bit(31);
                            (0, carry)
                        } else {
                            (0, false)
                        }
                    }
                };

                (result, Some(carry_out), result as i32, None)
            }
            ThumbRegisterOperation::Asr => {
                let (result, carry_out) = match second_operand {
                    ThumbRegisterOrImmediate::Immediate(shift) => {
                        if second_operand_value == 0 {
                            let carry = first_operand_value.get_bit(31);
                            let result = if carry { !0 } else { 0 };

                            (result, carry)
                        } else {
                            let result = ShiftType::Asr.evaluate(first_operand_value, shift);
                            let carry = first_operand_value.get_bit((shift - 1) as usize);
                            (result, carry)
                        }
                    }
                    ThumbRegisterOrImmediate::Register(_) => {
                        let shift = second_operand_value;

                        if shift == 0 {
                            (first_operand_value, self.get_carry_flag())
                        } else if shift < 32 {
                            let result = ShiftType::Asr.evaluate(first_operand_value, shift);
                            let carry = first_operand_value.get_bit((shift - 1) as usize);

                            (result, carry)
                        } else {
                            let carry = first_operand_value.get_bit(31);
                            let result = if carry { !0 } else { 0 };

                            (result, carry)
                        }
                    }
                };

                (result, Some(carry_out), result as i32, None)
            }
            ThumbRegisterOperation::Ror => {
                let (result, carry_out) = match second_operand {
                    ThumbRegisterOrImmediate::Immediate(shift) => {
                        if second_operand_value == 0 {
                            let old_carry = self.get_carry_flag();
                            let new_carry = first_operand_value.get_bit(0);
                            let result = first_operand_value.rotate_right(1).set_bit(31, old_carry);

                            (result, new_carry)
                        } else {
                            let result = ShiftType::Ror.evaluate(first_operand_value, shift);
                            let carry = first_operand_value.get_bit((shift - 1) as usize);
                            (result, carry)
                        }
                    }
                    ThumbRegisterOrImmediate::Register(_) => {
                        let shift = second_operand_value;
                        let effective_shift = shift % 32;

                        if shift == 0 {
                            (first_operand_value, self.get_carry_flag())
                        } else if effective_shift == 0 {
                            let carry = first_operand_value.get_bit(31);

                            (first_operand_value, carry)
                        } else {
                            let result =
                                ShiftType::Ror.evaluate(first_operand_value, effective_shift);
                            let carry = first_operand_value.get_bit((effective_shift - 1) as usize);

                            (result, carry)
                        }
                    }
                };

                (result, Some(carry_out), result as i32, None)
            }
            ThumbRegisterOperation::Tst => {
                let result = first_operand_value & second_operand_value;
                (result, None, result as i32, None)
            }
            ThumbRegisterOperation::And => {
                let result = first_operand_value & second_operand_value;
                (result, None, result as i32, None)
            }
            ThumbRegisterOperation::Orr => {
                let result = first_operand_value | second_operand_value;
                (result, None, result as i32, None)
            }
            ThumbRegisterOperation::Eor => {
                let result = first_operand_value ^ second_operand_value;
                (result, None, result as i32, None)
            }
            ThumbRegisterOperation::Bic => {
                let result = first_operand_value & (!second_operand_value);
                (result, None, result as i32, None)
            }
            ThumbRegisterOperation::Mul => {
                let result = first_operand_value.wrapping_mul(second_operand_value);
                (result, None, result as i32, None)
            }
            _ => todo!("{:?}", operation),
        };

        if let Some(carry_flag) = carry_flag {
            self.set_carry_flag(carry_flag);
        }

        if let Some(overflow_flag) = overflow_flag {
            self.set_overflow_flag(overflow_flag);
        }

        self.set_sign_flag(signed_result < 0);
        self.set_zero_flag(unsigned_result == 0);

        if matches!(
            operation,
            ThumbRegisterOperation::Lsl
                | ThumbRegisterOperation::Lsr
                | ThumbRegisterOperation::Asr
                | ThumbRegisterOperation::Add
                | ThumbRegisterOperation::Sub
                | ThumbRegisterOperation::Mov
                | ThumbRegisterOperation::And
                | ThumbRegisterOperation::Eor
                | ThumbRegisterOperation::Adc
                | ThumbRegisterOperation::Sbc
                | ThumbRegisterOperation::Ror
                | ThumbRegisterOperation::Tst
                | ThumbRegisterOperation::Neg
                | ThumbRegisterOperation::Orr
                | ThumbRegisterOperation::Mul
                | ThumbRegisterOperation::Bic
                | ThumbRegisterOperation::Mvn
        ) {
            self.write_register(unsigned_result, destination_register);
        }
    }

    fn execute_thumb_high_register_operation(
        &mut self,
        destination_register: Register,
        operation: ThumbHighRegisterOperation,
        source: Register,
    ) {
        let destination_register_value = self.read_register(destination_register, |pc| pc + 2);
        let source_value = self.read_register(source, |pc| pc + 2);
        match operation {
            ThumbHighRegisterOperation::Add => {
                let result = destination_register_value.wrapping_add(source_value);
                self.write_register(result, destination_register);
            }
            ThumbHighRegisterOperation::Cmp => {
                // cmp is only high register operation that sets flags, but it doesn't write value out to destination register.
                let (unsigned_result, borrow) =
                    destination_register_value.overflowing_sub(source_value);
                let (signed_result, overflow) =
                    (destination_register_value as i32).overflowing_sub(source_value as i32);

                self.set_sign_flag(signed_result < 0);
                self.set_zero_flag(unsigned_result == 0);
                self.set_carry_flag(!borrow);
                self.set_overflow_flag(overflow);
            }
            ThumbHighRegisterOperation::Mov => {
                self.write_register(source_value, destination_register)
            }
        }
    }

    fn execute_thumb_ldr(
        &mut self,
        base_register: Register,
        offset: ThumbRegisterOrImmediate,
        destination_register: Register,
        size: ThumbLoadStoreDataSize,
        sign_extend: bool,
    ) {
        let base_address = self.read_register(base_register, |pc| (pc + 2) & (!2));
        let base_offset = match offset {
            ThumbRegisterOrImmediate::Immediate(immediate) => immediate,
            ThumbRegisterOrImmediate::Register(register) => {
                self.read_register(register, |_| unreachable!())
            }
        };

        let real_address = base_address + base_offset;

        let result_value = match (size, sign_extend) {
            (ThumbLoadStoreDataSize::Byte, false) => {
                u32::from(self.bus.read_byte_address(real_address))
            }
            (ThumbLoadStoreDataSize::Byte, true) => {
                self.bus.read_byte_address(real_address) as i8 as i32 as u32
            }
            (ThumbLoadStoreDataSize::HalfWord, false) => {
                u32::from(self.bus.read_halfword_address(real_address))
            }
            (ThumbLoadStoreDataSize::HalfWord, true) => {
                self.bus.read_halfword_address(real_address) as i16 as i32 as u32
            }
            (ThumbLoadStoreDataSize::Word, false) => self.bus.read_word_address(real_address),
            _ => unreachable!(),
        };

        self.write_register(result_value, destination_register);
    }

    fn execute_thumb_str(
        &mut self,
        base_register: Register,
        offset: ThumbRegisterOrImmediate,
        source_register: Register,
        size: ThumbLoadStoreDataSize,
    ) {
        let base_address = self.read_register(base_register, |_| unreachable!());
        let base_offset = match offset {
            ThumbRegisterOrImmediate::Immediate(immediate) => immediate,
            ThumbRegisterOrImmediate::Register(register) => {
                self.read_register(register, |_| unreachable!())
            }
        };

        let real_address = base_address.wrapping_add(base_offset);
        let source_register_value = self.read_register(source_register, |_| unreachable!());

        match size {
            ThumbLoadStoreDataSize::Byte => self
                .bus
                .write_byte_address(source_register_value as u8, real_address),
            ThumbLoadStoreDataSize::HalfWord => self
                .bus
                .write_halfword_address(source_register_value as u16, real_address),
            ThumbLoadStoreDataSize::Word => self
                .bus
                .write_word_address(source_register_value, real_address),
        }
    }

    fn execute_thumb_b(&mut self, condition: InstructionCondition, offset: i16) {
        if self.evaluate_instruction_condition(condition) {
            let old_pc = self.read_register(Register::R15, |pc| pc);
            let new_pc = old_pc.wrapping_add(offset as u32).wrapping_add(2);
            self.write_register(new_pc, Register::R15);
        }
    }

    // LR = PC + 4 + offset
    fn execute_thumb_bl_part_1(&mut self, offset: i32) {
        let old_pc = self.read_register(Register::R15, |pc| pc);
        let new_lr = old_pc.wrapping_add(offset as u32).wrapping_add(2);

        self.write_register(new_lr, Register::R14);
    }

    // PC = LR + (nn SHL 1), and LR = PC+2 OR 1
    fn execute_thumb_bl_part_2(&mut self, offset: u16) {
        let new_pc = self
            .read_register(Register::R14, |_| unreachable!())
            .wrapping_add(u32::from(offset));
        let new_lr = self.read_register(Register::R15, |pc| pc) | 1;

        self.write_register(new_pc, Register::R15);
        self.write_register(new_lr, Register::R14);
    }

    fn execute_thumb_bx(&mut self, operand: Register) {
        const NEW_STATE_BIT_INDEX: usize = 0;

        // "BX R15: CPU switches to ARM state, and PC is auto-aligned as (($+4) AND NOT 2)."
        //
        // Ensure bit 0 is also cleared to make sure we switch to ARM state.
        let operand_value = self.read_register(operand, |pc| (pc + 2) & (!2) & (!1));

        let new_state_bit = operand_value.get_bit(NEW_STATE_BIT_INDEX);
        self.set_cpu_state_bit(new_state_bit);

        let new_pc = operand_value & (!1);

        self.write_register(new_pc, Register::R15);
    }

    fn execute_thumb_push(&mut self, register_bit_list: [bool; 8], push_lr: bool) {
        // Lowest register index goes at lowest address. As this is equivalent to STMDB, lowest register index needs to be considered last.
        //  In order to achieve this, iterate in reverse order.
        if push_lr {
            let lr_value = self.read_register(Register::R14, |_| unreachable!());

            let new_r13 = self.read_register(Register::R13, |_| unreachable!()) - 4;
            self.write_register(new_r13, Register::R13);
            self.bus.write_word_address(lr_value, new_r13);
        }

        for (register_idx, register_pushed) in register_bit_list.into_iter().enumerate().rev() {
            if register_pushed {
                let pushed_register = Register::from_index(register_idx as u32);
                let pushed_register_value = self.read_register(pushed_register, |_| unreachable!());

                let new_r13 = self.read_register(Register::R13, |_| unreachable!()) - 4;
                self.write_register(new_r13, Register::R13);
                self.bus.write_word_address(pushed_register_value, new_r13);
            }
        }
    }

    fn execute_thumb_pop(&mut self, register_bit_list: [bool; 8], pop_pc: bool) {
        for (register_idx, register_popped) in register_bit_list.into_iter().enumerate() {
            if register_popped {
                let popped_register = Register::from_index(register_idx as u32);
                let old_r13 = self.read_register(Register::R13, |_| unreachable!());
                let popped_register_value = self.bus.read_word_address(old_r13);

                self.write_register(old_r13 + 4, Register::R13);
                self.write_register(popped_register_value, popped_register);
            }
        }

        if pop_pc {
            // POP {PC} ignores the least significant bit of the return address (processor remains in thumb state even if bit0 was cleared).
            let old_r13 = self.read_register(Register::R13, |_| unreachable!());
            let pc_value = self.bus.read_word_address(old_r13) & (!1);

            self.write_register(old_r13 + 4, Register::R13);
            self.write_register(pc_value, Register::R15);
        }
    }

    fn execute_thumb_stmia_write_back(
        &mut self,
        base_register: Register,
        register_bit_list: [bool; 8],
    ) {
        let mut current_write_address = self.read_register(base_register, |_| todo!());

        for (register_idx, register_stored) in register_bit_list.into_iter().enumerate() {
            if register_stored {
                let stored_register = Register::from_index(register_idx as u32);
                let stored_register_value = self.read_register(stored_register, |_| todo!());

                self.bus
                    .write_word_address(stored_register_value, current_write_address);

                current_write_address += 4;
            }
        }

        self.write_register(current_write_address, base_register);
    }

    fn execute_thumb_ldmia_write_back(
        &mut self,
        base_register: Register,
        register_bit_list: [bool; 8],
    ) {
        let mut current_read_address = self.read_register(base_register, |_| todo!());

        for (register_idx, register_loaded) in register_bit_list.into_iter().enumerate() {
            if register_loaded {
                let loaded_register = Register::from_index(register_idx as u32);
                let loaded_value = self.bus.read_word_address(current_read_address);

                self.write_register(loaded_value, loaded_register);

                current_read_address += 4;
            }
        }

        self.write_register(current_read_address, base_register);
    }

    fn execute_thumb_add_special(
        &mut self,
        source_register: Register,
        dest_register: Register,
        sign_bit: bool,
        unsigned_offset: u16,
    ) {
        // (when reading PC): "Rd = (($+4) AND NOT 2) + nn"
        let source_register_value = self.read_register(source_register, |pc| (pc + 2) & (!2));

        let result_value = if sign_bit {
            source_register_value - u32::from(unsigned_offset)
        } else {
            source_register_value + u32::from(unsigned_offset)
        };

        self.write_register(result_value, dest_register);
    }
}

impl Cpu {
    fn evaluate_instruction_condition(&self, condition: InstructionCondition) -> bool {
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
            InstructionCondition::Always => true,
            InstructionCondition::Never => false,
        }
    }

    fn evaluate_alu_second_operand(&self, info: AluSecondOperandInfo) -> (u32, bool) {
        match info {
            AluSecondOperandInfo::Immediate { base, shift } => {
                let result = base.rotate_right(shift);
                let shift_out = if shift == 0 {
                    self.get_carry_flag()
                } else {
                    result.get_bit(31)
                };

                (result, shift_out)
            }
            AluSecondOperandInfo::Register {
                register,
                shift_info,
                shift_type,
            } => {
                match shift_info {
                    ArmRegisterOrImmediate::Immediate(shift) => {
                        // When using R15 as operand (Rm or Rn), the returned value depends on the instruction:
                        //   - PC+12 if I=0,R=1 (shift by register)
                        //   - otherwise, PC+8 (shift by immediate).
                        //
                        // The first case is always present here.
                        //
                        // When shifting by register, only lower 8bit 0-255 used.
                        let register_value = self.read_register(register, |pc| pc + 4);

                        if shift == 0 {
                            match shift_type {
                                ShiftType::Lsl => (register_value, self.get_carry_flag()),
                                ShiftType::Lsr => (0, register_value.get_bit(31)),
                                ShiftType::Asr => {
                                    let carry = register_value.get_bit(31);
                                    let result = if carry { !0 } else { 0 };

                                    (result, carry)
                                }
                                ShiftType::Ror => {
                                    let old_carry = self.get_carry_flag();
                                    let new_carry = register_value.get_bit(0);
                                    let result =
                                        register_value.rotate_right(1).set_bit(31, old_carry);

                                    (result, new_carry)
                                }
                            }
                        } else {
                            let result = shift_type.evaluate(register_value, shift);
                            let carry = match shift_type {
                                ShiftType::Lsl => register_value.get_bit((32 - shift) as usize),
                                ShiftType::Lsr => register_value.get_bit((shift - 1) as usize),
                                ShiftType::Asr => register_value.get_bit((shift - 1) as usize),
                                ShiftType::Ror => register_value.get_bit((shift - 1) as usize),
                            };

                            (result, carry)
                        }
                    }
                    ArmRegisterOrImmediate::Register(shift_register) => {
                        // When using R15 as operand (Rm or Rn), the returned value depends on the instruction:
                        //   - PC+12 if I=0,R=1 (shift by register)
                        //   - otherwise, PC+8 (shift by immediate).
                        //
                        // The first case is always present here.
                        //
                        // When shifting by register, only lower 8bit 0-255 used.
                        let register_value = self.read_register(register, |pc| pc + 8);
                        let shift_amount = self.read_register(shift_register, |pc| pc) & 0xFF;

                        match shift_type {
                            ShiftType::Lsl => {
                                if shift_amount == 0 {
                                    (register_value, self.get_carry_flag())
                                } else if shift_amount < 32 {
                                    let result =
                                        ShiftType::Lsl.evaluate(register_value, shift_amount);
                                    let carry =
                                        register_value.get_bit((32 - shift_amount) as usize);
                                    (result, carry)
                                } else if shift_amount == 32 {
                                    let carry = register_value.get_bit(0);
                                    (0, carry)
                                } else {
                                    (0, false)
                                }
                            }
                            ShiftType::Lsr => {
                                if shift_amount == 0 {
                                    (register_value, self.get_carry_flag())
                                } else if shift_amount < 32 {
                                    let result =
                                        ShiftType::Lsr.evaluate(register_value, shift_amount);
                                    let carry = register_value.get_bit((shift_amount - 1) as usize);

                                    (result, carry)
                                } else if shift_amount == 32 {
                                    let carry = register_value.get_bit(31);
                                    (0, carry)
                                } else {
                                    (0, false)
                                }
                            }
                            ShiftType::Asr => {
                                if shift_amount == 0 {
                                    (register_value, self.get_carry_flag())
                                } else if shift_amount < 32 {
                                    let result =
                                        ShiftType::Asr.evaluate(register_value, shift_amount);
                                    let carry = register_value.get_bit((shift_amount - 1) as usize);
                                    (result, carry)
                                } else {
                                    let carry = register_value.get_bit(31);
                                    let result = if carry { !0 } else { 0 };
                                    (result, carry)
                                }
                            }
                            ShiftType::Ror => {
                                let effective_shift = shift_amount % 32;
                                if shift_amount == 0 {
                                    (register_value, self.get_carry_flag())
                                } else if effective_shift == 0 {
                                    let carry = register_value.get_bit(31);
                                    (register_value, carry)
                                } else {
                                    let result =
                                        ShiftType::Ror.evaluate(register_value, effective_shift);
                                    let carry =
                                        register_value.get_bit((effective_shift - 1) as usize);
                                    (result, carry)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn evaluate_thumb_register_or_immedate(
        &self,
        value: ThumbRegisterOrImmediate,
        pc_calculation: fn(u32) -> u32,
    ) -> u32 {
        match value {
            ThumbRegisterOrImmediate::Immediate(immediate) => immediate,
            ThumbRegisterOrImmediate::Register(register) => {
                self.read_register(register, pc_calculation)
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
