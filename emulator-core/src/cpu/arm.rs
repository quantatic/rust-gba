use super::{Cpu, ExceptionType, InstructionCondition, Register, ShiftType};

use crate::bus::BusAccessType;
use crate::cpu::thumb::decode_thumb;
use crate::{BitManipulation, DataAccess, InstructionSet};

use std::fmt::Display;
use std::ops::RangeInclusive;

#[derive(Clone, Copy, Debug)]
pub(super) enum OffsetModifierType {
    AddToBase,
    SubtractFromBase,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SingleDataMemoryAccessSize {
    Byte,
    HalfWord,
    Word,
    DoubleWord,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum ArmInstructionType {
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
        force_user_mode: bool,
    },
    Stm {
        index_type: BlockDataTransferIndexType,
        offset_modifier: OffsetModifierType,
        write_back: bool,
        base_register: Register,
        register_bit_list: [bool; 16],
        force_user_mode: bool,
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
    Swp {
        access_size: SwpAccessSize,
        base_register: Register,
        dest_register: Register,
        source_register: Register,
    },
    Invalid {
        opcode: u32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ArmInstruction {
    instruction_type: ArmInstructionType,
    condition: InstructionCondition,
}

impl ArmInstruction {
    pub(super) fn instruction_type(&self) -> ArmInstructionType {
        self.instruction_type
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
pub enum SwpAccessSize {
    Word,
    Byte,
}

impl Cpu {
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
                        // The second case is always present here.
                        //
                        // When shifting by register, only lower 8bit 0-255 used.
                        let register_value = self.read_register(register, |pc| pc);

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
                        let register_value = self.read_register(register, |pc| pc + 4);
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

fn get_condition(opcode: u32) -> InstructionCondition {
    const CONDITION_SHIFT: usize = 28;
    const CONDITION_MASK: u32 = 0b1111 << CONDITION_SHIFT;

    match (opcode & CONDITION_MASK) >> CONDITION_SHIFT {
        0 => InstructionCondition::Equal,
        1 => InstructionCondition::NotEqual,
        2 => InstructionCondition::UnsignedHigherOrSame,
        3 => InstructionCondition::UnsignedLower,
        4 => InstructionCondition::SignedNegative,
        5 => InstructionCondition::SignedPositiveOrZero,
        6 => InstructionCondition::SignedOverflow,
        7 => InstructionCondition::SignedNoOverflow,
        8 => InstructionCondition::UnsignedHigher,
        9 => InstructionCondition::UnsignedLowerOrSame,
        10 => InstructionCondition::SignedGreaterOrEqual,
        11 => InstructionCondition::SignedLessThan,
        12 => InstructionCondition::SignedGreaterThan,
        13 => InstructionCondition::SignedLessOrEqual,
        14 => InstructionCondition::Always,
        15 => InstructionCondition::Never,
        _ => unreachable!(),
    }
}

fn get_register_at_offset(opcode: u32, offset: usize) -> Register {
    let mask = 0b1111 << offset;
    let register_index = (opcode & mask) >> offset;
    Register::from_index(register_index)
}

fn get_shift_type(opcode: u32) -> ShiftType {
    match opcode.get_bit_range(5..=6) {
        0 => ShiftType::Lsl,
        1 => ShiftType::Lsr,
        2 => ShiftType::Asr,
        3 => ShiftType::Ror,
        _ => unreachable!(),
    }
}

pub fn decode_arm(opcode: u32) -> ArmInstruction {
    let condition = get_condition(opcode);

    const OPCODE_MASK: u32 = 0b00001110_00000000_00000000_00000000;
    const MUST_BE_000: u32 = 0b00000000_00000000_00000000_00000000;
    const MUST_BE_001: u32 = 0b00000010_00000000_00000000_00000000;
    const MUST_BE_010: u32 = 0b00000100_00000000_00000000_00000000;
    const MUST_BE_011: u32 = 0b00000110_00000000_00000000_00000000;
    const MUST_BE_100: u32 = 0b00001000_00000000_00000000_00000000;
    const MUST_BE_101: u32 = 0b00001010_00000000_00000000_00000000;
    const MUST_BE_110: u32 = 0b00001100_00000000_00000000_00000000;
    const MUST_BE_111: u32 = 0b00001110_00000000_00000000_00000000;

    let mask_result = opcode & OPCODE_MASK;
    let maybe_instruction_type = if mask_result == MUST_BE_000 {
        None.or_else(|| try_decode_arm_branch_exchange(opcode))
            .or_else(|| try_decode_arm_data_process(opcode))
            .or_else(|| try_decode_arm_multiply(opcode))
            .or_else(|| try_decode_arm_psr_transfer(opcode))
            .or_else(|| try_decode_arm_special_single_data_transfer(opcode))
            .or_else(|| try_decode_arm_single_data_swap(opcode))
    } else if mask_result == MUST_BE_001 {
        None.or_else(|| try_decode_arm_data_process(opcode))
            .or_else(|| try_decode_arm_psr_transfer(opcode))
    } else if mask_result == MUST_BE_010 || mask_result == MUST_BE_011 {
        try_decode_arm_single_data_transfer(opcode)
    } else if mask_result == MUST_BE_100 {
        try_decode_arm_block_data_transfer(opcode)
    } else if mask_result == MUST_BE_101 {
        try_decode_arm_branch_basic(opcode)
    } else if mask_result == MUST_BE_110 {
        None
    } else if mask_result == MUST_BE_111 {
        try_decode_arm_swi(opcode)
    } else {
        None
    };

    let instruction_type = if let Some(instruction_type) = maybe_instruction_type {
        instruction_type
    } else {
        ArmInstructionType::Invalid { opcode }
    };

    ArmInstruction {
        condition,
        instruction_type,
    }
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

            let operand = get_register_at_offset(opcode, OPERAND_REGISTER_OFFSET);

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
        let first_operand = get_register_at_offset(opcode, FIRST_OPERATION_REGISTER_OFFSET);
        let destination_operand =
            get_register_at_offset(opcode, DESTINATION_OPERATION_REGISTER_OFFSET);

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

            let shift_type = get_shift_type(opcode);
            let shift_by_register_bit = opcode.get_bit(SHIFT_BY_REGISTER_BIT_INDEX);
            let second_operand_register =
                get_register_at_offset(opcode, SECOND_OPERAND_REGISTER_OFFSET);

            let shift_info = if shift_by_register_bit {
                // Shift by Register
                const SHIFT_REGISTER_OFFSET: usize = 8;
                const MUST_BE_0_BIT_RANGE: RangeInclusive<usize> = 7..=7;

                let shift_register = get_register_at_offset(opcode, SHIFT_REGISTER_OFFSET);

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

    if opcode.get_bit_range(MUST_BE_000_BIT_RANGE) != 0b000 {
        return None;
    }

    if opcode.get_bit_range(MUST_BE_1001_BIT_RANGE) != 0b1001 {
        return None;
    }

    let mul_opcode_value = opcode.get_bit_range(MUL_OPCODE_BIT_RANGE);
    let operation_type = match mul_opcode_value {
        0b0000 => MultiplyOperation::Mul,
        0b0001 => MultiplyOperation::Mla,
        0b0010 => MultiplyOperation::Umaal,
        0b0100 => MultiplyOperation::Umull,
        0b0101 => MultiplyOperation::Umlal,
        0b0110 => MultiplyOperation::Smull,
        0b0111 => MultiplyOperation::Smlal,
        _ => return None,
    };

    let set_condition_codes_bit = opcode.get_bit(SET_CONDITION_CODES_BIT_INDEX);
    let destination_register = get_register_at_offset(opcode, DESTINATION_REGISTER_OFFSET);
    let accumulate_register = get_register_at_offset(opcode, ACCUMULATE_REGISTER_OFFSET);
    let operand_rs = get_register_at_offset(opcode, OPERAND_REGISTER_RS_OFFSET);
    let operand_rm = get_register_at_offset(opcode, OPERAND_REGISTER_RM_OFFSET);

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
    None.or_else(|| try_decode_arm_mrs(opcode))
        .or_else(|| try_decode_arm_msr(opcode))
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

    let destination_register = get_register_at_offset(opcode, DEST_REGISTER_OFFSET);

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

        let source_register = get_register_at_offset(opcode, SOURCE_REGISTER_OFFSET);

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
    None.or_else(|| try_decode_arm_basic_single_data_transfer(opcode))
        .or_else(|| try_decode_arm_special_single_data_transfer(opcode))
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

        let base_register = get_register_at_offset(opcode, BASE_REGISTER_OFFSET);
        let source_destination_register =
            get_register_at_offset(opcode, SOURCE_DEST_REGISTER_OFFSET);

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
            let shift_type = get_shift_type(opcode);
            let offset_register = get_register_at_offset(opcode, OFFSET_REGISTER_OFFSET);

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
    let write_back_memory_management_flag = opcode.get_bit(WRITE_BACK_MEMORY_MANAGEMENT_BIT_INDEX);
    let load_store_flag = opcode.get_bit(LOAD_STORE_BIT_INDEX);
    let base_register = get_register_at_offset(opcode, BASE_REGISTER_OFFSET);
    let source_dest_register = get_register_at_offset(opcode, SOURCE_DEST_REGISTER_OFFSET);
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

        let base_register = get_register_at_offset(opcode, BASE_REGISTER_OFFSET);

        let register_list_raw = opcode.get_bit_range(REGISTER_LIST_BIT_RANGE);

        let mut register_bit_list = [false; 16];
        for (register_idx, register_bit) in register_bit_list.iter_mut().enumerate() {
            let register_mask = 1 << register_idx;
            let register_used = (register_list_raw & register_mask) == register_mask;
            *register_bit = register_used;
        }

        Some(match access_type {
            BlockDataTransferType::Stm => ArmInstructionType::Stm {
                index_type,
                offset_modifier,
                write_back,
                base_register,
                register_bit_list,
                force_user_mode: psr_force_user_bit,
            },
            BlockDataTransferType::Ldm => ArmInstructionType::Ldm {
                index_type,
                offset_modifier,
                write_back,
                base_register,
                register_bit_list,
                force_user_mode: psr_force_user_bit,
            },
        })
    } else {
        None
    }
}

fn try_decode_arm_single_data_swap(opcode: u32) -> Option<ArmInstructionType> {
    const MUST_BE_00010_BIT_RANGE: RangeInclusive<usize> = 23..=27;
    const BYTE_WORD_BIT_INDEX: usize = 22;
    const MUST_BE_00_BIT_RANGE: RangeInclusive<usize> = 20..=21;
    const BASE_REGISTER_OFFSET: usize = 16;
    const DEST_REGISTER_OFFSET: usize = 12;
    const MUST_BE_00001001_BIT_RANGE: RangeInclusive<usize> = 4..=11;
    const SOURCE_REGISTER_OFFSET: usize = 0;

    if opcode.get_bit_range(MUST_BE_00010_BIT_RANGE) != 0b00010 {
        return None;
    }

    if opcode.get_bit_range(MUST_BE_00_BIT_RANGE) != 0b00 {
        return None;
    }

    if opcode.get_bit_range(MUST_BE_00001001_BIT_RANGE) != 0b00001001 {
        return None;
    }

    let byte_word_bit_value = opcode.get_bit(BYTE_WORD_BIT_INDEX);
    let base_register = get_register_at_offset(opcode, BASE_REGISTER_OFFSET);
    let dest_register = get_register_at_offset(opcode, DEST_REGISTER_OFFSET);
    let source_register = get_register_at_offset(opcode, SOURCE_REGISTER_OFFSET);

    let access_size = if byte_word_bit_value {
        // swap 8bit/byte
        SwpAccessSize::Byte
    } else {
        // swap 32bit/word
        SwpAccessSize::Word
    };

    Some(ArmInstructionType::Swp {
        access_size,
        base_register,
        dest_register,
        source_register,
    })
}

impl Cpu {
    pub fn execute_arm(&mut self, instruction: ArmInstruction) {
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
                    force_user_mode,
                } => self.execute_arm_ldm(
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                    force_user_mode,
                ),
                ArmInstructionType::Stm {
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                    force_user_mode,
                } => self.execute_arm_stm(
                    index_type,
                    offset_modifier,
                    write_back,
                    base_register,
                    register_bit_list,
                    force_user_mode,
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
                ArmInstructionType::Swp {
                    access_size,
                    base_register,
                    dest_register,
                    source_register,
                } => {
                    self.execute_arm_swp(access_size, base_register, dest_register, source_register)
                }
                _ => todo!("{:#08x?}", instruction),
            }
        } else {
            // If instruction condition fails, we still need to increment to the next instruction.
            // This takes one cycle, and simply performs prefetch.
            let old_pc = self.read_register(Register::R15, |pc| pc);
            self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
            self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));
            self.write_register(old_pc + 4, Register::R15);
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
        // cycle 1: prefetch next instruction
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        // When using R15 as operand (Rm or Rn), the returned value depends on the instruction:
        //   - $+12 if I=0,R=1 (shift by register)
        //   - otherwise, $+8 (shift by immediate).
        //
        // Note that that pc = $ + 8 due to prefetch.
        let pc_operand_calculation = match second_operand {
            AluSecondOperandInfo::Register {
                shift_info: ArmRegisterOrImmediate::Register(_),
                ..
            } => {
                // TODO: This may possible be a merged IS cycle.
                self.bus.step(); // if shift by register, we take an extra I cycle to calculate this.
                |pc| pc + 4
            }
            _ => |pc| pc,
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
                    (final_signed_result, carry_1 ^ carry_2)
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
                    (signed_result, overflow_1 ^ overflow_2)
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
                    (signed_result, overflow_1 ^ overflow_2)
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
                let (unsigned_result, carry) =
                    first_operand_value.overflowing_add(second_operand_value);
                let (signed_result, overflow) =
                    (first_operand_value as i32).overflowing_add(second_operand_value as i32);

                (unsigned_result, carry, signed_result, overflow)
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
                self.write_register(saved_cpsr, Register::Cpsr);
            }
        }

        let do_write = matches!(
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
        );

        if do_write {
            self.write_register(unsigned_result, destination_operand);
            // If dest = r15, re-fill prefetch buffer.
            if matches!(destination_operand, Register::R15) {
                match self.get_instruction_mode() {
                    InstructionSet::Arm => {
                        self.pre_decode_arm =
                            Some(decode_arm(self.bus.fetch_arm_opcode(unsigned_result)));
                        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(unsigned_result + 4));

                        self.write_register(unsigned_result + 8, Register::R15);
                    }
                    InstructionSet::Thumb => {
                        self.pre_decode_thumb =
                            Some(decode_thumb(self.bus.fetch_thumb_opcode(unsigned_result)));
                        self.prefetch_opcode =
                            Some(u32::from(self.bus.fetch_thumb_opcode(unsigned_result + 2)));

                        self.write_register(unsigned_result + 4, Register::R15);
                    }
                }
            } else {
                self.write_register(old_pc + 4, Register::R15);
            }
        } else {
            self.write_register(old_pc + 4, Register::R15);
        }
    }

    // pc is already at $ + 8 because of prefetch.
    // documentation specifies that branch is to ($ + offset + 8).
    fn execute_arm_b(&mut self, offset: i32) {
        let old_pc = self.read_register(Register::R15, |pc| pc);

        // cycle 1
        // pre-fetch still occurs, but we won't bother storing it anywhere or performing decode.
        self.bus.fetch_arm_opcode(old_pc);

        // cycle 2
        let new_pc = old_pc.wrapping_add(offset as u32);
        self.write_register(new_pc + 8, Register::R15);
        self.pre_decode_arm = Some(decode_arm(self.bus.fetch_arm_opcode(new_pc)));

        // cycle 3
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(new_pc + 4));
    }

    // PC is already at $ + 8 because of prefetch.
    // documentation specifies that branch is to ($ + offset + 8).
    // save ($ + 4) in lr.
    fn execute_arm_bl(&mut self, offset: i32) {
        let old_pc = self.read_register(Register::R15, |pc| pc);

        // cycle 1
        // pre-fetch still occurs, but we won't bother storing it anywhere or performing decode.
        self.bus.fetch_arm_opcode(old_pc);

        // cycle 2
        self.write_register(old_pc - 4, Register::R14);
        let new_pc = old_pc.wrapping_add(offset as u32);
        self.write_register(new_pc + 8, Register::R15);
        self.pre_decode_arm = Some(decode_arm(self.bus.fetch_arm_opcode(new_pc)));

        // cycle 3
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(new_pc + 4));
    }

