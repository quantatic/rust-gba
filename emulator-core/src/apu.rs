mod dma_fifo;

mod noise;
mod tone;
mod tone_and_sweep;
mod wave;

use std::ops::RangeInclusive;

use crate::{bit_manipulation::BitManipulation, bus::TimerStepResult, DataAccess};

use dma_fifo::DmaFifo;
use noise::Noise;
use tone::Tone;
use tone_and_sweep::ToneAndSweep;
use wave::Wave;

#[derive(Clone, Copy, Debug)]
enum DmaFifoTimerSelect {
    Timer0,
    Timer1,
}

#[derive(Clone, Debug, Default)]
pub struct Apu {
    channel_lr_volume_enable: u16,
    dma_sound_control: u16,
    sound_on_off: u32,
    sound_pwm_control: u32,

    fifo_a: DmaFifo,
    fifo_b: DmaFifo,
    tone_and_sweep: ToneAndSweep,
    tone: Tone,
    wave: Wave,
    noise: Noise,
}

impl Apu {
    // returns a value from -1.0 to 1.0
    pub fn sample(&self) -> [f32; 2] {
        let tone_and_sweep_sample = self.tone_and_sweep.sample();
        let tone_sample = self.tone.sample();
        let wave_sample = self.wave.sample();
        let noise_sample = self.noise.sample();
        let dma_fifo_a_sample = self.fifo_a.sample();
        let dma_fifo_b_sample = self.fifo_b.sample();

        let tone_and_sweep_sample_scaled =
            (((f32::from(tone_and_sweep_sample) / 15.0) * 2.0) - 1.0) / 4.0;
        let tone_sample_scaled = (((f32::from(tone_sample) / 15.0) * 2.0) - 1.0) / 4.0;
        let wave_sample_scaled = (((f32::from(wave_sample) / 15.0) * 2.0) - 1.0) / 4.0;
        let noise_sample_scaled = (((f32::from(noise_sample) / 15.0) * 2.0) - 1.0) / 4.0;

        let dma_fifo_a_scaled = (((f32::from(dma_fifo_a_sample) / 255.0) * 2.0) - 1.0) / 4.0;
        let dma_fifo_b_scaled = (((f32::from(dma_fifo_b_sample) / 255.0) * 2.0) - 1.0) / 4.0;

        let left_enabled = self.get_enable_flags_left();
        let right_enabled = self.get_enable_flags_left();

        let dma_a_enabled = self.get_dma_sound_a_enable();
        let dma_b_enabled = self.get_dma_sound_b_enable();

        // log::error!("{:?}", dma_a_enabled);
        // let left_enabled = [false, false, false, true];
        // let right_enabled = [false, false, false, true];
        // log::error!("{:?} {:?}", left_enabled, right_enabled);

        // let left_enabled = [false, false, false, true];
        // let right_enabled = [false, false, false, true];

        let mut sample_left = 0.0;
        let mut sample_right = 0.0;

        if left_enabled[0] {
            sample_left += tone_and_sweep_sample_scaled;
        }

        if right_enabled[0] {
            sample_right += tone_and_sweep_sample_scaled;
        }

        if left_enabled[1] {
            sample_left += tone_sample_scaled;
        }

        if right_enabled[1] {
            sample_right += tone_sample_scaled;
        }

        if left_enabled[2] {
            sample_left += wave_sample_scaled;
        }

        if right_enabled[2] {
            sample_right += wave_sample_scaled;
        }

        if left_enabled[3] {
            sample_left += noise_sample_scaled;
        }

        if right_enabled[3] {
            sample_right += noise_sample_scaled;
        }

        if dma_a_enabled.0 {
            sample_left += dma_fifo_a_scaled;
        }

        if dma_a_enabled.1 {
            sample_right += dma_fifo_a_scaled;
        }

        if dma_b_enabled.0 {
            sample_left += dma_fifo_b_scaled;
        }

        if dma_b_enabled.1 {
            sample_right += dma_fifo_b_scaled;
        }

        [sample_left, sample_right]
    }
}

