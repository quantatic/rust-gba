use std::ops::RangeInclusive;

use crate::{BitManipulation, DataAccess};

#[derive(Clone, Copy, Debug)]
pub enum Key {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
    R,
    L,
}

#[derive(Debug)]
pub struct Keypad {
    key_status: u16, // 0 = pressed, 1 = released
    interrupt_control: u16,
}

impl Default for Keypad {
    fn default() -> Self {
        Self {
            key_status: 0xFF_FF,
            interrupt_control: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum IrqCondition {
    LogicalOr,
    LogicalAnd,
}

impl Keypad {
    const BUTTON_A_BIT_INDEX: usize = 0;
    const BUTTON_B_BIT_INDEX: usize = 1;
    const BUTTON_SELECT_BIT_INDEX: usize = 2;
    const BUTTON_START_BIT_INDEX: usize = 3;
    const BUTTON_RIGHT_BIT_INDEX: usize = 4;
    const BUTTON_LEFT_BIT_INDEX: usize = 5;
    const BUTTON_UP_BIT_INDEX: usize = 6;
    const BUTTON_DOWN_BIT_INDEX: usize = 7;
    const BUTTON_R_BIT_INDEX: usize = 8;
    const BUTTON_L_BIT_INDEX: usize = 9;
}

impl Keypad {
    pub fn set_pressed(&mut self, key: Key, pressed: bool) {
        let bit_index = match key {
            Key::A => Self::BUTTON_A_BIT_INDEX,
            Key::B => Self::BUTTON_B_BIT_INDEX,
            Key::Select => Self::BUTTON_SELECT_BIT_INDEX,
            Key::Start => Self::BUTTON_START_BIT_INDEX,
            Key::Right => Self::BUTTON_RIGHT_BIT_INDEX,
            Key::Left => Self::BUTTON_LEFT_BIT_INDEX,
            Key::Up => Self::BUTTON_UP_BIT_INDEX,
            Key::Down => Self::BUTTON_DOWN_BIT_INDEX,
            Key::R => Self::BUTTON_R_BIT_INDEX,
            Key::L => Self::BUTTON_L_BIT_INDEX,
        };

        self.key_status = self.key_status.set_bit(bit_index, !pressed);
    }
}

impl Keypad {
    pub fn read_key_status<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.key_status.get_data(index)
    }

    pub fn read_key_interrupt_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.interrupt_control.get_data(index)
    }

    pub fn write_key_interrupt_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.interrupt_control = self.interrupt_control.set_data(value, index);
        println!("key interrupt control: 0b{:016b}", self.interrupt_control);
    }

    pub fn poll_pending_interrupts(&mut self) -> bool {
        const IRQ_MASK_BIT_RANGE: RangeInclusive<usize> = 0..=9;

        if !self.get_irq_enabled() {
            return false;
        }

        // Keep in mind that 0 means pressed and 1 means released, so we must invert this bitmask.
        let pressed_bits = !self.key_status.get_bit_range(IRQ_MASK_BIT_RANGE);
        let irq_bits = self.interrupt_control.get_bit_range(IRQ_MASK_BIT_RANGE);

        match self.get_irq_condition() {
            // In logical OR mode, an interrupt is requested when at least one of the selected buttons is pressed.
            IrqCondition::LogicalOr => (pressed_bits & irq_bits) != 0,
            // In logical AND mode, an interrupt is requested when ALL of the selected buttons are pressed.
            IrqCondition::LogicalAnd => (pressed_bits & irq_bits) == irq_bits,
        }
    }
}

impl Keypad {
    fn get_irq_enabled(&self) -> bool {
        const IRQ_ENABLED_BIT_INDEX: usize = 14;

        self.interrupt_control.get_bit(IRQ_ENABLED_BIT_INDEX)
    }

    fn get_irq_condition(&self) -> IrqCondition {
        const IRQ_CONDITION_BIT_INDEX: usize = 15;

        if self.interrupt_control.get_bit(IRQ_CONDITION_BIT_INDEX) {
            IrqCondition::LogicalAnd
        } else {
            IrqCondition::LogicalOr
        }
    }
}
