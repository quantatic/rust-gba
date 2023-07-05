use std::{collections::btree_map::Range, ops::RangeInclusive};

use crate::{bit_manipulation::BitManipulation, data_access::DataAccess, CYCLES_PER_SECOND};

// Clocks per second
const SEQUENCER_CLOCK_FREQUENCY: u64 = 512;

// CPU cycles per clock
const SEQUENCER_CLOCK_PERIOD: u64 = CYCLES_PER_SECOND / SEQUENCER_CLOCK_FREQUENCY;

const LENGTH_COUNTER_CLOCKS: [bool; 8] = [true, false, true, false, true, false, true, false];
const VOLUME_ENVELOPE_CLOCKS: [bool; 8] = [false, false, false, false, false, false, false, true];
const SWEEP_CLOCKS: [bool; 8] = [false, false, true, false, false, false, true, false];

#[derive(Clone, Debug)]
enum SweepBehavior {
    FrequencyIncrease,
    FrequencyDecrease,
}

#[derive(Clone, Debug)]
enum EnvelopeBehavior {
    VolumeIncrease,
    VolumeDecrease,
}

#[derive(Clone, Debug, Default)]
pub struct ToneAndSweep {
    sweep_register: u16,
    duty_length_envelope: u16,
    frequency_control: u16,

    frequency_sweep_enabled: bool,

    length_counter: u8,

    frame_sequencer_idx: u16,
    clock: u64,

    wave_duty_index: u8,
    wave_duty_timer_ticks_left: u16,
    envelope_ticks_left: u8,
    sweep_ticks_left: u8,

    enabled: bool,
    volume: u8,

    frequency_shadow: u16,
}

impl ToneAndSweep {
    pub fn step(&mut self) {
        if self.clock % SEQUENCER_CLOCK_PERIOD == 0 {
            if LENGTH_COUNTER_CLOCKS[usize::from(self.frame_sequencer_idx)] {
                if self.get_length_flag() {
                    todo!("Validate this functionality");
                    self.length_counter = self.length_counter.saturating_sub(1);

                    if self.length_counter == 0 {
                        self.enabled = false;
                    }
                }
            }

            if VOLUME_ENVELOPE_CLOCKS[usize::from(self.frame_sequencer_idx)] {
                self.envelope_ticks_left = self.envelope_ticks_left.saturating_sub(1);

                if self.envelope_ticks_left == 0 {
                    if self.get_envelope_sweep_period() != 0 {
                        match self.get_envelope_direction() {
                            EnvelopeBehavior::VolumeIncrease => {
                                self.volume = u8::min(self.volume + 1, 0xF)
                            }
                            EnvelopeBehavior::VolumeDecrease => {
                                self.volume = self.volume.saturating_sub(1)
                            }
                        }
                    }

                    self.envelope_ticks_left = if self.get_envelope_sweep_period() == 0 {
                        8
                    } else {
                        self.get_envelope_sweep_period()
                    }
                }
            }

            if SWEEP_CLOCKS[usize::from(self.frame_sequencer_idx)] {
                self.sweep_ticks_left = self.sweep_ticks_left.saturating_sub(1);
                if self.sweep_ticks_left == 0 {
                    if self.frequency_sweep_enabled && self.get_sweep_period() != 0 {
                        let new_frequency = match self.get_sweep_behavior() {
                            SweepBehavior::FrequencyIncrease => {
                                self.frequency_shadow
                                    + (self.frequency_shadow >> self.get_sweep_shift())
                            }
                            SweepBehavior::FrequencyDecrease => {
                                self.frequency_shadow
                                    - (self.frequency_shadow >> self.get_sweep_shift())
                            }
                        };

                        // The sweep timer is clocked at 128 Hz by the frame sequencer. When it generates a clock and the sweep's
                        // internal enabled flag is set and the sweep period is not zero, a new frequency is calculated and the overflow check is performed.
                        //
                        // If the new frequency is 2047 or less and the sweep shift is not zero, this new frequency is written
                        // back to the shadow frequency and square 1's frequency in NR13 and NR14, then frequency calculation and
                        // overflow check are run AGAIN immediately using this new value, but this second new frequency is not written back.
                        if self.get_sweep_shift() != 0 {
                            if new_frequency > 2047 {
                                self.enabled = false;
                            }

                            self.frequency_shadow = new_frequency;
                            self.set_frequency(new_frequency);

                            // If the new frequency is 2047 or less and the sweep shift is not zero, this new frequency is written back to
                            // the shadow frequency and square 1's frequency in NR13 and NR14, then frequency calculation and overflow check
                            // are run AGAIN immediately using this new value, but this second new frequency is not written back.
                            let test_frequency = match self.get_sweep_behavior() {
                                SweepBehavior::FrequencyIncrease => {
                                    self.frequency_shadow
                                        + (self.frequency_shadow >> self.get_sweep_shift())
                                }
                                SweepBehavior::FrequencyDecrease => {
                                    self.frequency_shadow
                                        - (self.frequency_shadow >> self.get_sweep_shift())
                                }
                            };

                            if test_frequency > 2047 {
                                self.enabled = false;
                            }
                        }
                    }

                    self.sweep_ticks_left = if self.get_sweep_period() == 0 {
                        8
                    } else {
                        self.get_sweep_period()
                    };
                }
            }

            self.frame_sequencer_idx = (self.frame_sequencer_idx + 1) % 8;
        }

        self.wave_duty_timer_ticks_left = self.wave_duty_timer_ticks_left.saturating_sub(1);
        if self.wave_duty_timer_ticks_left == 0 {
            self.wave_duty_index = (self.wave_duty_index + 1) % 8;

            // *4 on the GB, *16 on the GBA -- the GBA core clock runs at 4x the frequency.
            self.wave_duty_timer_ticks_left = (2048 - self.get_frequency()) * 16;
        }

        self.clock += 1;
    }

