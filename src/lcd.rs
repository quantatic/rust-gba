mod layer_0;
mod layer_2;

use crate::{BitManipulation, DataAccess};
use layer_0::Layer0;
use layer_2::Layer2;

use std::ops::RangeInclusive;

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

#[derive(Clone, Copy, Debug)]
enum DisplayFrame {
    Frame0,
    Frame1,
}

#[derive(Clone, Copy, Debug)]
enum PaletteDepth {
    FourBit,
    EightBit,
}

#[derive(Clone, Copy, Debug)]
enum TextScreenSize {
    Size32x32,
    Size64x32,
    Size32x64,
    Size64x64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rgb555 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb555 {
    const RED_INTENSITY_BIT_RANGE: RangeInclusive<usize> = 0..=4;
    const GREEN_INTENSITY_BIT_RANGE: RangeInclusive<usize> = 5..=9;
    const BLUE_INTENSITY_BIT_RANGE: RangeInclusive<usize> = 10..=14;

    fn to_int(self) -> u16 {
        0.set_bit_range(u16::from(self.red), Self::RED_INTENSITY_BIT_RANGE)
            .set_bit_range(u16::from(self.green), Self::GREEN_INTENSITY_BIT_RANGE)
            .set_bit_range(u16::from(self.blue), Self::BLUE_INTENSITY_BIT_RANGE)
    }

    fn from_int(val: u16) -> Self {
        let red = val.get_bit_range(Rgb555::RED_INTENSITY_BIT_RANGE) as u8;
        let green = val.get_bit_range(Rgb555::GREEN_INTENSITY_BIT_RANGE) as u8;
        let blue = val.get_bit_range(Rgb555::BLUE_INTENSITY_BIT_RANGE) as u8;

        Self { red, green, blue }
    }
}

#[derive(Debug)]
pub struct Lcd {
    dot: u16,
    vcount: u16,
    lcd_control: u16,
    lcd_status: u16,
    state: LcdState,
    bg_palette_ram: Box<[Rgb555; 0x100]>,
    obj_palette_ram: Box<[Rgb555; 0x100]>,
    vram: Box<[u8; 0x18000]>,
    oam: Box<[u8; 0x400]>,
    vblank_interrupt_waiting: bool,
    hblank_interrupt_waiting: bool,
    vcount_interrupt_waiting: bool,
    buffer: Box<[[Rgb555; LCD_WIDTH]; LCD_HEIGHT]>, // access as buffer[y][x]
    back_buffer: Box<[[Rgb555; LCD_WIDTH]; LCD_HEIGHT]>,
    layer_0: Layer0,
    layer_2: Layer2,
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
            bg_palette_ram: Box::new([Rgb555::default(); 0x100]),
            obj_palette_ram: Box::new([Rgb555::default(); 0x100]),
            vram: Box::new([0; 0x18000]),
            oam: Box::new([0; 0x400]),
            vblank_interrupt_waiting: false,
            hblank_interrupt_waiting: false,
            vcount_interrupt_waiting: false,
            buffer: Box::new([[Rgb555::default(); LCD_WIDTH]; LCD_HEIGHT]),
            back_buffer: Box::new([[Rgb555::default(); LCD_WIDTH]; LCD_HEIGHT]),
            layer_0: Layer0::default(),
            layer_2: Layer2::default(),
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
            std::mem::swap(&mut self.buffer, &mut self.back_buffer);
        }

        if matches!(self.state, LcdState::Visible) {
            let pixel_x = self.dot;
            let pixel_y = self.vcount;

            let current_mode = self.get_bg_mode();
            let display_frame = self.get_display_frame();

            let layer_0_pixel = if self.get_screen_display_bg_0() {
                self.layer_0.get_pixel(
                    pixel_x,
                    pixel_y,
                    current_mode,
                    self.vram.as_slice(),
                    self.bg_palette_ram.as_slice(),
                )
            } else {
                None
            };

            let layer_2_pixel = if self.get_screen_display_bg_2() {
                self.layer_2.get_pixel(
                    pixel_x,
                    pixel_y,
                    current_mode,
                    display_frame,
                    self.vram.as_slice(),
                    self.bg_palette_ram.as_slice(),
                )
            } else {
                None
            };

            let final_pixel = None.or(layer_0_pixel).or(layer_2_pixel);

            if let Some(pixel) = final_pixel {
                self.back_buffer[usize::from(pixel_y)][usize::from(pixel_x)] = pixel;
            }
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
    pub fn read_vcount<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.vcount.get_data(index)
    }