impl Apu {
    pub(super) fn step(&mut self, timer_result: TimerStepResult) {
        self.tone_and_sweep.step();
        self.tone.step();
        self.wave.step();
        self.noise.step();

        let sound_a_overflow = match self.get_dma_sound_a_timer_select() {
            DmaFifoTimerSelect::Timer0 => timer_result.overflows[0],
            DmaFifoTimerSelect::Timer1 => timer_result.overflows[1],
        };

        let sound_b_overflow = match self.get_dma_sound_b_timer_select() {
            DmaFifoTimerSelect::Timer0 => timer_result.overflows[0],
            DmaFifoTimerSelect::Timer1 => timer_result.overflows[1],
        };

        self.fifo_a.step(sound_a_overflow);
        self.fifo_b.step(sound_b_overflow);
    }

    pub fn write_fifo_a(&mut self, value: u32) {
        self.fifo_a.write_data(value);
    }

    pub fn write_fifo_b(&mut self, value: u32) {
        self.fifo_b.write_data(value);
    }

    pub fn poll_fifo_a_wants_dma(&mut self) -> bool {
        self.fifo_a.poll_wants_dma()
    }

    pub fn poll_fifo_b_wants_dma(&mut self) -> bool {
        self.fifo_b.poll_wants_dma()
    }
}

impl Apu {
    pub fn read_ch1_sweep<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.tone_and_sweep.read_sweep_register(index)
    }

    pub fn write_ch1_sweep<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.tone_and_sweep.write_sweep_register(value, index);
    }

    pub fn read_ch1_duty_length_envelope<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.tone_and_sweep.read_duty_length_envelope(index)
    }

    pub fn write_ch1_duty_length_envelope<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.tone_and_sweep.write_duty_length_envelope(value, index)
    }

    pub fn read_ch1_frequency_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.tone_and_sweep.read_frequency_control(index)
    }

    pub fn write_ch1_frequency_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.tone_and_sweep.write_frequency_control(value, index)
    }
}

impl Apu {
    pub fn read_ch2_duty_length_envelope<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.tone.read_duty_length_envelope(index)
    }

    pub fn write_ch2_duty_length_envelope<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.tone.write_duty_length_envelope(value, index)
    }

    pub fn read_ch2_frequency_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.tone.read_frequency_control(index)
    }

    pub fn write_ch2_frequency_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.tone.write_frequency_control(value, index)
    }
}

impl Apu {
    pub fn read_ch3_stop_wave_ram_select<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.wave.read_stop_wave_ram_select(index)
    }

    pub fn write_ch3_stop_wave_ram_select<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.wave.write_stop_wave_ram_select(value, index);
    }

    pub fn read_ch3_length_volume<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.wave.read_length_volume(index)
    }

    pub fn write_ch3_length_volume<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.wave.write_length_volume(value, index);
    }

    pub fn read_ch3_frequency_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.wave.read_frequency_control(index)
    }

    pub fn write_ch3_frequency_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.wave.write_frequency_control(value, index)
    }

    pub fn read_ch3_wave_ram_byte(&self, offset: u32) -> u8 {
        self.wave.read_wave_ram_byte(offset)
    }

    pub fn write_ch3_wave_ram_byte(&mut self, value: u8, offset: u32) {
        self.wave.write_wave_ram_byte(value, offset)
    }
}

impl Apu {
    pub fn read_ch4_length_envelope<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.noise.read_length_envelope(index)
    }

    pub fn write_ch4_length_envelope<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.noise.write_length_envelope(value, index)
    }

    pub fn read_ch4_frequency_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.noise.read_frequency_control(index)
    }

    pub fn write_ch4_frequency_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.noise.write_frequency_control(value, index)
    }
}

impl Apu {
    fn get_master_volume_right(&self) -> u8 {
        const MASTER_VOLUME_RIGHT_BIT_RANGE: RangeInclusive<usize> = 0..=2;

        self.channel_lr_volume_enable
            .get_bit_range(MASTER_VOLUME_RIGHT_BIT_RANGE) as u8
    }

    fn get_master_volume_left(&self) -> u8 {
        const MASTER_VOLUME_LEFT_BIT_RANGE: RangeInclusive<usize> = 4..=6;

        self.channel_lr_volume_enable
            .get_bit_range(MASTER_VOLUME_LEFT_BIT_RANGE) as u8
    }

    fn get_enable_flags_right(&self) -> [bool; 4] {
        const ENABLE_FLAGS_RIGHT_BIT_RANGE: RangeInclusive<usize> = 8..=11;

        let enabled_raw = self
            .channel_lr_volume_enable
            .get_bit_range(ENABLE_FLAGS_RIGHT_BIT_RANGE);

        let mut result = [false; 4];
        for idx in 0..result.len() {
            result[idx] = enabled_raw.get_bit(idx);
        }

        result
    }