    pub fn sample(&self) -> u8 {
        if !self.enabled {
            return 0;
        }

        let wave_duty_index = usize::from(self.wave_duty_index);

        if self.get_wave_pattern_duty()[wave_duty_index] {
            self.volume
        } else {
            0
        }
    }

    // During a trigger event, several things occur:
    // - Square 1's frequency is copied to the shadow register.
    // - The sweep timer is reloaded.
    // - The internal enabled flag is set if either the sweep period or shift are non-zero, cleared otherwise.
    // - If the sweep shift is non-zero, frequency calculation and the overflow check are performed immediately.
    fn trigger(&mut self) {
        self.enabled = true;
        self.volume = self.get_envelope_initial_volume();

        if self.length_counter == 0 {
            self.length_counter = 64;
        }

        self.frequency_sweep_enabled = self.get_sweep_period() > 0 || self.get_sweep_shift() > 0;
        self.frequency_shadow = self.get_frequency();
    }
}

impl ToneAndSweep {
    fn get_sweep_shift(&self) -> u8 {
        const SWEEP_SHIFT_BIT_RANGE: RangeInclusive<usize> = 0..=2;

        self.sweep_register.get_bit_range(SWEEP_SHIFT_BIT_RANGE) as u8
    }

    fn get_sweep_behavior(&self) -> SweepBehavior {
        const SWEEP_BEHAVIOR_BIT_INDEX: usize = 3;
        if self.sweep_register.get_bit(SWEEP_BEHAVIOR_BIT_INDEX) {
            SweepBehavior::FrequencyDecrease
        } else {
            SweepBehavior::FrequencyIncrease
        }
    }

    fn get_sweep_period(&self) -> u8 {
        const SWEEP_PERIOD_BIT_RANGE: RangeInclusive<usize> = 4..=6;
        self.sweep_register.get_bit_range(SWEEP_PERIOD_BIT_RANGE) as u8
    }

    fn get_sound_length(&self) -> u8 {
        const SOUND_LENGTH_BIT_RANGE: RangeInclusive<usize> = 0..=5;
        self.duty_length_envelope
            .get_bit_range(SOUND_LENGTH_BIT_RANGE) as u8
    }

