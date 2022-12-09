mod arm;
mod thumb;

use std::cell::Cell;

use std::fmt::Display;
use std::rc::Rc;
use std::{fmt::Debug, ops::RangeInclusive};

use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::BitManipulation;

use self::arm::ArmInstruction;
use self::thumb::ThumbInstruction;

struct ModeRegisters {
    r0: Rc<Cell<u32>>,
    r1: Rc<Cell<u32>>,
    r2: Rc<Cell<u32>>,
    r3: Rc<Cell<u32>>,
    r4: Rc<Cell<u32>>,
    r5: Rc<Cell<u32>>,
    r6: Rc<Cell<u32>>,
    r7: Rc<Cell<u32>>,
    r8: Rc<Cell<u32>>,
    r9: Rc<Cell<u32>>,
    r10: Rc<Cell<u32>>,
    r11: Rc<Cell<u32>>,
    r12: Rc<Cell<u32>>,
    r13: Rc<Cell<u32>>, // SP
    r14: Rc<Cell<u32>>, // LR
    r15: Rc<Cell<u32>>, // PC
    spsr: Rc<Cell<u32>>,
}

pub struct Cpu {
    user_registers: ModeRegisters,
    fiq_registers: ModeRegisters,
    svc_registers: ModeRegisters,
    abt_registers: ModeRegisters,
    irq_registers: ModeRegisters,
    und_registers: ModeRegisters,
    cpsr: Rc<Cell<u32>>,
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
        let r0 = Rc::default();
        let r1 = Rc::default();
        let r2 = Rc::default();
        let r3 = Rc::default();
        let r4 = Rc::default();
        let r5 = Rc::default();
        let r6 = Rc::default();
        let r7 = Rc::default();
        let r8 = Rc::default();
        let r9 = Rc::default();
        let r10 = Rc::default();
        let r11 = Rc::default();
        let r12 = Rc::default();
        let r13 = Rc::default();
        let r14 = Rc::default();
        let r15 = Rc::default();

        let r8_fiq = Rc::default();
        let r9_fiq = Rc::default();
        let r10_fiq = Rc::default();
        let r11_fiq = Rc::default();
        let r12_fiq = Rc::default();
        let r13_fiq = Rc::default();
        let r14_fiq = Rc::default();

        let r13_svc = Rc::default();
        let r14_svc = Rc::default();

        let r13_abt = Rc::default();
        let r14_abt = Rc::default();

        let r13_irq = Rc::default();
        let r14_irq = Rc::default();

        let r13_und = Rc::default();
        let r14_und = Rc::default();

        // treated as SPSR in system and user mode
        let cpsr = Rc::new(Cell::new(Self::SYSTEM_MODE_BITS));

        let user_registers = ModeRegisters {
            r0: Rc::clone(&r0),
            r1: Rc::clone(&r1),
            r2: Rc::clone(&r2),
            r3: Rc::clone(&r3),
            r4: Rc::clone(&r4),
            r5: Rc::clone(&r5),
            r6: Rc::clone(&r6),
            r7: Rc::clone(&r7),
            r8: Rc::clone(&r8),
            r9: Rc::clone(&r9),
            r10: Rc::clone(&r10),
            r11: Rc::clone(&r11),
            r12: Rc::clone(&r12),
            r13: Rc::clone(&r13),
            r14: Rc::clone(&r14),
            r15: Rc::clone(&r15),
            spsr: Rc::clone(&cpsr),
        };

        let fiq_registers = ModeRegisters {
            r0: Rc::clone(&r0),
            r1: Rc::clone(&r1),
            r2: Rc::clone(&r2),
            r3: Rc::clone(&r3),
            r4: Rc::clone(&r4),
            r5: Rc::clone(&r5),
            r6: Rc::clone(&r6),
            r7: Rc::clone(&r7),
            r8: Rc::clone(&r8_fiq),
            r9: Rc::clone(&r9_fiq),
            r10: Rc::clone(&r10_fiq),
            r11: Rc::clone(&r11_fiq),
            r12: Rc::clone(&r12_fiq),
            r13: Rc::clone(&r13_fiq),
            r14: Rc::clone(&r14_fiq),
            r15: Rc::clone(&r15),
            spsr: Rc::default(),
        };

        let svc_registers = ModeRegisters {
            r0: Rc::clone(&r0),
            r1: Rc::clone(&r1),
            r2: Rc::clone(&r2),
            r3: Rc::clone(&r3),
            r4: Rc::clone(&r4),
            r5: Rc::clone(&r5),
            r6: Rc::clone(&r6),
            r7: Rc::clone(&r7),
            r8: Rc::clone(&r8),
            r9: Rc::clone(&r9),
            r10: Rc::clone(&r10),
            r11: Rc::clone(&r11),
            r12: Rc::clone(&r12),
            r13: Rc::clone(&r13_svc),
            r14: Rc::clone(&r14_svc),
            r15: Rc::clone(&r15),
            spsr: Rc::default(),
        };

