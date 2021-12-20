use std::fmt::Display;
use std::{fmt::Debug, ops::RangeInclusive};

use crate::bit_manipulation::BitManipulation;
use crate::bus::Bus;

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
    bus: Bus,
}

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            " R0: 0x{:08x}  R1: 0x{:08x}  R2: 0x{:08x}  R3: 0x{:08x}",
            self.r0, self.r1, self.r2, self.r3
        )?;
        writeln!(
            f,
            " R4: 0x{:08x}  R5: 0x{:08x}  R6: 0x{:08x}  R7: 0x{:08x}",
            self.r4, self.r5, self.r6, self.r7
        )?;
        writeln!(
            f,
            " R8: 0x{:08x}  R9: 0x{:08x} R10: 0x{:08x} R11: 0x{:08x}",
            self.r8, self.r9, self.r10, self.r11
        )?;
        write!(
            f,
            "R12: 0x{:08x} R13: 0x{:08x} R14: 0x{:08x} R15: 0x{:08x}",
            self.r12, self.r13, self.r14, self.r15
        )?;

        Ok(())
    }
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
            cpsr: Self::SUPERVISOR_MODE_BITS,
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
            Self::SignedGreaterThan => f.write_str("g"),
            Self::SignedLessOrEqual => f.write_str("le"),
            Self::Always => Ok(()),
            Self::Never => unreachable!("never branch condition"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum OffsetModifierType {
    AddToBase,
    SubtractFromBase,
}

impl Display for OffsetModifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OffsetModifierType::AddToBase => f.write_str("+"),
            OffsetModifierType::SubtractFromBase => f.write_str("-"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SingleDataMemoryAccessSize {
    Byte,
    HalfWord,
    Word,
    DoubleWord,
}

#[derive(Clone, Copy, Debug)]
pub enum ArmInstructionType {
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
        destination_rdhi_register: Register,
        accumulate_rdlo_register: Register,
        operand_register: Register,
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
        source: ThumbRegisterOrImmediate,
        second_operand: Option<ThumbRegisterOrImmediate>,
    },
    B {
        condition: InstructionCondition,
        offset: i16,
    },
    Bl {
        offset: i32,
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

impl Display for ThumbRegisterOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThumbRegisterOperation::Lsl => f.write_str("lsl"),
            ThumbRegisterOperation::Lsr => f.write_str("lsr"),
            ThumbRegisterOperation::Asr => f.write_str("asr"),
            ThumbRegisterOperation::Add => f.write_str("add"),
            ThumbRegisterOperation::Sub => f.write_str("sub"),
            ThumbRegisterOperation::Mov => f.write_str("mov"),
            ThumbRegisterOperation::Cmp => f.write_str("cmp"),
            ThumbRegisterOperation::And => f.write_str("and"),
            ThumbRegisterOperation::Eor => f.write_str("eor"),
            ThumbRegisterOperation::Adc => f.write_str("adc"),
            ThumbRegisterOperation::Sbc => f.write_str("sbc"),
            ThumbRegisterOperation::Ror => f.write_str("ror"),
            ThumbRegisterOperation::Tst => f.write_str("tst"),
            ThumbRegisterOperation::Neg => f.write_str("neg"),
            ThumbRegisterOperation::Cmn => f.write_str("cmn"),
            ThumbRegisterOperation::Orr => f.write_str("orr"),
            ThumbRegisterOperation::Mul => f.write_str("mul"),
            ThumbRegisterOperation::Bic => f.write_str("bic"),
            ThumbRegisterOperation::Mvn => f.write_str("mvn"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbRegisterOrImmediate {
    Immediate(u32),
    Register(Register),
}

impl Display for ThumbRegisterOrImmediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThumbRegisterOrImmediate::Immediate(value) => write!(f, "#{}", value),
            ThumbRegisterOrImmediate::Register(register) => write!(f, "{}", register),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbLoadStoreDataSize {
    Byte,
    HalfWord,
    Word,
}

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    ArmInstruction(ArmInstruction),
    ThumbInstruction(ThumbInstruction),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::ArmInstruction(instruction) => {
                write!(f, "ARM   0x{:08x}: {}", instruction.address, instruction)
            }
            Instruction::ThumbInstruction(instruction) => {
                write!(f, "Thumb 0x{:08x}: {}", instruction.address, instruction)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ArmInstruction {
    instruction_type: ArmInstructionType,
    condition: InstructionCondition,
    address: u32,
}

impl Display for ArmInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.instruction_type {
            ArmInstructionType::Alu {
                operation,
                set_conditions: _,
                first_operand,
                second_operand,
                destination_operand,
            } => match operation {
                AluOperation::And
                | AluOperation::Eor
                | AluOperation::Sub
                | AluOperation::Rsb
                | AluOperation::Add
                | AluOperation::Adc
                | AluOperation::Sbc
                | AluOperation::Rsc
                | AluOperation::Orr
                | AluOperation::Bic => write!(
                    f,
                    "{}{} {}, {}, {}",
                    operation, self.condition, destination_operand, first_operand, second_operand
                ),
                AluOperation::Tst | AluOperation::Teq | AluOperation::Cmp | AluOperation::Cmn => {
                    write!(
                        f,
                        "{}{} {}, {}",
                        operation, self.condition, first_operand, second_operand
                    )
                }
                AluOperation::Mov | AluOperation::Mvn => write!(
                    f,
                    "{}{} {}, {}",
                    operation, self.condition, destination_operand, second_operand
                ),
                _ => todo!(),
            },
            ArmInstructionType::B { offset } => write!(f, "b{} 0x{:08X}", self.condition, offset),
            ArmInstructionType::Bl { offset } => write!(f, "bl{} 0x{:08X}", self.condition, offset),
            ArmInstructionType::Bx { operand } => write!(f, "bx{} {}", self.condition, operand),
            ArmInstructionType::Ldr {
                access_size,
                base_register,
                destination_register,
                index_type,
                offset_info,
            } => {
                write!(f, "ldr{}", self.condition)?;
                if offset_info.sign {
                    f.write_str("s")?;
                }

                match access_size {
                    SingleDataMemoryAccessSize::Byte => f.write_str("b")?,
                    SingleDataMemoryAccessSize::HalfWord => f.write_str("h")?,
                    SingleDataMemoryAccessSize::Word => {}
                    SingleDataMemoryAccessSize::DoubleWord => f.write_str("d")?,
                };

                write!(f, " {}, ", destination_register)?;

                match index_type {
                    SingleDataTransferIndexType::PreIndex { write_back } => {
                        write!(f, "[{}, {}]", base_register, offset_info)?;
                        if write_back {
                            f.write_str("!")?;
                        }
                    }
                    SingleDataTransferIndexType::PostIndex { .. } => {
                        write!(f, "[{}], {}", base_register, offset_info)?
                    }
                }

                Ok(())
            }
            ArmInstructionType::Str {
                access_size,
                base_register,
                source_register,
                index_type,
                offset_info,
            } => {
                write!(f, "str{}", self.condition)?;
                if offset_info.sign {
                    f.write_str("s")?;
                }

                match access_size {
                    SingleDataMemoryAccessSize::Byte => f.write_str("b")?,
                    SingleDataMemoryAccessSize::HalfWord => f.write_str("h")?,
                    SingleDataMemoryAccessSize::Word => {}
                    SingleDataMemoryAccessSize::DoubleWord => f.write_str("d")?,
                };

                write!(f, " {}, ", source_register)?;

                match index_type {
                    SingleDataTransferIndexType::PreIndex { write_back } => {
                        write!(f, "[{}, {}]", base_register, offset_info)?;
                        if write_back {
                            f.write_str("!")?;
                        }
                    }
                    SingleDataTransferIndexType::PostIndex { .. } => {
                        write!(f, "[{}], {}", base_register, offset_info)?;
                    }
                }

                Ok(())
            }
            ArmInstructionType::Mrs {
                destination_register,
                source_psr,
            } => write!(
                f,
                "mrs{} {}, {}",
                self.condition, destination_register, source_psr
            ),
            ArmInstructionType::Msr {
                destination_psr,
                write_flags_field,
                write_status_field,
                write_extension_field,
                write_control_field,
                source_info,
            } => {
                write!(f, "msr{} {}", self.condition, destination_psr)?;

                if write_flags_field
                    || write_status_field
                    || write_extension_field
                    || write_control_field
                {
                    f.write_str("_")?;
                }

                if write_control_field {
                    f.write_str("c")?;
                }

                if write_flags_field {
                    f.write_str("f")?;
                }

                if write_status_field {
                    f.write_str("s")?;
                }

                if write_extension_field {
                    f.write_str("x")?;
                }

                write!(f, ", {}", source_info)?;

                Ok(())
            }
            ArmInstructionType::Stm {
                base_register,
                index_type,
                offset_modifier,
                register_bit_list,
                write_back,
            } => {
                write!(f, "stm{}", self.condition)?;

                match offset_modifier {
                    OffsetModifierType::AddToBase => f.write_str("i")?,
                    OffsetModifierType::SubtractFromBase => f.write_str("d")?,
                };

                match index_type {
                    BlockDataTransferIndexType::PreIndex => f.write_str("b")?,
                    BlockDataTransferIndexType::PostIndex => f.write_str("a")?,
                };

                write!(f, " {}", base_register)?;

                if write_back {
                    f.write_str("!")?;
                }
                f.write_str(" {")?;

                let mut start_idx = 0;
                let mut printed_register = false;
                for (register_idx, register_used) in register_bit_list.into_iter().enumerate() {
                    if !register_used {
                        let idx_delta = register_idx - start_idx;

                        if idx_delta == 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}", start_idx)?;
                            printed_register = true
                        } else if idx_delta > 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}-r{}", start_idx, register_idx - 1)?;
                            printed_register = true;
                        }

                        start_idx = register_idx + 1;
                    }
                }

                let idx_delta = register_bit_list.len() - start_idx;
                if idx_delta == 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}", start_idx)?;
                } else if idx_delta > 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}-r{}", start_idx, register_bit_list.len() - 1)?;
                }

                f.write_str("}")?;

                Ok(())
            }
            ArmInstructionType::Ldm {
                base_register,
                index_type,
                offset_modifier,
                register_bit_list,
                write_back,
            } => {
                f.write_str("ldm")?;
                write!(f, "ldm{}", self.condition)?;

                match offset_modifier {
                    OffsetModifierType::AddToBase => f.write_str("i")?,
                    OffsetModifierType::SubtractFromBase => f.write_str("d")?,
                };

                match index_type {
                    BlockDataTransferIndexType::PreIndex => f.write_str("b")?,
                    BlockDataTransferIndexType::PostIndex => f.write_str("a")?,
                };

                write!(f, " {}", base_register)?;

                if write_back {
                    f.write_str("!")?;
                }
                f.write_str(" {")?;

                let mut start_idx = 0;
                let mut printed_register = false;
                for (register_idx, register_used) in register_bit_list.into_iter().enumerate() {
                    if !register_used {
                        let idx_delta = register_idx - start_idx;

                        if idx_delta == 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}", start_idx)?;
                            printed_register = true
                        } else if idx_delta > 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}-r{}", start_idx, register_idx - 1)?;
                            printed_register = true;
                        }

                        start_idx = register_idx + 1;
                    }
                }

                let idx_delta = register_bit_list.len() - start_idx;
                if idx_delta == 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}", start_idx)?;
                } else if idx_delta > 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}-r{}", start_idx, register_bit_list.len() - 1)?;
                }

                f.write_str("}")?;

                Ok(())
            }
            _ => todo!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ThumbInstruction {
    instruction_type: ThumbInstructionType,
    address: u32,
}