    fn get_wave_pattern_duty(&self) -> [bool; 8] {
        const EIGHTH_WAVE_DUTY_WAVEFORM: [bool; 8] =
            [false, false, false, false, false, false, false, true];
        const FOURTH_WAVE_DUTY_WAVEFORM: [bool; 8] =
            [true, false, false, false, false, false, false, true];
        const HALF_WAVE_DUTY_WAVEFORM: [bool; 8] =
            [true, false, false, false, false, true, true, true];
        const THREE_QUARTERS_WAVE_DUTY_WAVEFORM: [bool; 8] =
            [false, true, true, true, true, true, true, false];

        const WAVE_PATTERN_DUTY_BIT_RANGE: RangeInclusive<usize> = 6..=7;
        match self
            .duty_length_envelope
            .get_bit_range(WAVE_PATTERN_DUTY_BIT_RANGE)
        {
            0 => EIGHTH_WAVE_DUTY_WAVEFORM,
            1 => FOURTH_WAVE_DUTY_WAVEFORM,
            2 => HALF_WAVE_DUTY_WAVEFORM,
            3 => THREE_QUARTERS_WAVE_DUTY_WAVEFORM,
            _ => unreachable!(),
        }
    }

    fn get_envelope_sweep_period(&self) -> u8 {
        const ENVELOPE_SWEEP_PERIOD_BIT_RANGE: RangeInclusive<usize> = 8..=10;

        self.duty_length_envelope
            .get_bit_range(ENVELOPE_SWEEP_PERIOD_BIT_RANGE) as u8
    }

    fn get_envelope_direction(&self) -> EnvelopeBehavior {
        const ENVELOPE_DIRECTION_BIT_INDEX: usize = 11;

        if self
            .duty_length_envelope
            .get_bit(ENVELOPE_DIRECTION_BIT_INDEX)
        {
            EnvelopeBehavior::VolumeIncrease
        } else {
            EnvelopeBehavior::VolumeDecrease
        }
    }

    fn get_envelope_initial_volume(&self) -> u8 {
        const ENVELOPE_INITIAL_VOLUME_BIT_RANGE: RangeInclusive<usize> = 12..=15;

        self.duty_length_envelope
            .get_bit_range(ENVELOPE_INITIAL_VOLUME_BIT_RANGE) as u8
    }

    const FREQUENCY_BIT_RANGE: RangeInclusive<usize> = 0..=10;

    fn get_frequency(&self) -> u16 {
        self.frequency_control
            .get_bit_range(Self::FREQUENCY_BIT_RANGE)
    }

    fn set_frequency(&mut self, new_frequency: u16) {
        self.frequency_control = self
            .frequency_control
            .set_bit_range(new_frequency, Self::FREQUENCY_BIT_RANGE);
    }

    fn get_length_flag(&self) -> bool {
        const LENGTH_FLAG_BIT_INDEX: usize = 14;

        self.frequency_control.get_bit(LENGTH_FLAG_BIT_INDEX)
    }
}

impl ToneAndSweep {
    pub fn read_sweep_register<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.sweep_register.get_data(index)
    }

    pub fn write_sweep_register<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const SWEEP_WRITE_MASK: u16 = 0x007F;
        self.sweep_register = self.sweep_register.set_data(value, index) & SWEEP_WRITE_MASK;
    }

    pub fn read_duty_length_envelope<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.duty_length_envelope.get_data(index)
    }

    pub fn write_duty_length_envelope<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const LENGTH_COUNTER_BIT_RANGE: RangeInclusive<usize> = 0..=5;

        // Length counter in duty length envelope is always zero.
        self.duty_length_envelope = self.duty_length_envelope.set_data(value, index);

        self.length_counter = self
            .duty_length_envelope
            .get_bit_range(LENGTH_COUNTER_BIT_RANGE) as u8;

        self.duty_length_envelope = self
            .duty_length_envelope
            .set_bit_range(0, LENGTH_COUNTER_BIT_RANGE);
    }

    pub fn read_frequency_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        const FREQUENCY_CONTROL_READ_MASK: u16 = 0x4000;
        (self.frequency_control & FREQUENCY_CONTROL_READ_MASK).get_data(index)
    }

    pub fn write_frequency_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        // Trigger/Initital bit is always set to false for storage.
        const TRIGGER_BIT_INDEX: usize = 15;

        self.frequency_control = self.frequency_control.set_data(value, index);
        if self.frequency_control.get_bit(TRIGGER_BIT_INDEX) {
            self.trigger();
        }

        self.frequency_control = self.frequency_control.set_bit(TRIGGER_BIT_INDEX, false);
        if self.get_length_flag() {
            log::error!("length flag set");
        }
    }
}
