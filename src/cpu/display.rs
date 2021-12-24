use super::{
    AluOperation, AluSecondOperandInfo, ArmInstruction, ArmInstructionType, ArmRegisterOrImmediate,
    BlockDataTransferIndexType, Cpu, InstructionCondition, MsrSourceInfo, MultiplyOperation,
    OffsetModifierType, PsrTransferPsr, Register, ShiftType, SingleDataMemoryAccessSize,
    SingleDataTransferIndexType, SingleDataTransferOffsetInfo, SingleDataTransferOffsetValue,
    ThumbHighRegisterOperation, ThumbInstruction, ThumbInstructionType, ThumbLoadStoreDataSize,
    ThumbRegisterOrImmediate,
};

use std::fmt::Display;

use super::ThumbRegisterOperation;

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

impl Display for OffsetModifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OffsetModifierType::AddToBase => f.write_str("+"),
            OffsetModifierType::SubtractFromBase => f.write_str("-"),
        }
    }
}

impl Display for ThumbHighRegisterOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThumbHighRegisterOperation::Add => f.write_str("add"),
            ThumbHighRegisterOperation::Cmp => f.write_str("cmp"),
            ThumbHighRegisterOperation::Mov => f.write_str("mov"),
        }
    }
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

impl Display for ThumbRegisterOrImmediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThumbRegisterOrImmediate::Immediate(value) => write!(f, "#{}", value),
            ThumbRegisterOrImmediate::Register(register) => write!(f, "{}", register),
        }
    }
}

impl Display for ArmInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.instruction_type {
            ArmInstructionType::Alu {
                operation,
                set_conditions,
                first_operand,
                second_operand,
                destination_operand,
            } => {
                let set_string = if set_conditions { "s" } else { "" };

                match operation {
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
                        "{}{}{} {}, {}, {}",
                        operation,
                        set_string,
                        self.condition,
                        destination_operand,
                        first_operand,
                        second_operand
                    ),
                    AluOperation::Tst
                    | AluOperation::Teq
                    | AluOperation::Cmp
                    | AluOperation::Cmn => {
                        write!(
                            f,
                            "{}{}{} {}, {}",
                            operation, set_string, self.condition, first_operand, second_operand
                        )
                    }
                    AluOperation::Mov | AluOperation::Mvn => write!(
                        f,
                        "{}{}{} {}, {}",
                        operation, set_string, self.condition, destination_operand, second_operand
                    ),
                    _ => todo!(),
                }
            }
            ArmInstructionType::B { offset } => write!(f, "b{} 0x{:08X}", self.condition, offset),
            ArmInstructionType::Bl { offset } => write!(f, "bl{} 0x{:08X}", self.condition, offset),
            ArmInstructionType::Bx { operand } => write!(f, "bx{} {}", self.condition, operand),
            ArmInstructionType::Ldr {
                access_size,
                base_register,
                destination_register,
                index_type,
                offset_info,
                sign_extend,
            } => {
                write!(f, "ldr{}", self.condition)?;
                if sign_extend {
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
            ArmInstructionType::Mul {
                accumulate_register,
                destination_register,
                operand_register_rm,
                operand_register_rs,
                operation,
                set_conditions: _,
            } => match operation {
                MultiplyOperation::Mul => write!(
                    f,
                    "mul{} {}, {}, {}",
                    self.condition, destination_register, operand_register_rm, operand_register_rs
                ),
                MultiplyOperation::Mla => write!(
                    f,
                    "mla{} {}, {}, {}, {}",
                    self.condition,
                    destination_register,
                    operand_register_rm,
                    operand_register_rs,
                    accumulate_register,
                ),
                MultiplyOperation::Umull => write!(
                    f,
                    "umull{} {}, {}, {}, {}",
                    self.condition,
                    accumulate_register,
                    destination_register,
                    operand_register_rm,
                    operand_register_rs
                ),
                MultiplyOperation::Umlal => write!(
                    f,
                    "umlal{} {}, {}, {}, {}",
                    self.condition,
                    accumulate_register,
                    destination_register,
                    operand_register_rm,
                    operand_register_rs
                ),
                MultiplyOperation::Smull => write!(
                    f,
                    "smull{} {}, {}, {}, {}",
                    self.condition,
                    accumulate_register,
                    destination_register,
                    operand_register_rm,
                    operand_register_rs
                ),
                MultiplyOperation::Smlal => write!(
                    f,
                    "smlal{} {}, {}, {}, {}",
                    self.condition,
                    accumulate_register,
                    destination_register,
                    operand_register_rm,
                    operand_register_rs
                ),
                _ => todo!("{:?}", operation),
            },
            ArmInstructionType::Swi { comment } => write!(f, "swi #{}", comment),
            _ => todo!("{:#?}", self),
        }
    }
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
                write!(
                    f,
                    "{} {}, {}, {}",
                    operation, destination_register, source, second_operand
                )
            }
            ThumbInstructionType::HighRegister {
                destination_register,
                operation,
                source,
            } => {
                write!(f, "{} {}, {}", operation, destination_register, source)
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
            ThumbInstructionType::BlPartOne { offset } => write!(f, "bl_1 0x{:08x}", offset),
            ThumbInstructionType::BlPartTwo { offset } => write!(f, "bl_2 0x{:04x}", offset),
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

impl Display for PsrTransferPsr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsrTransferPsr::Cpsr => f.write_str("cpsr"),
            PsrTransferPsr::Spsr => f.write_str("spsr"),
        }
    }
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

impl Display for AluSecondOperandInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            AluSecondOperandInfo::Register {
                register,
                shift_info,
                shift_type,
            } => write!(f, "{}, {} {}", register, shift_type, shift_info),
            AluSecondOperandInfo::Immediate { base, shift } => {
                write!(f, "#{}", base.rotate_right(shift))
            }
        }
    }
}

impl Display for ArmRegisterOrImmediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArmRegisterOrImmediate::Immediate(value) => write!(f, "#{}", value),
            ArmRegisterOrImmediate::Register(register) => write!(f, "{}", register),
        }
    }
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
impl Display for MsrSourceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MsrSourceInfo::Register(register) => write!(f, "{}", register),
            MsrSourceInfo::Immediate { value } => write!(f, "#{}", value),
        }
    }
}