impl Display for ThumbInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.instruction_type {
            ThumbInstructionType::Register {
                operation,
                destination_register,
                source,
                second_operand,
            } => {
                write!(f, "{} {}, {}", operation, destination_register, source)?;
                if let Some(second_operand) = second_operand {
                    write!(f, ", {}", second_operand)?;
                }
                Ok(())
            }
            ThumbInstructionType::Ldr {
                base_register,
                destination_register,
                offset,
                sign_extend,
                size,
            } => {
                f.write_str("ld")?;
                if sign_extend {
                    f.write_str("s")?;
                } else {
                    f.write_str("r")?;
                }

                match size {
                    ThumbLoadStoreDataSize::Byte => f.write_str("b")?,
                    ThumbLoadStoreDataSize::HalfWord => f.write_str("h")?,
                    ThumbLoadStoreDataSize::Word => {}
                };
                f.write_str(" ")?;

                write!(
                    f,
                    "{}, [{}, {}]",
                    destination_register, base_register, offset
                )?;

                Ok(())
            }
            ThumbInstructionType::Str {
                base_register,
                source_register,
                offset,
                size,
            } => {
                f.write_str("str")?;

                match size {
                    ThumbLoadStoreDataSize::Byte => f.write_str("b")?,
                    ThumbLoadStoreDataSize::HalfWord => f.write_str("h")?,
                    ThumbLoadStoreDataSize::Word => {}
                };
                f.write_str(" ")?;

                write!(f, "{}, [{}, {}]", source_register, base_register, offset)?;

                Ok(())
            }
            ThumbInstructionType::Bl { offset } => write!(f, "bl 0x{:08x}", offset),
            ThumbInstructionType::B { condition, offset } => {
                write!(f, "b{} 0x{:08X}", condition, offset)
            }
            ThumbInstructionType::Bx { operand } => write!(f, "bx {}", operand),
            ThumbInstructionType::Push {
                register_bit_list,
                push_lr,
            } => {
                f.write_str("push {")?;
                let mut start_idx = 0;
                let mut printed_register = false;

                for (register_idx, register_used) in register_bit_list.into_iter().enumerate() {
                    if !register_used {
                        let idx_delta = register_idx - start_idx;
                        if idx_delta == 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }
                            write!(f, "r{}", start_idx)?;
                            printed_register = true;
                        } else if idx_delta > 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}-r{}", start_idx, register_idx - 1)?;
                            printed_register = true;
                        }

                        start_idx = register_idx + 1;
                    }
                }

                let idx_delta = register_bit_list.len() - start_idx;
                if idx_delta == 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }
                    write!(f, "r{}", start_idx)?;
                    printed_register = true;
                } else if idx_delta > 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}-r{}", start_idx, register_bit_list.len() - 1)?;
                    printed_register = true;
                }

                if push_lr {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    f.write_str("r14")?;
                }

                f.write_str("}")?;

                Ok(())
            }
            ThumbInstructionType::Pop {
                register_bit_list,
                pop_pc,
            } => {
                f.write_str("pop {")?;
                let mut start_idx = 0;
                let mut printed_register = false;

                for (register_idx, register_used) in register_bit_list.into_iter().enumerate() {
                    if !register_used {
                        let idx_delta = register_idx - start_idx;
                        if idx_delta == 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }
                            write!(f, "r{}", start_idx)?;
                            printed_register = true;
                        } else if idx_delta > 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}-r{}", start_idx, register_idx - 1)?;
                            printed_register = true;
                        }

                        start_idx = register_idx + 1;
                    }
                }

                let idx_delta = register_bit_list.len() - start_idx;
                if idx_delta == 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }
                    write!(f, "r{}", start_idx)?;
                    printed_register = true;
                } else if idx_delta > 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}-r{}", start_idx, register_bit_list.len() - 1)?;
                    printed_register = true;
                }

                if pop_pc {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    f.write_str("r15")?;
                }

                f.write_str("}")?;

                Ok(())
            }
            ThumbInstructionType::AddSpecial {
                source_register,
                dest_register,
                unsigned_offset,
                sign_bit,
            } => {
                if sign_bit {
                    f.write_str("sub")?;
                } else {
                    f.write_str("add")?;
                }

                write!(
                    f,
                    " {}, {}, #{}",
                    dest_register, source_register, unsigned_offset
                )?;

                Ok(())
            }
            ThumbInstructionType::LdmiaWriteBack {
                base_register,
                register_bit_list,
            } => {
                write!(f, "ldmia {}!, {{", base_register)?;

                let mut start_idx = 0;
                let mut printed_register = false;

                for (register_idx, register_used) in register_bit_list.into_iter().enumerate() {
                    if !register_used {
                        let idx_delta = register_idx - start_idx;
                        if idx_delta == 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }
                            write!(f, "r{}", start_idx)?;
                            printed_register = true;
                        } else if idx_delta > 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}-r{}", start_idx, register_idx - 1)?;
                            printed_register = true;
                        }

                        start_idx = register_idx + 1;
                    }
                }

                let idx_delta = register_bit_list.len() - start_idx;
                if idx_delta == 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }
                    write!(f, "r{}", start_idx)?;
                } else if idx_delta > 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}-r{}", start_idx, register_bit_list.len() - 1)?;
                }

                f.write_str("}")?;

                Ok(())
            }
            ThumbInstructionType::StmiaWriteBack {
                base_register,
                register_bit_list,
            } => {
                write!(f, "stmia {}!, {{", base_register)?;

                let mut start_idx = 0;
                let mut printed_register = false;

                for (register_idx, register_used) in register_bit_list.into_iter().enumerate() {
                    if !register_used {
                        let idx_delta = register_idx - start_idx;
                        if idx_delta == 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }
                            write!(f, "r{}", start_idx)?;
                            printed_register = true;
                        } else if idx_delta > 1 {
                            if printed_register {
                                f.write_str(", ")?;
                            }

                            write!(f, "r{}-r{}", start_idx, register_idx - 1)?;
                            printed_register = true;
                        }

                        start_idx = register_idx + 1;
                    }
                }

                let idx_delta = register_bit_list.len() - start_idx;
                if idx_delta == 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }
                    write!(f, "r{}", start_idx)?;
                } else if idx_delta > 1 {
                    if printed_register {
                        f.write_str(", ")?;
                    }

                    write!(f, "r{}-r{}", start_idx, register_bit_list.len() - 1)?;
                }

                f.write_str("}")?;

                Ok(())
            }
            _ => todo!("{:#?}", self),
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

