use std::ops::RangeInclusive;

use crate::{bit_manipulation::BitManipulation, data_access::DataAccess, CYCLES_PER_SECOND};

// Clocks per second
const SEQUENCER_CLOCK_FREQUENCY: u64 = 512;

// CPU cycles per clock
const SEQUENCER_CLOCK_PERIOD: u64 = CYCLES_PER_SECOND / SEQUENCER_CLOCK_FREQUENCY;

const LENGTH_COUNTER_CLOCKS: [bool; 8] = [true, false, true, false, true, false, true, false];
const VOLUME_ENVELOPE_CLOCKS: [bool; 8] = [false, false, false, false, false, false, false, true];

#[derive(Clone, Copy, Debug)]
enum EnvelopeBehavior {
    VolumeIncrease,
    VolumeDecrease,
}

#[derive(Clone, Copy, Debug)]
enum CounterStepWidth {
    FifteenBit,
    SevenBit,
}

#[derive(Clone, Debug, Default)]
pub struct Noise {
    length_envelope: u16,
    frequency_control: u16,

    length_counter: u8,

    frame_sequencer_idx: u16,
    clock: u64,

    linear_feedback_shift_register: u16,
    noise_ticks_left: u16,

    envelope_ticks_left: u8,

    enabled: bool,
    volume: u8,
}

impl Noise {
    pub fn step(&mut self) {
        if self.clock % SEQUENCER_CLOCK_PERIOD == 0 {
            if LENGTH_COUNTER_CLOCKS[usize::from(self.frame_sequencer_idx)] {
                if self.get_length_flag() {
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

            self.frame_sequencer_idx = (self.frame_sequencer_idx + 1) % 8;
        }

        // When clocked by the frequency timer, the low two bits (0 and 1) are XORed, all bits are shifted
        // right by one, and the result of the XOR is put into the now-empty high bit.
        self.noise_ticks_left = self.noise_ticks_left.saturating_sub(1);
        if self.noise_ticks_left == 0 {
            let xor_result = self.linear_feedback_shift_register.get_bit(0)
                ^ self.linear_feedback_shift_register.get_bit(1);
            self.linear_feedback_shift_register =
                (self.linear_feedback_shift_register >> 1).set_bit(14, xor_result);

            // If width mode is 1 (NR43, 7 bit counter step/width), the XOR result is ALSO put into bit 6 AFTER the shift, resulting in a 7-bit LFSR.
            match self.get_counter_step_width() {
                CounterStepWidth::SevenBit => {
                    self.linear_feedback_shift_register =
                        self.linear_feedback_shift_register.set_bit(6, xor_result)
                }
                CounterStepWidth::FifteenBit => {}
            };

            // GBATEK: Frequency = 524288 Hz / r / 2^(s+1) ;For r=0 assume r=0.5 instead
            //
            // This can be equivalently modeled as 262_144 Hz / r / 2^s (still treating r=0 as r=0.5)
            // Converted to ticks between update (at a tick rate of 16_777_216 Hz), we get:
            // r * 2^s * 64 (with r=0 treated as r=0.5), or (r << 6) * 2^s, or (r << 6) * (1 << s), with (r << 6)=32 when r=0

            self.noise_ticks_left = match self.get_frequency_dividing_ratio() {
                0 => 32 << self.get_shift_clock_frequency(),
                ratio => (u16::from(ratio) << 6) << self.get_shift_clock_frequency(),
            };
        }

        self.clock += 1;
    }

    pub fn sample(&self) -> u8 {
        if !self.enabled {
            return 0;
        }

        // Audio is high when lowest bit is 0.
        if !self.linear_feedback_shift_register.get_bit(0) {
            self.volume
        } else {
            0
        }
    }

    // During a trigger event, several things occur:
    // - Channel is enabled (see length counter).
    // - If length counter is zero, it is set to 64 (256 for wave channel).
    // - Volume envelope timer is reloaded with period.
    // - Channel volume is reloaded from NRx2.
    // - Noise channel's LFSR bits are all set to 1.
    // - Wave channel's position is set to 0 but sample buffer is NOT refilled.
    // - Square 1's sweep does several things (see frequency sweep).
    fn trigger(&mut self) {
        self.enabled = true;
        self.volume = self.get_envelope_initial_volume();
        self.linear_feedback_shift_register = !0;

        if self.length_counter == 0 {
            self.length_counter = 64;
        }
    }
}

impl Noise {
    fn get_sound_length(&self) -> u8 {
        const SOUND_LENGTH_BIT_RANGE: RangeInclusive<usize> = 0..=5;
        self.length_envelope.get_bit_range(SOUND_LENGTH_BIT_RANGE) as u8
    }

    fn get_envelope_sweep_period(&self) -> u8 {
        const ENVELOPE_SWEEP_PERIOD_BIT_RANGE: RangeInclusive<usize> = 8..=10;

        self.length_envelope
            .get_bit_range(ENVELOPE_SWEEP_PERIOD_BIT_RANGE) as u8
    }

    fn get_envelope_direction(&self) -> EnvelopeBehavior {
        const ENVELOPE_DIRECTION_BIT_INDEX: usize = 11;

        if self.length_envelope.get_bit(ENVELOPE_DIRECTION_BIT_INDEX) {
            EnvelopeBehavior::VolumeIncrease
        } else {
            EnvelopeBehavior::VolumeDecrease
        }
    }

    fn get_envelope_initial_volume(&self) -> u8 {
        const ENVELOPE_INITIAL_VOLUME_BIT_RANGE: RangeInclusive<usize> = 12..=15;

        self.length_envelope
            .get_bit_range(ENVELOPE_INITIAL_VOLUME_BIT_RANGE) as u8
    }

    fn get_frequency_dividing_ratio(&self) -> u8 {
        const FREQUENCY_DIVIDING_RATIO_BIT_RANGE: RangeInclusive<usize> = 0..=2;

        self.frequency_control
            .get_bit_range(FREQUENCY_DIVIDING_RATIO_BIT_RANGE) as u8
    }

    fn get_counter_step_width(&self) -> CounterStepWidth {
        const COUNTER_STEP_WIDTH_BIT_INDEX: usize = 3;

        if self.frequency_control.get_bit(COUNTER_STEP_WIDTH_BIT_INDEX) {
            CounterStepWidth::SevenBit
        } else {
            CounterStepWidth::FifteenBit
        }
    }

    fn get_shift_clock_frequency(&self) -> u8 {
        const SHIFT_CLOCK_FREQUENCY_BIT_RANGE: RangeInclusive<usize> = 4..=7;

        self.frequency_control
            .get_bit_range(SHIFT_CLOCK_FREQUENCY_BIT_RANGE) as u8
    }

    fn get_length_flag(&self) -> bool {
        const LENGTH_FLAG_BIT_INDEX: usize = 14;

        self.frequency_control.get_bit(LENGTH_FLAG_BIT_INDEX)
    }
}

impl Noise {
    pub fn read_length_envelope<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.length_envelope.get_data(index)
    }

    pub fn write_length_envelope<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const LENGTH_COUNTER_BIT_RANGE: RangeInclusive<usize> = 0..=5;

        // Length counter in duty length envelope is always zero.
        self.length_envelope = self.length_envelope.set_data(value, index);

        self.length_counter = self.length_envelope.get_bit_range(LENGTH_COUNTER_BIT_RANGE) as u8;

        self.length_envelope = self
            .length_envelope
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
    }
}