    // PC = operand, T = Rn.0
    fn execute_arm_bx(&mut self, operand: Register) {
        const NEW_STATE_BIT_INDEX: usize = 0;

        let old_pc = self.read_register(Register::R15, |pc| pc);

        // cycle 1
        // pre-fetch still occurs, but we won't bother storing it anywhere or performing decode.
        self.bus.fetch_arm_opcode(old_pc);
        let operand_value = self.read_register(operand, |_| todo!());

        // cycle 2
        let new_state_bit = operand_value.get_bit(NEW_STATE_BIT_INDEX);
        self.set_cpu_state_bit(new_state_bit);

        let new_pc = operand_value & (!1);

        // conditional on new instruction mode
        match self.get_instruction_mode() {
            InstructionSet::Arm => {
                // still cycle 2
                self.pre_decode_arm = Some(decode_arm(self.bus.fetch_arm_opcode(new_pc)));

                // cycle 3
                self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(new_pc + 4));

                self.write_register(new_pc + 8, Register::R15);
            }
            InstructionSet::Thumb => {
                // still cycle 2
                self.pre_decode_thumb = Some(decode_thumb(self.bus.fetch_thumb_opcode(new_pc)));

                // cycle 3
                self.prefetch_opcode = Some(u32::from(self.bus.fetch_thumb_opcode(new_pc + 2)));

                self.write_register(new_pc + 4, Register::R15);
            }
        };
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
        // cycle 1: prefetch next instruction
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        const FLAGS_FIELD_MASK: u32 = 0b11111111_00000000_00000000_00000000;
        const STATUS_FIELD_MASK: u32 = 0b00000000_11111111_00000000_00000000;
        const EXTENSION_FIELD_MASK: u32 = 0b00000000_00000000_11111111_00000000;
        const CONTROL_FIELD_MASK: u32 = 0b00000000_00000000_00000000_11111111;

