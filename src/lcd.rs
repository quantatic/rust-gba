use std::ops::RangeInclusive;

use crate::{bit_manipulation::BitManipulation, bus::DataAccess};

pub const LCD_WIDTH: usize = 240;
pub const LCD_HEIGHT: usize = 160;

#[derive(Debug)]
enum LcdState {
    Visible,
    HBlank,
    VBlank,
}

#[derive(Clone, Copy, Debug)]
enum BgMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
    Mode4,
    Mode5,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PixelColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug)]
pub struct Lcd {
    dot: u16,
    vcount: u16,
    lcd_control: u16,
    lcd_status: u16,
    state: LcdState,
    palette_ram: Box<[u8; 0x400]>,
    vram: Box<[u8; 0x18000]>,
    oam: Box<[u8; 0x400]>,
    vblank_interrupt_waiting: bool,
    hblank_interrupt_waiting: bool,
    vcount_interrupt_waiting: bool,
    buffer: [[PixelColor; LCD_WIDTH]; LCD_HEIGHT], // access as buffer[y][x]
}

pub struct LcdInterruptInfo {
    pub vblank: bool,
    pub hblank: bool,
    pub vcount: bool,
}

impl Default for Lcd {
    fn default() -> Self {
        Self {
            dot: 0,
            vcount: 0,
            lcd_control: 0,
            lcd_status: 0,
            state: LcdState::Visible,
            palette_ram: Box::new([0; 0x400]),
            vram: Box::new([0; 0x18000]),
            oam: Box::new([0; 0x400]),
            vblank_interrupt_waiting: false,
            hblank_interrupt_waiting: false,
            vcount_interrupt_waiting: false,
            buffer: [[PixelColor::default(); LCD_WIDTH]; LCD_HEIGHT],
        }
    }
}

impl Lcd {
    pub fn step(&mut self) {
        if self.vcount < 160 {
            if self.dot == 0 {
                self.set_vblank_flag(false);
                self.set_hblank_flag(false);
                self.state = LcdState::Visible;
            } else if self.dot == 240 {
                self.set_hblank_flag(true);
                self.hblank_interrupt_waiting = true;
                self.state = LcdState::HBlank;
            }
        } else if self.vcount == 160 && self.dot == 0 {
            self.set_vblank_flag(true);
            self.vblank_interrupt_waiting = true;
            self.state = LcdState::VBlank;
        }

        if matches!(self.state, LcdState::Visible) {
            match self.get_bg_mode() {
                BgMode::Mode4 => {
                    let pixel_x = self.dot;
                    let pixel_y = self.vcount;

                    let byte_idx = (usize::from(pixel_y) * LCD_WIDTH) + usize::from(pixel_x);

                    let palette_idx = self.vram[byte_idx];

                    self.buffer[usize::from(pixel_y)][usize::from(pixel_x)] = if palette_idx == 0 {
                        PixelColor {
                            red: 0,
                            green: 0,
                            blue: 0,
                        }
                    } else {
                        PixelColor {
                            red: 255,
                            green: 255,
                            blue: 255,
                        }
                    };
                }
                _ => {}
            };
        }

        self.dot += 1;

        if self.dot >= 308 {
            self.dot = 0;
            self.vcount += 1;

            if self.vcount >= 228 {
                self.vcount = 0;
            }
        }
    }
}

impl Lcd {
    pub fn read_vcount<DataAccessType>(&self, index: u32) -> DataAccessType
    where
        u16: DataAccess<DataAccessType>,
    {
        self.vcount.get_data(index)
    }

    pub fn read_lcd_control<DataAccessType>(&self, index: u32) -> DataAccessType
    where
        u16: DataAccess<DataAccessType>,
    {
        self.lcd_control.get_data(index)
    }

    pub fn write_lcd_control<DataAccessType>(&mut self, value: DataAccessType, index: u32)
    where
        u16: DataAccess<DataAccessType>,
    {
        self.lcd_control = self.lcd_control.set_data(value, index);
    }