impl ShiftType {
    fn calculate(self, value: u32, shift: u32) -> (bool, u32) {
        assert!(shift < 32);

        let result = match self {
            ShiftType::Lsl => value << shift,
            ShiftType::Lsr => value >> shift,
            ShiftType::Asr => ((value as i32) >> shift) as u32,
            ShiftType::Ror => value.rotate_right(shift),
        };

        let carry = if shift == 0 {
            false
        } else {
            match self {
                ShiftType::Lsl => value.get_bit(32 - (shift as usize)),
                ShiftType::Lsr => value.get_bit((shift as usize) - 1),
                ShiftType::Asr => value.get_bit((shift as usize) - 1),
                ShiftType::Ror => value.get_bit((shift as usize) - 1),
            }
        };

        (carry, result)
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

impl Display for PsrTransferPsr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsrTransferPsr::Cpsr => f.write_str("cpsr"),
            PsrTransferPsr::Spsr => f.write_str("spsr"),
        }
    }
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

impl Display for SingleDataTransferOffsetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value {
            SingleDataTransferOffsetValue::Immediate { offset } => {
                f.write_str("#")?;
                if self.sign {
                    f.write_str("-")?;
                }
                write!(f, "{}", offset)?;
            }
            SingleDataTransferOffsetValue::Register { offset_register } => {
                if self.sign {
                    f.write_str("-")?;
                }
                write!(f, "{}", offset_register)?;
            }
            SingleDataTransferOffsetValue::RegisterImmediate {
                offset_register,
                shift_amount,
                shift_type,
            } => {
                if self.sign {
                    f.write_str("-")?;
                }
                write!(f, "{}, {} #{}", offset_register, shift_type, shift_amount)?;
            }
        };

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AluSecondOperandInfo {
    Register {
        shift_info: AluSecondOperandRegisterShiftInfo,
        shift_type: ShiftType,
        register: Register,
    },
    Immediate {
        value: u32,
    },
}

