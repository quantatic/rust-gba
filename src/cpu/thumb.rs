use std::{fmt::Display, ops::RangeInclusive};

use crate::BitManipulation;

use super::{Cpu, ExceptionType, InstructionCondition, Register, ShiftType};

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

impl Cpu {
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

#[derive(Clone, Copy, Debug)]
pub enum ThumbLoadStoreDataSize {
    Byte,
    HalfWord,
    Word,
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
    Swi {
        comment: u16,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ThumbInstruction {
    instruction_type: ThumbInstructionType,
    address: u32,
}

pub fn decode_thumb(opcode: u16, address: u32) -> ThumbInstruction {
    let maybe_instruction_type = None
        .or_else(|| try_decode_thumb_register_operation(opcode))
        .or_else(|| try_decode_thumb_memory_load_store(opcode))
        .or_else(|| try_decode_thumb_memory_addressing(opcode))
        .or_else(|| try_decode_thumb_memory_multiple_load_store(opcode))
        .or_else(|| try_decode_thumb_jump_call(opcode));

    let instruction_type = if let Some(instruction_type) = maybe_instruction_type {
        instruction_type
    } else {
        todo!("unrecognized Thumb opcode {:04X}", opcode)
    };

    ThumbInstruction {
        instruction_type,
        address,
    }
}

fn try_decode_thumb_register_operation(opcode: u16) -> Option<ThumbInstructionType> {
    None.or_else(|| try_decode_thumb_move_shifted_register(opcode))
        .or_else(|| try_decode_thumb_add_subtract(opcode))
        .or_else(|| try_decode_thumb_move_compare_add_subtract_immediate(opcode))
        .or_else(|| try_decode_thumb_alu_operations(opcode))
        .or_else(|| try_decode_thumb_high_register_operations_branch_exchange(opcode))
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
            let immediate_operand = u32::from(opcode.get_bit_range(IMMEDIATE_OPERAND_BIT_RANGE));
            let second_operand = ThumbRegisterOrImmediate::Immediate(immediate_operand);
            ThumbInstructionType::Register {
                destination_register: dest_register,
                operation: ThumbRegisterOperation::Add,
                source: source_register,
                second_operand: second_operand,
            }
        }
        SUB_IMMEDIATE_OPCODE_VALUE => {
            let immediate_operand = u32::from(opcode.get_bit_range(IMMEDIATE_OPERAND_BIT_RANGE));
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
    None.or_else(|| try_decode_thumb_load_pc_relative(opcode))
        .or_else(|| try_decode_thumb_load_store_register_offset(opcode))
        .or_else(|| try_decode_thumb_load_store_sign_extended_byte_halfword(opcode))
        .or_else(|| try_decode_thumb_load_store_immediate_offset(opcode))
        .or_else(|| try_decode_thumb_load_store_halfword(opcode))
        .or_else(|| try_decode_thumb_load_store_sp_relative(opcode))
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
    None.or_else(|| try_decode_thumb_get_relative_address(opcode))
        .or_else(|| try_decode_thumb_add_offset_stack_pointer(opcode))
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
    None.or_else(|| try_decode_thumb_push_pop_regs(opcode))
        .or_else(|| try_decode_thumb_multiple_load_store(opcode))
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
    None.or_else(|| try_decode_thumb_conditional_branch(opcode))
        .or_else(|| try_decode_thumb_unconditional_branch(opcode))
        .or_else(|| try_decode_thumb_long_branch_link_1(opcode))
        .or_else(|| try_decode_thumb_long_branch_link_2(opcode))
        .or_else(|| try_decode_thumb_swi(opcode))
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
        0xF => return None, // reserved for SWI
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

fn try_decode_thumb_swi(opcode: u16) -> Option<ThumbInstructionType> {
    const MUST_BE_11011111_BIT_RANGE: RangeInclusive<usize> = 8..=15;
    const COMMENT_BIT_RANGE: RangeInclusive<usize> = 0..=7;

    if opcode.get_bit_range(MUST_BE_11011111_BIT_RANGE) != 0b11011111 {
        return None;
    }

    let comment = opcode.get_bit_range(COMMENT_BIT_RANGE);

    Some(ThumbInstructionType::Swi { comment })
}

impl Cpu {
    pub fn execute_thumb(&mut self, instruction: ThumbInstruction) {
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
            ThumbInstructionType::Swi { comment: _ } => self.handle_exception(ExceptionType::Swi),
            _ => todo!("{:#016x?}", instruction),
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
                let hword_aligned = real_address & 1 == 0;

                if hword_aligned {
                    u32::from(self.bus.read_halfword_address(real_address))
                } else {
                    u32::from(self.bus.read_halfword_address(real_address - 1)).rotate_right(8)
                }
            }
            (ThumbLoadStoreDataSize::HalfWord, true) => {
                let hword_aligned = real_address & 1 == 0;

                if hword_aligned {
                    self.bus.read_halfword_address(real_address) as i16 as i32 as u32
                } else {
                    self.bus.read_byte_address(real_address) as i8 as i32 as u32
                }
            }
            (ThumbLoadStoreDataSize::Word, false) => {
                let rotate = (real_address & 0b11) * 8;
                let data_aligned = self.bus.read_word_address(real_address & (!0b11));
                data_aligned.rotate_right(rotate)
            }
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
            self.bus.write_word_address(lr_value, new_r13 & (!0b11));
        }

        for (register_idx, register_pushed) in register_bit_list.into_iter().enumerate().rev() {
            if register_pushed {
                let pushed_register = Register::from_index(register_idx as u32);
                let pushed_register_value = self.read_register(pushed_register, |_| unreachable!());

                let new_r13 = self.read_register(Register::R13, |_| unreachable!()) - 4;
                self.write_register(new_r13, Register::R13);
                self.bus
                    .write_word_address(pushed_register_value, new_r13 & (!0b11));
            }
        }
    }

    fn execute_thumb_pop(&mut self, register_bit_list: [bool; 8], pop_pc: bool) {
        for (register_idx, register_popped) in register_bit_list.into_iter().enumerate() {
            if register_popped {
                let popped_register = Register::from_index(register_idx as u32);
                let old_r13 = self.read_register(Register::R13, |_| unreachable!());
                let popped_register_value = self.bus.read_word_address(old_r13 & (!0b11));

                self.write_register(old_r13 + 4, Register::R13);
                self.write_register(popped_register_value, popped_register);
            }
        }

        if pop_pc {
            // POP {PC} ignores the least significant bit of the return address (processor remains in thumb state even if bit0 was cleared).
            let old_r13 = self.read_register(Register::R13, |_| unreachable!());
            let pc_value = self.bus.read_word_address(old_r13 & (!0b11)) & (!1);

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
            ThumbInstructionType::Swi { comment } => write!(f, "swi #{}", comment),
        }
    }
}
