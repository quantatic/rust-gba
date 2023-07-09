use std::ops::RangeInclusive;

use crate::{bit_manipulation::BitManipulation, data_access::DataAccess, CYCLES_PER_SECOND};

// Clocks per second
const SEQUENCER_CLOCK_FREQUENCY: u64 = 512;

// CPU cycles per clock
const SEQUENCER_CLOCK_PERIOD: u64 = CYCLES_PER_SECOND / SEQUENCER_CLOCK_FREQUENCY;

const LENGTH_COUNTER_CLOCKS: [bool; 8] = [true, false, true, false, true, false, true, false];
const VOLUME_ENVELOPE_CLOCKS: [bool; 8] = [false, false, false, false, false, false, false, true];

#[derive(Clone, Copy, Debug)]
enum WaveRamDimensions {
    OneBank,
    TwoBanks,
}

#[derive(Clone, Debug, Default)]
pub struct Wave {
    stop_wave_ram_select: u16,
    length_volume: u16,
    frequency_control: u16,

    length_counter: u16,

    wave_ram_low: [u8; 16],
    wave_ram_high: [u8; 16],

    // implemented internally with shift register, but use index for now.
    sample_idx: u8,
    wave_sample_timer_ticks_left: u16,

    frame_sequencer_idx: u8,
    clock: u64,
    enabled: bool,
}

impl Wave {
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

            self.frame_sequencer_idx = (self.frame_sequencer_idx + 1) % 8;
        }

        self.wave_sample_timer_ticks_left = self.wave_sample_timer_ticks_left.saturating_sub(1);
        if self.wave_sample_timer_ticks_left == 0 {
            self.sample_idx += 1;
            if self.sample_idx
                >= match self.get_wave_ram_dimensions() {
                    WaveRamDimensions::OneBank => 32,
                    WaveRamDimensions::TwoBanks => 64,
                }
            {
                self.sample_idx = 0;
            }

            self.wave_sample_timer_ticks_left = (2048 - self.get_sample_rate()) * 8;
        }

        self.clock += 1;
    }

    // 0 to 15 (inclusive) for now
    pub fn sample(&self) -> u8 {
        if !self.enabled || !self.sound_channel_playback() {
            return 0;
        }

        let (selected_bank, other_bank) = if self.get_wave_ram_high_bank() {
            (&self.wave_ram_high, &self.wave_ram_low)
        } else {
            (&self.wave_ram_low, &self.wave_ram_high)
        };

        let (wave_bank, index) = match (self.get_wave_ram_dimensions(), self.sample_idx) {
            (WaveRamDimensions::OneBank, index) | (WaveRamDimensions::TwoBanks, index @ 0..=31) => {
                (selected_bank, index)
            }
            (WaveRamDimensions::TwoBanks, index @ 32..=63) => (other_bank, index - 32),
            _ => unreachable!(),
        };

        let sample_nibble = if index % 2 == 0 {
            wave_bank[usize::from(index / 2)] >> 4
        } else {
            wave_bank[usize::from(index / 2)] & 0x0F
        };

        let scaled_nibble = if self.get_force_volume_75_percent() {
            sample_nibble / 4 * 3
        } else {
            match self.get_sound_volume_shift() {
                0 => 0,
                shift @ 1..=3 => sample_nibble >> shift,
                _ => unreachable!(),
            }
        };

        scaled_nibble
    }

    fn trigger(&mut self) {
        self.enabled = true;
        if self.length_counter == 0 {
            self.length_counter = 256;
        }
    }
}

impl Wave {
    fn get_wave_ram_dimensions(&self) -> WaveRamDimensions {
        const WAVE_RAM_DIMENSION_BIT_INDEX: usize = 5;

        if self
            .stop_wave_ram_select
            .get_bit(WAVE_RAM_DIMENSION_BIT_INDEX)
        {
            WaveRamDimensions::TwoBanks
        } else {
            WaveRamDimensions::OneBank
        }
    }

    fn get_wave_ram_high_bank(&self) -> bool {
        const WAVE_RAM_HIGH_BANK_BIT_INDEX: usize = 6;

        self.stop_wave_ram_select
            .get_bit(WAVE_RAM_HIGH_BANK_BIT_INDEX)
    }

    fn sound_channel_playback(&self) -> bool {
        const SOUND_CHANNEL_PLAYBACK_BIT_INDEX: usize = 7;

        self.stop_wave_ram_select
            .get_bit(SOUND_CHANNEL_PLAYBACK_BIT_INDEX)
    }

    fn get_sound_length(&self) -> u8 {
        const SOUND_LENGTH_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        self.length_volume.get_bit_range(SOUND_LENGTH_BIT_RANGE) as u8
    }

    fn get_sound_volume_shift(&self) -> u8 {
        const SOUND_VOLUME_SHIFT_BIT_RANGE: RangeInclusive<usize> = 13..=14;

        self.length_volume
            .get_bit_range(SOUND_VOLUME_SHIFT_BIT_RANGE) as u8
    }

    fn get_force_volume_75_percent(&self) -> bool {
        const FORCE_VOLUME_75_PERCENT_BIT_INDEX: usize = 15;

        self.length_volume
            .get_bit(FORCE_VOLUME_75_PERCENT_BIT_INDEX)
    }

    fn get_sample_rate(&self) -> u16 {
        const SAMPLE_RATE_BIT_RANGE: RangeInclusive<usize> = 0..=10;

        self.frequency_control.get_bit_range(SAMPLE_RATE_BIT_RANGE)
    }

    fn get_length_flag(&self) -> bool {
        const LENGTH_FLAG_BIT_INDEX: usize = 14;

        self.frequency_control.get_bit(LENGTH_FLAG_BIT_INDEX)
    }
}

impl Wave {
    pub fn read_stop_wave_ram_select<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.stop_wave_ram_select.get_data(index)
    }

    pub fn write_stop_wave_ram_select<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.stop_wave_ram_select = self.stop_wave_ram_select.set_data(value, index)
    }

    pub fn read_length_volume<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.length_volume.get_data(index)
    }

    pub fn write_length_volume<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.length_volume = self.length_volume.set_data(value, index)
    }

    pub fn read_frequency_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.frequency_control.get_data(index)
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

    pub fn write_wave_ram_byte(&mut self, value: u8, offset: u32) {
        // We write to the unselected wave RAM bank.
        if self.get_wave_ram_high_bank() {
            self.wave_ram_low[offset as usize] = value;
        } else {
            self.wave_ram_high[offset as usize] = value;
        };
    }

    pub fn read_wave_ram_byte(&self, offset: u32) -> u8 {
        if self.get_wave_ram_high_bank() {
            self.wave_ram_low[offset as usize]
        } else {
            self.wave_ram_high[offset as usize]
        }
    }
}