impl Display for AluSecondOperandInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AluSecondOperandInfo::Register {
                register,
                shift_info,
                shift_type,
            } => write!(f, "{}, {} {}", register, shift_type, shift_info),
            AluSecondOperandInfo::Immediate { value } => write!(f, "#{}", value),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AluSecondOperandRegisterShiftInfo {
    Immediate(u32),
    Register(Register),
}

impl Display for AluSecondOperandRegisterShiftInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AluSecondOperandRegisterShiftInfo::Immediate(value) => write!(f, "#{}", value),
            AluSecondOperandRegisterShiftInfo::Register(register) => write!(f, "{}", register),
        }
    }
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

impl Display for AluOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AluOperation::And => f.write_str("and"),
            AluOperation::Eor => f.write_str("eor"),
            AluOperation::Sub => f.write_str("sub"),
            AluOperation::Rsb => f.write_str("rsb"),
            AluOperation::Add => f.write_str("add"),
            AluOperation::Adc => f.write_str("adc"),
            AluOperation::Sbc => f.write_str("sbc"),
            AluOperation::Rsc => f.write_str("rsc"),
            AluOperation::Tst => f.write_str("tst"),
            AluOperation::Teq => f.write_str("teq"),
            AluOperation::Cmp => f.write_str("cmp"),
            AluOperation::Cmn => f.write_str("cmn"),
            AluOperation::Orr => f.write_str("orr"),
            AluOperation::Mov => f.write_str("mov"),
            AluOperation::Bic => f.write_str("bic"),
            AluOperation::Mvn => f.write_str("mvn"),
        }
    }
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
    Smlaxy,
    Smlawy,
    Smulwy,
    Smlalxy,
    Smulxy,
}

#[derive(Clone, Copy, Debug)]
pub enum MsrSourceInfo {
    Register(Register),
    Immediate { value: u32 },
}

impl Display for MsrSourceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MsrSourceInfo::Register(register) => write!(f, "{}", register),
            MsrSourceInfo::Immediate { value } => write!(f, "#{}", value),
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
                const NEW_STATE_BIT_INDEX: usize = 0;

                let new_state_bit = value.get_bit(NEW_STATE_BIT_INDEX);
                self.set_cpu_state_bit(new_state_bit);

                self.r15 = value & (!1);
            }
            (_, Register::Cpsr) => self.cpsr = value,
            (mode @ (CpuMode::User | CpuMode::System), register @ Register::Spsr) => {
                unreachable!("{:?}, {:?}", mode, register)
            }
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
            (mode @ (CpuMode::User | CpuMode::System), register @ Register::Spsr) => {
                unreachable!("{:?}, {:?}", mode, register)
            }
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
    pub fn decode(&mut self) -> Instruction {
        // println!("decoding: {:?}", self.get_instruction_mode());
        let pc = self.r15;
        match self.get_instruction_mode() {
            InstructionSet::Arm => {
                let opcode = self.bus.read_word_address(self.r15);
                println!("0b{:032b}", opcode);
                let condition = opcode.get_condition();

                let maybe_instruction_type = None
                    .or_else(|| Self::try_decode_arm_branch(opcode))
                    .or_else(|| Self::try_decode_arm_branch_exchange(opcode))
                    .or_else(|| Self::try_decode_arm_data_process(opcode))
                    .or_else(|| Self::try_decode_arm_multiply(opcode))
                    .or_else(|| Self::try_decode_arm_psr_transfer(opcode))
                    .or_else(|| Self::try_decode_arm_single_data_transfer(opcode))
                    .or_else(|| Self::try_decode_arm_block_data_transfer(opcode));

                let instruction_type = if let Some(instruction_type) = maybe_instruction_type {
                    instruction_type
                } else {
                    todo!("unrecognized ARM opcode")
                };

                self.r15 += 4;
                Instruction::ArmInstruction(ArmInstruction {
                    condition,
                    instruction_type,
                    address: pc,
                })
            }
            InstructionSet::Thumb => {
                println!("address: 0x{:08x}", pc);
                let opcode = self.bus.read_halfword_address(pc);
                let second_opcode = self.bus.read_halfword_address(pc + 2);
                println!("0b{:016b}", opcode);

                let maybe_instruction_type = None
                    .or_else(|| Self::try_decode_thumb_register_operation(opcode))
                    .or_else(|| Self::try_decode_thumb_memory_load_store(opcode))
                    .or_else(|| Self::try_decode_thumb_memory_addressing(opcode))
                    .or_else(|| Self::try_decode_thumb_memory_multiple_load_store(opcode))
                    .or_else(|| Self::try_decode_thumb_jump_call(opcode));

                let maybe_long_instruction_type =
                    Self::try_decode_thumb_long_branch_link(opcode, second_opcode);

                let instruction_type = if let Some(instruction_type) = maybe_instruction_type {
                    self.r15 += 2;
                    instruction_type
                } else if let Some(long_instruction_type) = maybe_long_instruction_type {
                    self.r15 += 4;
                    long_instruction_type
                } else {
                    todo!("unrecognized Thumb opcode")
                };

                Instruction::ThumbInstruction(ThumbInstruction {
                    instruction_type,
                    address: pc,
                })
            }
        }
    }
}