    fn get_enable_flags_left(&self) -> [bool; 4] {
        const ENABLE_FLAGS_LEFT_BIT_RANGE: RangeInclusive<usize> = 12..=15;

        let enabled_raw = self
            .channel_lr_volume_enable
            .get_bit_range(ENABLE_FLAGS_LEFT_BIT_RANGE);

        let mut result = [false; 4];
        for idx in 0..result.len() {
            result[idx] = enabled_raw.get_bit(idx);
        }

        result
    }
}

impl Apu {
    pub fn read_channel_lr_volume_enable<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.channel_lr_volume_enable.get_data(index)
    }

    pub fn write_channel_lr_volume_enable<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const CHANNEL_LR_VOLUME_ENABLE_WRITE_MASK: u16 = 0xFF77;
        self.channel_lr_volume_enable = self.channel_lr_volume_enable.set_data(value, index)
            & CHANNEL_LR_VOLUME_ENABLE_WRITE_MASK;
    }

    pub fn read_dma_sound_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.dma_sound_control.get_data(index)
    }

    pub fn write_dma_sound_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        // TODO: Handle bit 15 and 11 manually.
        const DMA_SOUND_CONTROL_WRITE_MASK: u16 = 0x770F;
        self.dma_sound_control =
            self.dma_sound_control.set_data(value, index) & DMA_SOUND_CONTROL_WRITE_MASK;
    }

    pub fn read_sound_on_off<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.sound_on_off.get_data(index)
    }

    pub fn write_sound_on_off<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        // TODO: Handle bits 0-3 manually in read.
        const SOUND_ON_OFF_WRITE_MASK: u32 = 0x0000_0080;
        self.sound_on_off = self.sound_on_off.set_data(value, index) & SOUND_ON_OFF_WRITE_MASK;
    }

    pub fn read_sound_pwm_control<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.sound_pwm_control.get_data(index)
    }

    pub fn write_sound_pwm_control<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        const SOUND_PWM_CONTROL_WRITE_MASK: u32 = 0x0000_BFFE;
        self.sound_pwm_control =
            self.sound_pwm_control.set_data(value, index) & SOUND_PWM_CONTROL_WRITE_MASK;
    }
}

impl Apu {
    fn get_dma_sound_a_enable(&self) -> (bool, bool) {
        const DMA_SOUND_A_ENABLE_RIGHT_BIT_INDEX: usize = 8;
        const DMA_SOUND_A_ENABLE_LEFT_BIT_INDEX: usize = 9;

        let enable_right = self
            .dma_sound_control
            .get_bit(DMA_SOUND_A_ENABLE_RIGHT_BIT_INDEX);
        let enable_left = self
            .dma_sound_control
            .get_bit(DMA_SOUND_A_ENABLE_LEFT_BIT_INDEX);

        (enable_left, enable_right)
    }

    fn get_dma_sound_a_timer_select(&self) -> DmaFifoTimerSelect {
        const DMA_SOUND_A_TIMER_SELECT_BIT_INDEX: usize = 10;

        if self
            .dma_sound_control
            .get_bit(DMA_SOUND_A_TIMER_SELECT_BIT_INDEX)
        {
            DmaFifoTimerSelect::Timer1
        } else {
            DmaFifoTimerSelect::Timer0
        }
    }

    fn get_dma_sound_b_enable(&self) -> (bool, bool) {
        const DMA_SOUND_B_ENABLE_RIGHT_BIT_INDEX: usize = 12;
        const DMA_SOUND_B_ENABLE_LEFT_BIT_INDEX: usize = 13;

        let enable_right = self
            .dma_sound_control
            .get_bit(DMA_SOUND_B_ENABLE_RIGHT_BIT_INDEX);
        let enable_left = self
            .dma_sound_control
            .get_bit(DMA_SOUND_B_ENABLE_LEFT_BIT_INDEX);

        (enable_left, enable_right)
    }

    fn get_dma_sound_b_timer_select(&self) -> DmaFifoTimerSelect {
        const DMA_SOUND_B_TIMER_SELECT_BIT_INDEX: usize = 14;

        if self
            .dma_sound_control
            .get_bit(DMA_SOUND_B_TIMER_SELECT_BIT_INDEX)
        {
            DmaFifoTimerSelect::Timer1
        } else {
            DmaFifoTimerSelect::Timer0
        }
    }
}