        let abt_registers = ModeRegisters {
            r0: Rc::clone(&r0),
            r1: Rc::clone(&r1),
            r2: Rc::clone(&r2),
            r3: Rc::clone(&r3),
            r4: Rc::clone(&r4),
            r5: Rc::clone(&r5),
            r6: Rc::clone(&r6),
            r7: Rc::clone(&r7),
            r8: Rc::clone(&r8),
            r9: Rc::clone(&r9),
            r10: Rc::clone(&r10),
            r11: Rc::clone(&r11),
            r12: Rc::clone(&r12),
            r13: Rc::clone(&r13_abt),
            r14: Rc::clone(&r14_abt),
            r15: Rc::clone(&r15),
            spsr: Rc::default(),
        };

        let irq_registers = ModeRegisters {
            r0: Rc::clone(&r0),
            r1: Rc::clone(&r1),
            r2: Rc::clone(&r2),
            r3: Rc::clone(&r3),
            r4: Rc::clone(&r4),
            r5: Rc::clone(&r5),
            r6: Rc::clone(&r6),
            r7: Rc::clone(&r7),
            r8: Rc::clone(&r8),
            r9: Rc::clone(&r9),
            r10: Rc::clone(&r10),
            r11: Rc::clone(&r11),
            r12: Rc::clone(&r12),
            r13: Rc::clone(&r13_irq),
            r14: Rc::clone(&r14_irq),
            r15: Rc::clone(&r15),
            spsr: Rc::default(),
        };

        let und_registers = ModeRegisters {
            r0: Rc::clone(&r0),
            r1: Rc::clone(&r1),
            r2: Rc::clone(&r2),
            r3: Rc::clone(&r3),
            r4: Rc::clone(&r4),
            r5: Rc::clone(&r5),
            r6: Rc::clone(&r6),
            r7: Rc::clone(&r7),
            r8: Rc::clone(&r8),
            r9: Rc::clone(&r9),
            r10: Rc::clone(&r10),
            r11: Rc::clone(&r11),
            r12: Rc::clone(&r12),
            r13: Rc::clone(&r13_und),
            r14: Rc::clone(&r14_und),
            r15: Rc::clone(&r15),
            spsr: Rc::default(),
        };

        Self {
            user_registers,
            fiq_registers,
            svc_registers,
            abt_registers,
            irq_registers,
            und_registers,
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
        let registers = match self.get_cpu_mode() {
            CpuMode::User | CpuMode::System => &self.user_registers,
            CpuMode::Fiq => &self.fiq_registers,
            CpuMode::Supervisor => &self.svc_registers,
            CpuMode::Abort => &self.abt_registers,
            CpuMode::Irq => &self.irq_registers,
            CpuMode::Undefined => &self.und_registers,
        };

        match register {
            Register::R0 => registers.r0.set(value),
            Register::R1 => registers.r1.set(value),
            Register::R2 => registers.r2.set(value),
            Register::R3 => registers.r3.set(value),
            Register::R4 => registers.r4.set(value),
            Register::R5 => registers.r5.set(value),
            Register::R6 => registers.r6.set(value),
            Register::R7 => registers.r7.set(value),
            Register::R8 => registers.r8.set(value),
            Register::R9 => registers.r9.set(value),
            Register::R10 => registers.r10.set(value),
            Register::R11 => registers.r11.set(value),
            Register::R12 => registers.r12.set(value),
            Register::R13 => registers.r13.set(value),
            Register::R14 => registers.r14.set(value),
            Register::R15 => {
                if self.get_cpu_state_bit() {
                    if value & 0b1 != 0 {
                        log::warn!(
                            "writing to Thumb PC with unaligned value: 0x{:08X}, force aligning",
                            value
                        );
                    }
                    registers.r15.set(value & !0b1);
                } else {
                    if value & 0b11 != 0 {
                        log::warn!(
                            "writing to ARM PC with unaligned value: 0x{:08X}, force aligning",
                            value
                        );
                    }
                    registers.r15.set(value & !0b11);
                }
            }
            Register::Spsr => registers.spsr.set(value),
            Register::Cpsr => self.cpsr.set(value),
        }
    }

    fn read_register(&self, register: Register, pc_calculation: fn(u32) -> u32) -> u32 {
        let registers = match self.get_cpu_mode() {
            CpuMode::User | CpuMode::System => &self.user_registers,
            CpuMode::Fiq => &self.fiq_registers,
            CpuMode::Supervisor => &self.svc_registers,
            CpuMode::Abort => &self.abt_registers,
            CpuMode::Irq => &self.irq_registers,
            CpuMode::Undefined => &self.und_registers,
        };

        match register {
            Register::R0 => registers.r0.get(),
            Register::R1 => registers.r1.get(),
            Register::R2 => registers.r2.get(),
            Register::R3 => registers.r3.get(),
            Register::R4 => registers.r4.get(),
            Register::R5 => registers.r5.get(),
            Register::R6 => registers.r6.get(),
            Register::R7 => registers.r7.get(),
            Register::R8 => registers.r8.get(),
            Register::R9 => registers.r9.get(),
            Register::R10 => registers.r10.get(),
            Register::R11 => registers.r11.get(),
            Register::R12 => registers.r12.get(),
            Register::R13 => registers.r13.get(),
            Register::R14 => registers.r14.get(),
            Register::R15 => pc_calculation(registers.r15.get()),
            Register::Spsr => registers.spsr.get(),
            Register::Cpsr => self.cpsr.get(),
        }
    }