impl Cpu {
    fn try_decode_arm_branch(opcode: u32) -> Option<ArmInstructionType> {
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
                let unshifted_value = opcode.get_bit_range(SECOND_OPERAND_IMMEDIATE_BIT_RANGE);

                let value = unshifted_value.rotate_right(shift);

                AluSecondOperandInfo::Immediate { value }
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

                    AluSecondOperandRegisterShiftInfo::Register(shift_register)
                } else {
                    // Shift by Immediate
                    const SHIFT_AMOUNT_BIT_RANGE: RangeInclusive<usize> = 7..=11;

                    let shift_amount = opcode.get_bit_range(SHIFT_AMOUNT_BIT_RANGE);

                    AluSecondOperandRegisterShiftInfo::Immediate(shift_amount)
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
        const MULTIPLY_MASK: u32 = 0b00011100_00000000_00000000_00000000;
        const MULTIPLY_MASK_RESULT: u32 = 0b00000000_00000000_00000000_00000000;

        fn lookup_mul_opcode(opcode_value: u32) -> MultiplyOperation {
            match opcode_value {
                0b0000 => MultiplyOperation::Mul,
                0b0001 => MultiplyOperation::Mla,
                0b0010 => MultiplyOperation::Umaal,
                0b0100 => MultiplyOperation::Umull,
                0b0101 => MultiplyOperation::Umlal,
                0b0110 => MultiplyOperation::Smull,
                0b0111 => MultiplyOperation::Smlal,
                0b1000 => MultiplyOperation::Smlaxy,
                0b1001 => MultiplyOperation::Smulwy,
                0b1010 => MultiplyOperation::Smlalxy,
                0b1011 => MultiplyOperation::Smulxy,
                _ => unreachable!(),
            }
        }

        return None;
        if opcode.match_mask(MULTIPLY_MASK, MULTIPLY_MASK_RESULT) {
            todo!();

            const MUST_BE_000_BIT_RANGE: RangeInclusive<usize> = 25..=27;
            const MUL_OPCODE_BIT_RANGE: RangeInclusive<usize> = 21..=24;
            const SET_CONDITION_CODES_BIT_INDEX: usize = 20;
            const DESTINATION_REGISTER_OFFSET: usize = 16;
            const ACCUMULATE_REGISTER_OFFSET: usize = 12;

            let mul_operation_value = opcode.get_bit_range(MUL_OPCODE_BIT_RANGE);
            let set_condition_codes_bit = opcode.get_bit(SET_CONDITION_CODES_BIT_INDEX);
            let destination_register = opcode.get_register_at_offset(DESTINATION_REGISTER_OFFSET);
            let accumulate_register = opcode.get_register_at_offset(ACCUMULATE_REGISTER_OFFSET);

            let mul_operation = lookup_mul_opcode(opcode.get_bit_range(MUL_OPCODE_BIT_RANGE));
        } else {
            None
        }
    }