    pub fn read_lcd_status<DataAccessType>(&self, index: u32) -> DataAccessType
    where
        u16: DataAccess<DataAccessType>,
    {
        self.lcd_status.get_data(index)
    }

    pub fn write_lcd_status<DataAccessType>(&mut self, value: DataAccessType, index: u32)
    where
        u16: DataAccess<DataAccessType>,
    {
        self.lcd_status = self.lcd_status.set_data(value, index);
    }

    pub fn read_palette_ram(&self, offset: u32) -> u8 {
        self.palette_ram[offset as usize]
    }

    pub fn write_palette_ram(&mut self, value: u8, offset: u32) {
        self.palette_ram[offset as usize] = value;
    }

    pub fn read_vram(&self, offset: u32) -> u8 {
        self.vram[offset as usize]
    }

    pub fn write_vram(&mut self, value: u8, offset: u32) {
        self.vram[offset as usize] = value;
    }

    pub fn read_oam(&self, offset: u32) -> u8 {
        self.oam[offset as usize]
    }

    pub fn write_oam(&mut self, value: u8, offset: u32) {
        self.oam[offset as usize] = value;
    }
}

impl Lcd {
    fn get_bg_mode(&self) -> BgMode {
        const BG_MODE_FLAG_BIT_RANGE: RangeInclusive<usize> = 0..=2;

        let mode_index = self.lcd_control.get_bit_range(BG_MODE_FLAG_BIT_RANGE);
        match mode_index {
            0 => BgMode::Mode0,
            1 => BgMode::Mode1,
            2 => BgMode::Mode2,
            3 => BgMode::Mode3,
            4 => BgMode::Mode4,
            5 => BgMode::Mode5,
            _ => unreachable!("prohibited mode {}", mode_index),
        }
    }
    fn set_vblank_flag(&mut self, set: bool) {
        const VBLANK_FLAG_BIT_INDEX: usize = 0;

        self.lcd_status = self.lcd_status.set_bit(VBLANK_FLAG_BIT_INDEX, set);
    }

    fn set_hblank_flag(&mut self, set: bool) {
        const HBLANK_FLAG_BIT_INDEX: usize = 1;

        self.lcd_status = self.lcd_status.set_bit(HBLANK_FLAG_BIT_INDEX, set);
    }

    fn get_vblank_irq_enable(&self) -> bool {
        const VBLANK_IRQ_ENABLE_BIT_INDEX: usize = 3;

        self.lcd_status.get_bit(VBLANK_IRQ_ENABLE_BIT_INDEX)
    }

    fn get_hblank_irq_enable(&self) -> bool {
        const HBLANK_IRQ_ENABLE_BIT_INDEX: usize = 4;

        self.lcd_status.get_bit(HBLANK_IRQ_ENABLE_BIT_INDEX)
    }

    fn get_vcount_irq_enable(&self) -> bool {
        const VCOUNT_IRQ_ENABLE_BIT_INDEX: usize = 5;

        self.lcd_status.get_bit(VCOUNT_IRQ_ENABLE_BIT_INDEX)
    }

    fn get_vcount_setting(&self) -> u16 {
        const VCOUNT_SETTING_BIT_RANGE: RangeInclusive<usize> = 8..=15;

        self.lcd_status.get_bit_range(VCOUNT_SETTING_BIT_RANGE)
    }
}

impl Lcd {
    pub fn poll_pending_interrupts(&mut self) -> LcdInterruptInfo {
        LcdInterruptInfo {
            hblank: self.get_hblank_irq_enable() && self.hblank_interrupt_waiting,
            vblank: self.get_vblank_irq_enable() && self.vblank_interrupt_waiting,
            vcount: self.get_vcount_irq_enable() && self.vcount_interrupt_waiting,
        }
    }

    pub fn clear_pending_interrupts(&mut self) {
        self.hblank_interrupt_waiting = false;
        self.vblank_interrupt_waiting = false;
        self.vcount_interrupt_waiting = false;
    }

    pub fn buffer(&self) -> &[[PixelColor; LCD_WIDTH]; LCD_HEIGHT] {
        &self.buffer
    }
}
