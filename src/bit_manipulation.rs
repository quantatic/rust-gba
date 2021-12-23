use std::ops::RangeInclusive;

use crate::cpu::{InstructionCondition, Register, ShiftType};

pub trait BitManipulation {
    fn match_mask(self, mask: Self, result: Self) -> bool;

    fn get_condition(self) -> InstructionCondition;

    fn get_register_at_offset(self, offset: usize) -> Register;

    fn get_shift_type(self) -> ShiftType;

    fn get_bit(self, offset: usize) -> bool;

    fn set_bit(self, offset: usize, set: bool) -> Self;

    fn get_bit_range(self, bit_range: RangeInclusive<usize>) -> Self;

    fn set_bit_range(self, value: Self, bit_range: RangeInclusive<usize>) -> Self;
}

impl BitManipulation for u32 {
    fn match_mask(self, mask: Self, result: Self) -> bool {
        (self & mask) == result
    }

    fn get_condition(self) -> InstructionCondition {
        const CONDITION_SHIFT: usize = 28;
        const CONDITION_MASK: u32 = 0b1111 << CONDITION_SHIFT;

        match (self & CONDITION_MASK) >> CONDITION_SHIFT {
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

    fn get_register_at_offset(self, offset: usize) -> Register {
        let mask = 0b1111 << offset;
        let register_index = (self & mask) >> offset;
        Register::from_index(register_index)
    }

    fn get_shift_type(self) -> ShiftType {
        match self.get_bit_range(5..=6) {
            0 => ShiftType::Lsl,
            1 => ShiftType::Lsr,
            2 => ShiftType::Asr,
            3 => ShiftType::Ror,
            _ => unreachable!(),
        }
    }

    fn get_bit(self, offset: usize) -> bool {
        let mask = 1 << offset;
        (self & mask) == mask
    }

    fn set_bit(self, offset: usize, set: bool) -> Self {
        let mask = 1 << offset;
        if set {
            self | mask
        } else {
            self & !mask
        }
    }

    fn get_bit_range(self, bit_range: RangeInclusive<usize>) -> Self {
        if bit_range.is_empty() {
            return 0;
        }

        let shift = *bit_range.start();
        let num_ones = bit_range.end() - bit_range.start() + 1;
        let mask = 2u32.wrapping_pow(num_ones as u32).wrapping_sub(1) << shift;
        (self & mask) >> shift
    }

    fn set_bit_range(self, value: Self, bit_range: RangeInclusive<usize>) -> Self {
        if bit_range.is_empty() {
            return self;
        }

        let shift = *bit_range.start();
        let num_ones = bit_range.end() - bit_range.start() + 1;
        let mask = 2u32.wrapping_pow(num_ones as u32).wrapping_sub(1) << shift;
        (value & mask) | (self & (!mask))
    }
}

impl BitManipulation for u16 {
    fn match_mask(self, mask: Self, result: Self) -> bool {
        (self & mask) == result
    }

    fn get_condition(self) -> InstructionCondition {
        unreachable!()
    }

    fn get_register_at_offset(self, offset: usize) -> Register {
        let mask = 0b111 << offset;
        let register_index = (self & mask) >> offset;
        Register::from_index(u32::from(register_index))
    }

    fn get_shift_type(self) -> ShiftType {
        unreachable!()
    }

    fn get_bit(self, offset: usize) -> bool {
        let mask = 1 << offset;
        (self & mask) == mask
    }

    fn set_bit(self, offset: usize, set: bool) -> Self {
        let mask = 1 << offset;
        if set {
            self | mask
        } else {
            self & !mask
        }
    }

    fn get_bit_range(self, bit_range: RangeInclusive<usize>) -> Self {
        if bit_range.is_empty() {
            return 0;
        }

        let shift = *bit_range.start();
        let num_ones = bit_range.end() - bit_range.start() + 1;
        let mask = 2u16.wrapping_pow(num_ones as u32).wrapping_sub(1) << shift;
        (self & mask) >> shift
    }

    fn set_bit_range(self, value: Self, bit_range: RangeInclusive<usize>) -> Self {
        if bit_range.is_empty() {
            return self;
        }

        let shift = *bit_range.start();
        let num_ones = bit_range.end() - bit_range.start() + 1;
        let mask = 2u16.wrapping_pow(num_ones as u32).wrapping_sub(1) << shift;
        (value & mask) | (self & (!mask))
    }
}