    fn try_decode_arm_psr_transfer(opcode: u32) -> Option<ArmInstructionType> {
        const MUST_BE_00_BIT_RANGE: RangeInclusive<usize> = 26..=27;
        const IMMEDIATE_OFFSET_BIT_INDEX: usize = 25;
        const MUST_BE_10_BIT_RANGE: RangeInclusive<usize> = 23..=24;
        const SOURCE_DEST_PSR_BIT_INDEX: usize = 22;
        const PSR_OPCODE_BIT_INDEX: usize = 21;
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

        let immediate_operand_flag_bit = opcode.get_bit(IMMEDIATE_OFFSET_BIT_INDEX);
        let source_dest_psr_bit = opcode.get_bit(SOURCE_DEST_PSR_BIT_INDEX);
        let opcode_bit = opcode.get_bit(PSR_OPCODE_BIT_INDEX);

        let source_dest_psr = if source_dest_psr_bit {
            // SPSR
            PsrTransferPsr::Spsr
        } else {
            // CPSR
            PsrTransferPsr::Cpsr
        };

        let transfer_type = if opcode_bit {
            // MSR
            PsrTransferType::Msr
        } else {
            // MRS
            PsrTransferType::Mrs
        };

        match transfer_type {
            PsrTransferType::Mrs => {
                const DEST_REGISTER_OFFSET: usize = 12;
                let destination_register = opcode.get_register_at_offset(DEST_REGISTER_OFFSET);

                Some(ArmInstructionType::Mrs {
                    source_psr: source_dest_psr,
                    destination_register,
                })
            }
            PsrTransferType::Msr => {
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
        }
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
                },
                2 => todo!("support sign extension"),
                3 => todo!("support sign extension"),
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
            source: ThumbRegisterOrImmediate::Register(source_register),
            second_operand: Some(ThumbRegisterOrImmediate::Immediate(offset)),
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
                    source: ThumbRegisterOrImmediate::Register(source_register),
                    second_operand: Some(second_operand),
                }
            }
            SUB_REGISTER_OPCODE_VALUE => {
                let register_operand = opcode.get_register_at_offset(REGISTER_OPERAND_OFFSET);
                let second_operand = ThumbRegisterOrImmediate::Register(register_operand);
                ThumbInstructionType::Register {
                    destination_register: dest_register,
                    operation: ThumbRegisterOperation::Sub,
                    source: ThumbRegisterOrImmediate::Register(source_register),
                    second_operand: Some(second_operand),
                }
            }
            ADD_IMMEDIATE_OPCODE_VALUE => {
                let immediate_operand =
                    u32::from(opcode.get_bit_range(IMMEDIATE_OPERAND_BIT_RANGE));
                let second_operand = ThumbRegisterOrImmediate::Immediate(immediate_operand);
                ThumbInstructionType::Register {
                    destination_register: dest_register,
                    operation: ThumbRegisterOperation::Add,
                    source: ThumbRegisterOrImmediate::Register(source_register),
                    second_operand: Some(second_operand),
                }
            }
            SUB_IMMEDIATE_OPCODE_VALUE => {
                let immediate_operand =
                    u32::from(opcode.get_bit_range(IMMEDIATE_OPERAND_BIT_RANGE));
                let second_operand = ThumbRegisterOrImmediate::Immediate(immediate_operand);
                ThumbInstructionType::Register {
                    destination_register: dest_register,
                    operation: ThumbRegisterOperation::Sub,
                    source: ThumbRegisterOrImmediate::Register(source_register),
                    second_operand: Some(second_operand),
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
            destination_register: dest_register,
            operation,
            second_operand: None,
            source: ThumbRegisterOrImmediate::Immediate(immediate),
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
            destination_register: dest_register,
            operation: operation_type,
            second_operand: None,
            source: ThumbRegisterOrImmediate::Register(source_register),
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
            0 => ThumbInstructionType::Register {
                destination_register: dest_register,
                source: ThumbRegisterOrImmediate::Register(source_register),
                operation: ThumbRegisterOperation::Add,
                second_operand: None,
            },
            1 => ThumbInstructionType::Register {
                destination_register: dest_register,
                source: ThumbRegisterOrImmediate::Register(source_register),
                operation: ThumbRegisterOperation::Cmp,
                second_operand: None,
            },
            2 => ThumbInstructionType::Register {
                destination_register: dest_register,
                source: ThumbRegisterOrImmediate::Register(source_register),
                operation: ThumbRegisterOperation::Mov,
                second_operand: None,
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
            0xF => todo!("SWI"),
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
    // Second Instruction - PC = LR + (nn SHL 1), and LR = PC+2 OR 1 (and BLX: T=0)
    // 15-11  Opcode
    //      11111b: BL label   ;branch long with link
    //      11101b: BLX label  ;branch long with link switch to ARM mode (ARM9) (UNUSED)
    // 10-0   nn - Lower 11 bits of Target Address (BLX: Bit0 Must be zero)
    //
    // Offset is calculated as sign-extended (first_nn << 12) | (second_nn << 1).
    fn try_decode_thumb_long_branch_link(
        first_opcode: u16,
        second_opcode: u16,
    ) -> Option<ThumbInstructionType> {
        const OPCODE_1_MUST_BE_11110_BIT_RANGE: RangeInclusive<usize> = 11..=15;
        const OPCODE_1_TARGET_ADDRESS_UPPER_11_BITS_RANGE: RangeInclusive<usize> = 0..=10;

        const OPCODE_2_MUST_BE_11111_BIT_RANGE: RangeInclusive<usize> = 11..=15;
        const OPCODE_2_TARGET_ADDRESS_LOWER_11_BITS_RANGE: RangeInclusive<usize> = 0..=10;

        if first_opcode.get_bit_range(OPCODE_1_MUST_BE_11110_BIT_RANGE) != 0b11110 {
            return None;
        }

        if second_opcode.get_bit_range(OPCODE_2_MUST_BE_11111_BIT_RANGE) != 0b11111 {
            return None;
        }

        let target_offset_upper_11_bits =
            first_opcode.get_bit_range(OPCODE_1_TARGET_ADDRESS_UPPER_11_BITS_RANGE);
        let target_offset_lower_11_bits =
            second_opcode.get_bit_range(OPCODE_2_TARGET_ADDRESS_LOWER_11_BITS_RANGE);

        let offset_unsigned = (u32::from(target_offset_lower_11_bits) << 1)
            | (u32::from(target_offset_upper_11_bits) << 12);

        // 23-bit sign extension, by left shifting until effective sign bit is in MSB, then ASR
        // an equal amount back over.
        let offset = ((offset_unsigned as i32) << 9) >> 9;

        Some(ThumbInstructionType::Bl { offset })
    }
}

impl Cpu {
    pub fn execute(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::ArmInstruction(arm_instruction) => {
                if self.evaluate_instruction_condition(arm_instruction.condition) {
                    match arm_instruction.instruction_type {
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
                        ArmInstructionType::Ldr {
                            access_size,
                            base_register,
                            destination_register,
                            index_type,
                            offset_info,
                        } => self.execute_arm_ldr(
                            access_size,
                            base_register,
                            destination_register,
                            index_type,
                            offset_info,
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
                        _ => todo!("{:#08x?}", arm_instruction),
                    }
                } else {
                    eprintln!("evaluate false");
                }
            }
            Instruction::ThumbInstruction(thumb_instruction) => {
                match thumb_instruction.instruction_type {
                    ThumbInstructionType::Register {
                        destination_register,
                        operation,
                        second_operand,
                        source,
                    } => self.execute_thumb_register_operation(
                        destination_register,
                        operation,
                        second_operand,
                        source,
                    ),
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
                    ThumbInstructionType::Bl { offset } => self.execute_thumb_bl(offset),
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
                    _ => todo!("{:#016x?}", thumb_instruction),
                }
            }
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
        //   - PC+12 if I=0,R=1 (shift by register)
        //   - otherwise, PC+8 (shift by immediate).
        let pc_operand_calculation = match second_operand {
            AluSecondOperandInfo::Register {
                shift_info: AluSecondOperandRegisterShiftInfo::Register(_),
                ..
            } => |pc| pc + 8,
            _ => |pc| pc + 4,
        };

        let first_operand_value = self.read_register(first_operand, pc_operand_calculation);
        let (shift_carry, second_operand_value) = self.evaluate_alu_second_operand(second_operand);
        eprintln!("shift carry: {}", shift_carry);

        let (unsigned_result, carry_flag, signed_result, overflow_flag) = match operation {
            AluOperation::And => {
                let unsigned_result = first_operand_value & second_operand_value;
                let signed_result = unsigned_result as i32;

                (unsigned_result, Some(shift_carry), signed_result, None)
            }
            AluOperation::Add => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_add(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32);

                (unsigned_result, Some(carry), signed_result, Some(overflow))
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

                (unsigned_result, Some(carry), signed_result, Some(overflow))
            }
            AluOperation::Sub => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);

                (unsigned_result, Some(carry), signed_result, Some(overflow))
            }
            AluOperation::Rsb => {
                let (unsigned_result, carry) =
                    second_operand_value.overflowing_sub(first_operand_value);
                let (signed_result, overflow) =
                    (second_operand_value as i32).overflowing_sub(first_operand_value as i32);

                (unsigned_result, Some(carry), signed_result, Some(overflow))
            }
            AluOperation::Teq => {
                let unsigned_result = first_operand_value ^ second_operand_value;
                let signed_result = unsigned_result as i32;

                (unsigned_result, Some(shift_carry), signed_result, None)
            }
            AluOperation::Cmp => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);

                (unsigned_result, Some(carry), signed_result, Some(overflow))
            }
            AluOperation::Mov => (
                second_operand_value,
                Some(shift_carry),
                second_operand_value as i32,
                None,
            ),
            AluOperation::Bic => {
                let result = first_operand_value & (!second_operand_value);
                (result, Some(shift_carry), result as i32, None)
            }
            AluOperation::Tst => {
                let result = first_operand_value & second_operand_value;
                (result, Some(shift_carry), result as i32, None)
            }
            AluOperation::Orr => {
                let result = first_operand_value | second_operand_value;
                (result, None, result as i32, None)
            }
            AluOperation::Eor => {
                let result = first_operand_value ^ second_operand_value;
                (result, None, result as i32, None)
            }
            _ => todo!("ARM ALU: {:?}", operation),
        };

        if set_conditions {
            self.set_sign_flag(signed_result < 0);
            self.set_zero_flag(unsigned_result == 0);
            if let Some(carry_flag) = carry_flag {
                self.set_carry_flag(carry_flag);
            }

            if let Some(overflow_flag) = overflow_flag {
                self.set_overflow_flag(overflow_flag);
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
        self.r15 = self.r15.wrapping_add(offset as u32).wrapping_add(4);
    }

    // PC is already at $ + 4 because of decoding step.
    // documentation specifies that branch is to ($ + offset + 8).
    // save ($ + 4) in lr.
    fn execute_arm_bl(&mut self, offset: i32) {
        self.r14 = self.r15;
        self.r15 = self.r15.wrapping_add(offset as u32).wrapping_add(4);
    }

    // PC = operand, T = Rn.0
    fn execute_arm_bx(&mut self, operand: Register) {
        let operand_value = self.read_register(operand, |_| todo!());
        println!("new address: 0x{:08x}", operand_value);

        self.write_register(operand_value, Register::R15);
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
                let (_shift_carry, result) =
                    shift_type.calculate(offset_register_value, shift_amount);
                result
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
                let (_shift_carry, result) =
                    shift_type.calculate(offset_register_value, shift_amount);
                result
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

        let value = match access_size {
            SingleDataMemoryAccessSize::Byte => {
                self.bus.read_byte_address(data_read_address) as u32
            }
            SingleDataMemoryAccessSize::HalfWord => {
                self.bus.read_halfword_address(data_read_address) as u32
            }
            SingleDataMemoryAccessSize::Word => self.bus.read_word_address(data_read_address),
            _ => todo!("{:?}", access_size),
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
        println!("storing to base address: 0x{:08x}", current_address);

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
}

impl Cpu {
    fn execute_thumb_register_operation(
        &mut self,
        destination_register: Register,
        operation: ThumbRegisterOperation,
        second_operand: Option<ThumbRegisterOrImmediate>,
        source: ThumbRegisterOrImmediate,
    ) {
        let destination_value = self.read_register(destination_register, |pc| pc + 2);
        let source_value = self.evaluate_thumb_register_or_immedate(source, |pc| pc + 2);

        let (first_operand_value, second_operand_value) =
            if let Some(second_operand) = second_operand {
                // if explicit second operand, first operand is source, second operand is given second operand
                let second_value =
                    self.evaluate_thumb_register_or_immedate(second_operand, |_| unreachable!());
                (source_value, second_value)
            } else {
                // if no second operand, first operand is destination value and second operand is source
                (destination_value, source_value)
            };

        let (unsigned_result, carry_flag, signed_result, overflow_flag) = match operation {
            ThumbRegisterOperation::Add => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_add(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32);

                (unsigned_result, carry, signed_result, overflow)
            }
            ThumbRegisterOperation::Sub => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);
                (unsigned_result, carry, signed_result, overflow)
            }
            ThumbRegisterOperation::Neg => {
                let (unsigned_result, carry) = 0u32.overflowing_sub(second_operand_value);
                let (signed_result, overflow) = 0i32.overflowing_sub(second_operand_value as i32);
                (unsigned_result, carry, signed_result, overflow)
            }
            ThumbRegisterOperation::Cmp => {
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_sub(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_sub(second_operand_value as i32);
                (unsigned_result, carry, signed_result, overflow)
            }
            ThumbRegisterOperation::Mov => (
                second_operand_value,
                false,
                second_operand_value as i32,
                false,
            ),
            ThumbRegisterOperation::Mvn => {
                let result = !second_operand_value;
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Lsl => {
                let result = first_operand_value << second_operand_value;
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Lsr => {
                let result = first_operand_value >> second_operand_value;
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Asr => {
                let result = (first_operand_value as i32) >> second_operand_value;
                (result as u32, false, result, false)
            }
            ThumbRegisterOperation::Tst => {
                let result = first_operand_value & second_operand_value;
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::And => {
                let result = first_operand_value & second_operand_value;
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Orr => {
                let result = first_operand_value | second_operand_value;
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Eor => {
                let result = first_operand_value ^ second_operand_value;
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Bic => {
                let result = first_operand_value & (!second_operand_value);
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Ror => {
                let result = first_operand_value.rotate_right(second_operand_value);
                (result, false, result as i32, false)
            }
            ThumbRegisterOperation::Mul => {
                let result = first_operand_value * second_operand_value;
                (result, false, result as i32, false)
            }
            _ => todo!("{:?}", operation),
        };

        self.set_carry_flag(carry_flag);
        self.set_overflow_flag(overflow_flag);
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

    fn execute_thumb_ldr(
        &mut self,
        base_register: Register,
        offset: ThumbRegisterOrImmediate,
        destination_register: Register,
        size: ThumbLoadStoreDataSize,
        sign_extend: bool,
    ) {
        let base_address = self.read_register(base_register, |pc| (pc + 2) & (!2));
        println!("base address: 0x{:08x}", base_address);
        let base_offset = match offset {
            ThumbRegisterOrImmediate::Immediate(immediate) => immediate,
            ThumbRegisterOrImmediate::Register(register) => {
                self.read_register(register, |_| unreachable!())
            }
        };
        println!("offset: 0x{:08x}", base_offset);

        let real_address = base_address + base_offset;
        println!("real addr: 0x{:08x}", real_address);

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
        println!("thumb ldr with value: 0x{:08x}", result_value);

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
        println!("base address: 0x{:08x}", base_address);
        let base_offset = match offset {
            ThumbRegisterOrImmediate::Immediate(immediate) => immediate,
            ThumbRegisterOrImmediate::Register(register) => {
                self.read_register(register, |_| unreachable!())
            }
        };
        println!("base offset: 0x{:08x}", base_offset);

        let real_address = base_address.wrapping_add(base_offset);
        println!("real address: 0x{:08x}", real_address);
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
        println!("thumb branch with condition: {:?}", condition);
        if self.evaluate_instruction_condition(condition) {
            self.r15 = self.r15.wrapping_add(offset as u32).wrapping_add(2);
        } else {
            println!("no thumb branch");
        }
    }

    // PC = $ + 4 + offset
    // LR = ($ + 4) OR 1
    // Thumb bl is decoded as two instructions, so PC == $ + 4 at execution time.
    fn execute_thumb_bl(&mut self, offset: i32) {
        self.r14 = self.r15 | 1;
        self.r15 = self.r15.wrapping_add(offset as u32);
        println!("thumb bl new pc: 0x{:08x}", self.r15);
    }

    fn execute_thumb_bx(&mut self, operand: Register) {
        // "BX R15: CPU switches to ARM state, and PC is auto-aligned as (($+4) AND NOT 2)."
        //
        // Also clear low bit to ensure that CPU switches to ARM state.
        let operand_value = self.read_register(operand, |pc| (pc + 2) & (!3));
        println!("new address: 0x{:08x}", operand_value);

        self.write_register(operand_value, Register::R15);
    }

    fn execute_thumb_push(&mut self, register_bit_list: [bool; 8], push_lr: bool) {
        // Lowest register index goes at lowest address. As this is equivalent to STMDB, lowest register index needs to be considered last.
        //  In order to achieve this, iterate in reverse order.
        if push_lr {
            let lr_value = self.read_register(Register::R14, |_| unreachable!());

            self.r13 -= 4;
            self.bus.write_word_address(lr_value, self.r13);
        }

        for (register_idx, register_pushed) in register_bit_list.into_iter().enumerate().rev() {
            if register_pushed {
                let pushed_register = Register::from_index(register_idx as u32);
                let pushed_register_value = self.read_register(pushed_register, |_| unreachable!());

                self.r13 -= 4;
                self.bus.write_word_address(pushed_register_value, self.r13);
            }
        }
    }

    fn execute_thumb_pop(&mut self, register_bit_list: [bool; 8], pop_pc: bool) {
        for (register_idx, register_popped) in register_bit_list.into_iter().enumerate() {
            if register_popped {
                let popped_register = Register::from_index(register_idx as u32);
                let popped_register_value = self.bus.read_word_address(self.r13);

                self.r13 += 4;
                self.write_register(popped_register_value, popped_register);
            }
        }

        if pop_pc {
            // POP {PC} ignores the least significant bit of the return address (processor remains in thumb state even if bit0 was cleared).
            let pc_value = self.bus.read_word_address(self.r13) | 1;
            println!("new pc value: 0x{:08x}", pc_value);

            self.r13 += 4;
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

    fn evaluate_alu_second_operand(&self, info: AluSecondOperandInfo) -> (bool, u32) {
        match info {
            AluSecondOperandInfo::Immediate { value } => (false, value),
            AluSecondOperandInfo::Register {
                register,
                shift_info,
                shift_type,
            } => {
                let register_value = self.read_register(register, |pc| pc);
                let shift_amount = match shift_info {
                    AluSecondOperandRegisterShiftInfo::Immediate(shift) => shift,
                    AluSecondOperandRegisterShiftInfo::Register(shift_register) => {
                        // When using R15 as operand (Rm or Rn), the returned value depends on the instruction:
                        //   - PC+12 if I=0,R=1 (shift by register)
                        //   - otherwise, PC+8 (shift by immediate).
                        //
                        // The first case is always present here.
                        self.read_register(shift_register, |pc| pc + 8)
                    }
                };
                shift_type.calculate(register_value, shift_amount)
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

    fn get_fiq_disable(&self) -> bool {
        self.cpsr.get_bit(Self::FIQ_DISABLE_BIT_OFFSET)
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
