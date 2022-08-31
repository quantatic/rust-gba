use std::ops::RangeInclusive;

use crate::{BitManipulation, DataAccess};

#[derive(Clone, Copy, Debug)]
enum PrescalerInterval {
    Div1,
    Div64,
    Div256,
    Div1024,
}

#[derive(Debug)]
pub struct Timer {
    tick: u64,
    counter: u16,
    reload: u16,
    control: u16,
    prescaler_interval: PrescalerInterval,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            tick: 0,
            counter: 0,
            reload: 0,
            control: 0,
            prescaler_interval: PrescalerInterval::Div1,
        }
    }
}

impl Timer {
    pub fn step(&mut self, previous_overflow: bool) -> bool {
        // if timer disabled, don't handle any counting logic.
        if !self.get_timer_start_stop() {
            return false;
        }

        let increment = if self.get_count_up_timing() {
            previous_overflow
        } else {
            let increment_mask = match self.get_prescaler_interval() {
                PrescalerInterval::Div1 => 0x1,
                PrescalerInterval::Div64 => 0x3F,
                PrescalerInterval::Div256 => 0xFF,
                PrescalerInterval::Div1024 => 0x3FF,
            };

            (self.tick & increment_mask) == increment_mask
        };

        self.tick += 1;

        if increment {
            let (new_counter, overflow) = self.counter.overflowing_add(1);

            if overflow {
                self.counter = self.reload;
            } else {
                self.counter = new_counter;
            }

            overflow
        } else {
            false
        }
    }
}

impl Timer {
    pub fn read_timer_counter_reload<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.counter.get_data(index)
    }

    pub fn write_timer_counter_reload<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.reload = self.reload.set_data(value, index);
    }

    pub fn read_timer_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.control.get_data(index)
    }

    pub fn write_timer_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const PRESCALER_SELECTION_BIT_RANGE: RangeInclusive<usize> = 0..=1;
        const COUNT_UP_TIMING_BIT_INDEX: usize = 2;
        const TIMER_IRQ_ENABLE_BIT_INDEX: usize = 6;
        const TIMER_START_STOP_BIT_INDEX: usize = 7;

        self.control = self.control.set_data(value, index);
        self.prescaler_interval = match self.control.get_bit_range(PRESCALER_SELECTION_BIT_RANGE) {
            0 => PrescalerInterval::Div1,
            1 => PrescalerInterval::Div64,
            2 => PrescalerInterval::Div256,
            3 => PrescalerInterval::Div1024,
            _ => unreachable!(),
        };
    }
}

impl Timer {
    fn get_prescaler_interval(&self) -> PrescalerInterval {
        self.prescaler_interval
    }

    fn get_count_up_timing(&self) -> bool {
        const COUNT_UP_TIMING_BIT_INDEX: usize = 2;

        self.control.get_bit(COUNT_UP_TIMING_BIT_INDEX)
    }

    pub fn get_timer_irq_enable(&self) -> bool {
        const TIMER_IRQ_ENABLE_BIT_INDEX: usize = 6;

        self.control.get_bit(TIMER_IRQ_ENABLE_BIT_INDEX)
    }

    fn get_timer_start_stop(&self) -> bool {
        const TIMER_START_STOP_BIT_INDEX: usize = 7;

        self.control.get_bit(TIMER_START_STOP_BIT_INDEX)
    }
}