    fn read_user_register(&self, register: Register, pc_calculation: fn(u32) -> u32) -> u32 {
        match register {
            Register::R0 => self.user_registers.r0.get(),
            Register::R1 => self.user_registers.r1.get(),
            Register::R2 => self.user_registers.r2.get(),
            Register::R3 => self.user_registers.r3.get(),
            Register::R4 => self.user_registers.r4.get(),
            Register::R5 => self.user_registers.r5.get(),
            Register::R6 => self.user_registers.r6.get(),
            Register::R7 => self.user_registers.r7.get(),
            Register::R8 => self.user_registers.r8.get(),
            Register::R9 => self.user_registers.r9.get(),
            Register::R10 => self.user_registers.r10.get(),
            Register::R11 => self.user_registers.r11.get(),
            Register::R12 => self.user_registers.r12.get(),
            Register::R13 => self.user_registers.r13.get(),
            Register::R14 => self.user_registers.r14.get(),
            Register::R15 => pc_calculation(self.user_registers.r15.get()),
            Register::Spsr => self.user_registers.spsr.get(),
            Register::Cpsr => self.cpsr.get(),
        }
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

        let new_registers = match new_mode {
            CpuMode::Abort => &mut self.abt_registers,
            CpuMode::Fiq => &mut self.fiq_registers,
            CpuMode::Irq => &mut self.irq_registers,
            CpuMode::Supervisor => &mut self.svc_registers,
            CpuMode::Undefined => &mut self.und_registers,
            _ => unreachable!(),
        };

        // save old pc in new mode lr
        new_registers.r14.set(old_pc);

        // save old cpsr in new mode spsr
        new_registers.spsr.set(old_flags);

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

    fn get_sign_flag(&self) -> bool {
        self.cpsr.get().get_bit(Self::SIGN_FLAG_BIT_INDEX)
    }

    fn set_sign_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.get().set_bit(Self::SIGN_FLAG_BIT_INDEX, set);
        self.cpsr.set(new_cpsr);
    }

    fn get_zero_flag(&self) -> bool {
        self.cpsr.get().get_bit(Self::ZERO_FLAG_BIT_INDEX)
    }

    fn set_zero_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.get().set_bit(Self::ZERO_FLAG_BIT_INDEX, set);
        self.cpsr.set(new_cpsr);
    }

    fn get_carry_flag(&self) -> bool {
        self.cpsr.get().get_bit(Self::CARRY_FLAG_BIT_INDEX)
    }

    fn set_carry_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.get().set_bit(Self::CARRY_FLAG_BIT_INDEX, set);
        self.cpsr.set(new_cpsr);
    }

    fn get_overflow_flag(&self) -> bool {
        self.cpsr.get().get_bit(Self::OVERFLOW_FLAG_BIT_INDEX)
    }

    fn set_overflow_flag(&mut self, set: bool) {
        let new_cpsr = self.cpsr.get().set_bit(Self::OVERFLOW_FLAG_BIT_INDEX, set);
        self.cpsr.set(new_cpsr);
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
        self.cpsr.get().get_bit(Self::IRQ_DISABLE_BIT_OFFSET)
    }

    fn set_irq_disable(&mut self, set: bool) {
        let new_cpsr = self.cpsr.get().set_bit(Self::IRQ_DISABLE_BIT_OFFSET, set);
        self.cpsr.set(new_cpsr);
    }

    fn get_fiq_disable(&self) -> bool {
        self.cpsr.get().get_bit(Self::FIQ_DISABLE_BIT_OFFSET)
    }

    fn set_fiq_disable(&mut self, set: bool) {
        let new_cpsr = self.cpsr.get().set_bit(Self::FIQ_DISABLE_BIT_OFFSET, set);
        self.cpsr.set(new_cpsr);
    }

    fn get_instruction_mode(&self) -> InstructionSet {
        if self.get_cpu_state_bit() {
            InstructionSet::Thumb
        } else {
            InstructionSet::Arm
        }
    }

    fn get_cpu_state_bit(&self) -> bool {
        self.cpsr.get().get_bit(Self::STATE_BIT_OFFSET)
    }

    fn set_cpu_state_bit(&mut self, set: bool) {
        let new_cpsr = self.cpsr.get().set_bit(Self::STATE_BIT_OFFSET, set);
        self.cpsr.set(new_cpsr);
    }

    fn get_cpu_mode(&self) -> CpuMode {
        match self.cpsr.get().get_bit_range(Self::MODE_BITS_RANGE) {
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
            .cpsr
            .get()
            .set_bit_range(new_mode_bits, Self::MODE_BITS_RANGE);
        self.cpsr.set(new_cpsr);
    }
}