        let source_value = match source_info {
            MsrSourceInfo::Immediate { value } => value,
            MsrSourceInfo::Register(register) => self.read_register(register, |_| unreachable!()),
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

        let original_psr_value = self.read_register(psr_register, |_| unreachable!());
        let new_psr_value = (source_value & write_mask) | (original_psr_value & (!write_mask));

        // write back is merged into next instruction's cycle
        self.write_register(new_psr_value, psr_register);

        self.write_register(old_pc + 4, Register::R15);
    }

    fn execute_arm_mrs(&mut self, destination_register: Register, source_psr: PsrTransferPsr) {
        // cycle 1: prefetch next instruction
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        let source_psr_value = match source_psr {
            PsrTransferPsr::Cpsr => self.read_register(Register::Cpsr, |_| unreachable!()),
            PsrTransferPsr::Spsr => self.read_register(Register::Spsr, |_| unreachable!()),
        };

        // write back is merged into next instruction's cycle
        self.write_register(source_psr_value, destination_register);

        self.write_register(old_pc + 4, Register::R15);
    }

    fn execute_arm_str(
        &mut self,
        access_size: SingleDataMemoryAccessSize,
        base_register: Register,
        index_type: SingleDataTransferIndexType,
        offset_info: SingleDataTransferOffsetInfo,
        source_register: Register,
    ) {
        // cycle 1: perform address calculation (and do prefetch)
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        // "including R15=PC+8".
        //  Note that this is already the case due to prefetch (R15 = $ + 8).
        let base_address = self.read_register(base_register, |pc| pc);

        let offset_amount = match offset_info.value {
            SingleDataTransferOffsetValue::Immediate { offset } => offset,
            SingleDataTransferOffsetValue::Register { offset_register } => {
                self.read_register(offset_register, |_| unreachable!())
            }
            SingleDataTransferOffsetValue::RegisterImmediate {
                offset_register,
                shift_amount,
                shift_type,
            } => {
                assert!(!matches!(offset_register, Register::R15));

                let offset_register_value = self.read_register(offset_register, |_| unreachable!());
                shift_type.evaluate(offset_register_value, shift_amount)
            }
        };

        let offset_address = if offset_info.sign {
            base_address.wrapping_sub(offset_amount)
        } else {
            base_address.wrapping_add(offset_amount)
        };

        // "including R15=PC+12"
        //  Note that R15 = $ + 8 due to prefetch.
        //
        // ensure that we read value before doing any possible write-back, in
        // case source value and write-back register are the same.
        let value = self.read_register(source_register, |pc| pc + 4);

        // cycle 2: perform base modification and store register at memory address.
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

        match access_size {
            SingleDataMemoryAccessSize::Byte => self.bus.write_byte_address(
                value as u8,
                actual_address,
                BusAccessType::NonSequential,
            ),
            SingleDataMemoryAccessSize::HalfWord => self.bus.write_halfword_address(
                value as u16,
                actual_address,
                BusAccessType::NonSequential,
            ),
            SingleDataMemoryAccessSize::Word => {
                self.bus
                    .write_word_address(value, actual_address, BusAccessType::NonSequential)
            }
            _ => todo!("{:?}", access_size),
        };

        self.write_register(old_pc + 4, Register::R15);
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
        // cycle 1: perform address calculation (and do prefetch)
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        // "including R15=PC+8"
        //  Note that this is already the case due to prefetch (R15 = $ + 8).
        let base_address = self.read_register(base_register, |pc| pc);

        let offset_amount = match offset_info.value {
            SingleDataTransferOffsetValue::Immediate { offset } => offset,
            SingleDataTransferOffsetValue::Register { offset_register } => {
                self.read_register(offset_register, |_| unreachable!())
            }
            SingleDataTransferOffsetValue::RegisterImmediate {
                offset_register,
                shift_amount,
                shift_type,
            } => {
                let offset_register_value = self.read_register(offset_register, |_| unreachable!());
                match shift_type {
                    ShiftType::Lsl => {
                        if shift_amount == 0 {
                            offset_register_value
                        } else {
                            ShiftType::Lsl.evaluate(offset_register_value, shift_amount)
                        }
                    }
                    ShiftType::Lsr => {
                        if shift_amount == 0 {
                            0
                        } else {
                            ShiftType::Lsr.evaluate(offset_register_value, shift_amount)
                        }
                    }
                    ShiftType::Asr => {
                        if shift_amount == 0 {
                            let sign = offset_register_value.get_bit(31);
                            if sign {
                                !0
                            } else {
                                0
                            }
                        } else {
                            ShiftType::Asr.evaluate(offset_register_value, shift_amount)
                        }
                    }
                    ShiftType::Ror => {
                        if shift_amount == 0 {
                            let carry_in = self.get_carry_flag();
                            ShiftType::Ror
                                .evaluate(offset_register_value, 1)
                                .set_bit(31, carry_in)
                        } else {
                            ShiftType::Ror.evaluate(offset_register_value, shift_amount)
                        }
                    }
                }
            }
        };

        let offset_address = if offset_info.sign {
            base_address.wrapping_sub(offset_amount)
        } else {
            base_address.wrapping_add(offset_amount)
        };

        // cycle 2: fetch data from memory and adjust base register if necessary.
        let (data_read_address, base_modified) = match index_type {
            SingleDataTransferIndexType::PostIndex { .. } => {
                // post index always has write-back
                self.write_register(offset_address, base_register);
                (base_address, true)
            }
            SingleDataTransferIndexType::PreIndex { write_back } => {
                if write_back {
                    self.write_register(offset_address, base_register);
                }

                (offset_address, write_back)
            }
        };

        let value = match (access_size, sign_extend) {
            (SingleDataMemoryAccessSize::Byte, false) => self
                .bus
                .read_byte_address(data_read_address, BusAccessType::NonSequential)
                as u32,
            (SingleDataMemoryAccessSize::Byte, true) => self
                .bus
                .read_byte_address(data_read_address, BusAccessType::NonSequential)
                as i8 as i32 as u32,
            (SingleDataMemoryAccessSize::HalfWord, false) => {
                let rotation = (data_read_address & 0b1) * 8;
                u32::from(
                    self.bus
                        .read_halfword_address(data_read_address, BusAccessType::NonSequential),
                )
                .rotate_right(rotation)
            }
            (SingleDataMemoryAccessSize::HalfWord, true) => {
                // LDRSH Rd,[odd]  -->  LDRSB Rd,[odd]         ;sign-expand BYTE value
                let hword_aligned = data_read_address & 1 == 0;

                if hword_aligned {
                    self.bus
                        .read_halfword_address(data_read_address, BusAccessType::NonSequential)
                        as i16 as i32 as u32
                } else {
                    self.bus
                        .read_byte_address(data_read_address, BusAccessType::NonSequential)
                        as i8 as i32 as u32
                }
            }
            (SingleDataMemoryAccessSize::Word, false) => {
                let rotation = (data_read_address & 0b11) * 8;
                self.bus
                    .read_word_address(data_read_address, BusAccessType::NonSequential)
                    .rotate_right(rotation)
            }
            (SingleDataMemoryAccessSize::Word, true) => unreachable!(),
            _ => todo!("{:?} sign extend: {}", access_size, sign_extend),
        };

        // third cycle: store result in destination register.
        // TODO: This may possible a merged IS cycle.
        self.write_register(value, destination_register);
        self.bus.step();

        // if R15 is affected by this instruciton, add cycles to refill prefetch.
        let r15_modified = matches!(destination_register, Register::R15)
            || (base_modified && matches!(base_register, Register::R15));

        if r15_modified {
            let new_pc = self.read_register(Register::R15, |pc| pc);

            // cycle 4: prefetch next instruction (decode)
            self.pre_decode_arm = Some(decode_arm(self.bus.fetch_arm_opcode(new_pc)));

            // cycle 5: prefetch 2 instructions out (fetch)
            self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(new_pc + 4));

            // patch PC to correctly point to "most recently prefetched instruction"
            self.write_register(new_pc + 8, Register::R15);
        } else {
            self.write_register(old_pc + 4, Register::R15);
        }
    }

    fn execute_arm_ldm(
        &mut self,
        index_type: BlockDataTransferIndexType,
        offset_modifier: OffsetModifierType,
        write_back: bool,
        base_register: Register,
        register_bit_list: [bool; 16],
        force_user_mode: bool,
    ) {
        // cycle 1: perform address calculation (and do prefetch)
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        fn write_register_user_mode(cpu: &mut Cpu, value: u32, register: Register) {
            let old_mode = cpu.get_cpu_mode();
            cpu.set_cpu_mode(super::CpuMode::User);
            cpu.write_register(value, register);
            cpu.set_cpu_mode(old_mode);
        }

        let empty_rlist = register_bit_list.into_iter().all(|val| !val);

        // "not including R15".
        let mut current_address = self.read_register(base_register, |_| unreachable!());

        let mut r15_written = false;

        // cycles 1-n: read data
        match offset_modifier {
            OffsetModifierType::AddToBase => {
                for (register_idx, register_loaded) in register_bit_list.into_iter().enumerate() {
                    if register_loaded {
                        if matches!(index_type, BlockDataTransferIndexType::PreIndex) {
                            current_address += 4;
                        }

                        // The mis-aligned low bit(s) are ignored, the memory access goes to a forcibly aligned (rounded-down) memory address.
                        let value = self
                            .bus
                            .read_word_address(current_address, BusAccessType::NonSequential);
                        let register = Register::from_index(register_idx as u32);

                        r15_written |= matches!(register, Register::R15);

                        if force_user_mode {
                            write_register_user_mode(self, value, register);
                        } else {
                            self.write_register(value, register);
                        };

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

                        // The mis-aligned low bit(s) are ignored, the memory access goes to a forcibly aligned (rounded-down) memory address.
                        let value = self
                            .bus
                            .read_word_address(current_address, BusAccessType::NonSequential);
                        let register = Register::from_index(register_idx as u32);

                        r15_written |= matches!(register, Register::R15);

                        if force_user_mode {
                            write_register_user_mode(self, value, register);
                        } else {
                            self.write_register(value, register);
                        };

                        if matches!(index_type, BlockDataTransferIndexType::PostIndex) {
                            current_address -= 4;
                        }
                    }
                }
            }
        }

        if empty_rlist {
            if matches!(index_type, BlockDataTransferIndexType::PreIndex) {
                match offset_modifier {
                    OffsetModifierType::AddToBase => current_address += 0x40,
                    OffsetModifierType::SubtractFromBase => current_address -= 0x40,
                };
            }

            // The mis-aligned low bit(s) are ignored, the memory access goes to a forcibly aligned (rounded-down) memory address.
            let value = self
                .bus
                .read_word_address(current_address, BusAccessType::NonSequential);
            let register = Register::R15;

            r15_written |= true;

            if force_user_mode {
                write_register_user_mode(self, value, register);
            } else {
                self.write_register(value, register);
            };

            if matches!(index_type, BlockDataTransferIndexType::PostIndex) {
                match offset_modifier {
                    OffsetModifierType::AddToBase => current_address += 0x40,
                    OffsetModifierType::SubtractFromBase => current_address -= 0x40,
                };
            }
        }

        let base_in_rlist = register_bit_list
            .into_iter()
            .enumerate()
            .filter_map(|(register_idx, register_loaded)| {
                register_loaded.then(|| Register::from_index(register_idx as u32))
            })
            .any(|loaded_register| {
                std::mem::discriminant(&loaded_register) == std::mem::discriminant(&base_register)
            });

        // Writeback with Rb included in Rlist: no writeback (LDM/ARMv4).
        if !base_in_rlist && write_back {
            self.write_register(current_address, base_register);
        }

        // cycles 2-(1+n): I cycle: write result into register.

        // Write final register back.
        // TODO: This may possibly be a merged IS cycle.
        self.bus.step();

        if r15_written {
            let new_pc = self.read_register(Register::R15, |pc| pc);
            self.pre_decode_arm = Some(decode_arm(self.bus.fetch_arm_opcode(new_pc)));
            self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(new_pc + 4));
            self.write_register(new_pc + 8, Register::R15);
        } else {
            self.write_register(old_pc + 4, Register::R15);
        }
    }

    fn execute_arm_stm(
        &mut self,
        index_type: BlockDataTransferIndexType,
        offset_modifier: OffsetModifierType,
        write_back: bool,
        base_register: Register,
        register_bit_list: [bool; 16],
        force_user_mode: bool,
    ) {
        // cycle 1: perform address calculation (and do prefetch)
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        enum IncrementTiming {
            BeforeWrite,
            AfterWrite,
        }

        let raw_registers = register_bit_list
            .into_iter()
            .enumerate()
            .filter_map(|(register_idx, register_loaded)| {
                register_loaded.then(|| Register::from_index(register_idx as u32))
            })
            .collect::<Vec<_>>();
        let read_register_pc_calculation = |pc| pc + 4;

        // "not including R15".
        let base_address = self.read_register(base_register, |_| unreachable!());

        // number of non-zero registers
        //
        // empty rlist behaves as if we had _all_ registers when performing offset calculations.
        // Empty Rlist: R15 loaded/stored (ARMv4 only), and Rb=Rb+/-40h (ARMv4-v5).
        let (num_registers, stored_registers) = if raw_registers.is_empty() {
            (16, vec![Register::R15])
        } else {
            (raw_registers.len() as u32, raw_registers)
        };

        let increment_timing = match (offset_modifier, index_type) {
            (OffsetModifierType::AddToBase, BlockDataTransferIndexType::PreIndex)
            | (OffsetModifierType::SubtractFromBase, BlockDataTransferIndexType::PostIndex) => {
                IncrementTiming::BeforeWrite
            }
            (OffsetModifierType::AddToBase, BlockDataTransferIndexType::PostIndex)
            | (OffsetModifierType::SubtractFromBase, BlockDataTransferIndexType::PreIndex) => {
                IncrementTiming::AfterWrite
            }
        };

        let new_base = match offset_modifier {
            OffsetModifierType::AddToBase => base_address + (4 * num_registers),
            OffsetModifierType::SubtractFromBase => base_address - (4 * num_registers),
        };

        // we store registers from low to high address no matter what, but the final value of "current_address" does
        // not represent the true write-back address.
        let mut current_address = match offset_modifier {
            OffsetModifierType::AddToBase => base_address,
            OffsetModifierType::SubtractFromBase => base_address - (4 * num_registers),
        };

        // Writeback with Rb included in Rlist: Store OLD base if Rb is FIRST entry in Rlist, otherwise store NEW base
        let base_value_if_read = if stored_registers.first() == Some(&base_register) {
            base_address
        } else {
            new_base
        };

        for register in stored_registers {
            if matches!(increment_timing, IncrementTiming::BeforeWrite) {
                current_address += 4;
            }

            let register_value = if register == base_register {
                base_value_if_read
            } else if force_user_mode {
                self.read_user_register(register, read_register_pc_calculation)
            } else {
                self.read_register(register, read_register_pc_calculation)
            };

            self.bus.write_word_address(
                register_value,
                current_address,
                BusAccessType::NonSequential,
            );

            if matches!(increment_timing, IncrementTiming::AfterWrite) {
                current_address += 4;
            }
        }

        if write_back {
            self.write_register(new_base, base_register);
        }

        self.write_register(old_pc + 4, Register::R15);
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
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

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

        for _ in 0..4 {
            self.bus.step();
        }
        self.write_register(old_pc + 4, Register::R15);
    }

    fn execute_arm_swp(
        &mut self,
        access_size: SwpAccessSize,
        base_register: Register,
        dest_register: Register,
        source_register: Register,
    ) {
        let old_pc = self.read_register(Register::R15, |pc| pc);
        self.pre_decode_arm = self.prefetch_opcode.map(decode_arm);
        self.prefetch_opcode = Some(self.bus.fetch_arm_opcode(old_pc));

        let base_address = self.read_register(base_register, |_| unreachable!());
        match access_size {
            SwpAccessSize::Byte => {
                let old_base_value = self
                    .bus
                    .read_byte_address(base_address, BusAccessType::NonSequential);
                let new_base_value = self.read_register(source_register, |_| unreachable!()) as u8;

                self.write_register(u32::from(old_base_value), dest_register);
                self.bus.write_byte_address(
                    new_base_value,
                    base_address,
                    BusAccessType::NonSequential,
                );
            }
            SwpAccessSize::Word => {
                let rotate = (base_address & 0b11) * 8;
                let old_base_value = self
                    .bus
                    .read_word_address(base_address, BusAccessType::NonSequential)
                    .rotate_right(rotate);
                let new_base_value = self.read_register(source_register, |_| unreachable!());

                self.write_register(old_base_value, dest_register);
                self.bus.write_word_address(
                    new_base_value,
                    base_address,
                    BusAccessType::NonSequential,
                );
            }
        }

        self.write_register(old_pc + 4, Register::R15);
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
                force_user_mode,
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
                if force_user_mode {
                    f.write_str("^")?;
                }

                Ok(())
            }
            ArmInstructionType::Ldm {
                base_register,
                index_type,
                offset_modifier,
                register_bit_list,
                write_back,
                force_user_mode,
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
                if force_user_mode {
                    f.write_str("^")?;
                }

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
                MultiplyOperation::Umaal => write!(f, "umaal TODO"),
            },
            ArmInstructionType::Swi { comment } => write!(f, "swi #{}", comment),
            ArmInstructionType::Blx { .. } => todo!("display blx"),
            ArmInstructionType::Swp {
                access_size,
                base_register,
                dest_register,
                source_register,
            } => {
                f.write_str("swp")?;
                match access_size {
                    SwpAccessSize::Byte => f.write_str("b")?,
                    SwpAccessSize::Word => {}
                };

                write!(
                    f,
                    " {}, {}, [{}]",
                    dest_register, source_register, base_register
                )?;
                Ok(())
            }
            ArmInstructionType::Invalid { opcode } => write!(f, "INVALID 0x{opcode:08X}"),
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