    pub fn read_lcd_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.lcd_control.get_data(index)
    }

    pub fn write_lcd_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.lcd_control = self.lcd_control.set_data(value, index);
    }

    pub fn read_lcd_status<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.lcd_status.get_data(index)
    }

    pub fn write_lcd_status<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.lcd_status = self.lcd_status.set_data(value, index);
    }

    const BG_PALETTE_RAM_OFFSET_START: u32 = 0x000;
    const BG_PALETTE_RAM_OFFSET_END: u32 = 0x1FF;
    const OBJ_PALETTE_RAM_OFFSET_START: u32 = 0x200;
    const OBJ_PALETTE_RAM_OFFSET_END: u32 = 0x3FF;

    pub fn read_palette_ram(&self, offset: u32) -> u8 {
        let color = match offset {
            Self::BG_PALETTE_RAM_OFFSET_START..=Self::BG_PALETTE_RAM_OFFSET_END => {
                let color_idx = (offset - Self::BG_PALETTE_RAM_OFFSET_START) / 2;
                self.bg_palette_ram[color_idx as usize]
            }
            Self::OBJ_PALETTE_RAM_OFFSET_START..=Self::OBJ_PALETTE_RAM_OFFSET_END => {
                let color_idx = (offset - Self::OBJ_PALETTE_RAM_OFFSET_START) / 2;
                self.obj_palette_ram[color_idx as usize]
            }
            _ => unreachable!(),
        };

        color.to_int().get_data(offset & 0b1)
    }

    pub fn write_palette_ram(&mut self, value: u8, offset: u32) {
        let color = match offset {
            Self::BG_PALETTE_RAM_OFFSET_START..=Self::BG_PALETTE_RAM_OFFSET_END => {
                let color_idx = (offset - Self::BG_PALETTE_RAM_OFFSET_START) / 2;
                &mut self.bg_palette_ram[color_idx as usize]
            }
            Self::OBJ_PALETTE_RAM_OFFSET_START..=Self::OBJ_PALETTE_RAM_OFFSET_END => {
                let color_idx = (offset - Self::OBJ_PALETTE_RAM_OFFSET_START) / 2;
                &mut self.obj_palette_ram[color_idx as usize]
            }
            _ => unreachable!(),
        };

        let modified_range = match offset & 0b1 {
            0 => 0..=7,
            1 => 8..=15,
            _ => unreachable!(),
        };

        let new_color = Rgb555::from_int(
            color
                .to_int()
                .set_bit_range(u16::from(value), modified_range),
        );

        *color = new_color;
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

    pub fn read_layer0_bg_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_0.read_bg_control(index)
    }

    pub fn write_layer0_bg_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_0.write_bg_control(value, index);
    }

    pub fn read_layer0_x_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_0.read_x_offset(index)
    }

    pub fn write_layer0_x_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_0.write_x_offset(value, index);
    }

    pub fn read_layer0_y_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_0.read_y_offset(index)
    }

    pub fn write_layer0_y_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_0.write_y_offset(value, index);
    }

    pub fn read_layer2_bg_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_2.read_bg_control(index)
    }

    pub fn write_layer2_bg_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_2.write_bg_control(value, index);
    }

    pub fn read_layer2_text_x_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_2.read_text_x_offset(index)
    }

    pub fn write_layer2_text_x_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_2.write_text_x_offset(value, index);
    }

    pub fn read_layer2_text_y_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_2.read_text_y_offset(index)
    }

    pub fn write_layer2_text_y_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_2.write_text_y_offset(value, index);
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

    fn get_display_frame(&self) -> DisplayFrame {
        const DISPLAY_FRAME_SELECT_BIT_INDEX: usize = 4;

        if self.lcd_control.get_bit(DISPLAY_FRAME_SELECT_BIT_INDEX) {
            DisplayFrame::Frame1
        } else {
            DisplayFrame::Frame0
        }
    }

    fn get_screen_display_bg_0(&self) -> bool {
        const SCREEN_DISPLAY_BG_0_BIT_INDEX: usize = 8;

        self.lcd_control.get_bit(SCREEN_DISPLAY_BG_0_BIT_INDEX)
    }

    fn get_screen_display_bg_1(&self) -> bool {
        const SCREEN_DISPLAY_BG_1_BIT_INDEX: usize = 9;

        self.lcd_control.get_bit(SCREEN_DISPLAY_BG_1_BIT_INDEX)
    }

    fn get_screen_display_bg_2(&self) -> bool {
        const SCREEN_DISPLAY_BG_2_BIT_INDEX: usize = 10;

        self.lcd_control.get_bit(SCREEN_DISPLAY_BG_2_BIT_INDEX)
    }

    fn get_screen_display_bg_3(&self) -> bool {
        const SCREEN_DISPLAY_BG_3_BIT_INDEX: usize = 11;

        self.lcd_control.get_bit(SCREEN_DISPLAY_BG_3_BIT_INDEX)
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
        let hblank = self.get_hblank_irq_enable() && self.hblank_interrupt_waiting;
        let vblank = self.get_vblank_irq_enable() && self.vblank_interrupt_waiting;
        let vcount = self.get_vcount_irq_enable() && self.vcount_interrupt_waiting;

        self.hblank_interrupt_waiting = false;
        self.vblank_interrupt_waiting = false;
        self.vcount_interrupt_waiting = false;

        LcdInterruptInfo {
            hblank,
            vblank,
            vcount,
        }
    }

    pub fn buffer(&self) -> &[[Rgb555; LCD_WIDTH]; LCD_HEIGHT] {
        &self.buffer
    }
}
