use std::ops::RangeInclusive;

use crate::{BitManipulation, DataAccess};

#[derive(Clone, Copy, Debug)]
enum PrescalerInterval {
    Div1,
    Div64,
    Div256,
    Div1024,
}

#[derive(Clone, Debug)]
pub struct Timer {
    tick: u64,

    counter: u16,
    reload: u16,
    control: u16,

    startup_delay: bool,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            tick: 0,
            counter: 0,
            reload: 0,
            control: 0,

            startup_delay: false,
        }
    }
}

impl Timer {
    pub fn step(&mut self, previous_overflow: bool) -> bool {
        // if timer disabled, don't handle any counting logic.
        if !self.get_timer_start_stop() {
            return false;
        }

        if self.startup_delay {
            self.startup_delay = false;
            return false;
        }

        let increment = if self.get_count_up_timing() {
            previous_overflow
        } else {
            let increment_mask = match self.get_prescaler_interval() {
                PrescalerInterval::Div1 => 0x0,
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
        T: Copy,
    {
        self.reload = self.reload.set_data(value, index);
    }

    // Note: When simultaneously changing the start bit from 0 to 1, and setting the reload value at the same time
    // (by a single 32bit I/O operation), then the newly written reload value is recognized as new counter value.
    //
    // Here, we special-case this scenario to let the bus abstract this logic away.
    pub fn write_timer_counter_reload_word(&mut self, value: u32) {
        let new_counter_reload = value as u16;
        let new_timer_control = (value >> 16) as u16;

        // ensure that write to control happens second to ensure the newly written reload value is loaded, if applicable.
        self.write_timer_counter_reload(new_counter_reload, 0);
        self.write_timer_control(new_timer_control, 0);
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
        T: Copy,
    {
        const COUNT_UP_TIMING_BIT_INDEX: usize = 2;
        const TIMER_IRQ_ENABLE_BIT_INDEX: usize = 6;
        const TIMER_START_STOP_BIT_INDEX: usize = 7;

        let old_start_bit = self.get_timer_start_stop();

        self.control = self.control.set_data(value, index);

        let new_start_bit = self.get_timer_start_stop();

        // The reload value is copied into the counter only upon following two situations:
        // - Automatically upon timer overflows
        // - When the timer start bit becomes changed from 0 to 1. (handled here)
        if !old_start_bit && new_start_bit {
            self.counter = self.reload;
            self.startup_delay = true;
        }
    }
}

impl Timer {
    fn get_prescaler_interval(&self) -> PrescalerInterval {
        const PRESCALER_SELECTION_BIT_RANGE: RangeInclusive<usize> = 0..=1;

        match self.control.get_bit_range(PRESCALER_SELECTION_BIT_RANGE) {
            0 => PrescalerInterval::Div1,
            1 => PrescalerInterval::Div64,
            2 => PrescalerInterval::Div256,
            3 => PrescalerInterval::Div1024,
            _ => unreachable!(),
        }
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

// Public debugging interface
impl Timer {
    pub fn get_current_counter(&self) -> u16 {
        self.counter
    }

    pub fn get_current_reload(&self) -> u16 {
        self.reload
    }
}
