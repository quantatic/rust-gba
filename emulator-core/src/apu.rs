mod dma_fifo;

use crate::{bit_manipulation::BitManipulation, bus::TimerStepResult, DataAccess};
use dma_fifo::DmaFifo;

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
}

impl Apu {
    // returns a value from -1.0 to 1.0
    pub fn sample(&self) -> f32 {
        let a_sample = self.fifo_a.sample();
        let b_sample = self.fifo_b.sample();

        let a_sample_scaled = f32::from(a_sample) / 128.0;
        let b_sample_scaled = f32::from(b_sample) / 128.0;

        (a_sample_scaled + b_sample_scaled) / 2.0
    }
}

impl Apu {
    pub(super) fn step(&mut self, timer_result: TimerStepResult) {
        // todo!()
    }

    pub fn write_fifo_a(&mut self, value: u32) {
        self.fifo_a.write_data(value);
    }

    pub fn write_fifo_b(&mut self, value: u32) {
        self.fifo_b.write_data(value);
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
