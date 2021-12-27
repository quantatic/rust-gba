use std::ops::RangeInclusive;

use crate::{BitManipulation, DataAccess};

use super::{BgMode, DisplayFrame, PaletteDepth, Rgb555, TextScreenSize};

#[derive(Debug, Default)]
pub(super) struct Layer2 {
    bg_control: u16,
    text_x_offset: u16,
    text_y_offset: u16,
    affine_x_offset: u32,
    affine_y_offset: u32,
    affine_a: u16,
    affine_b: u16,
    affine_c: u16,
    affine_d: u16,
}

impl Layer2 {
    pub fn get_pixel(
        &self,
        pixel_x: u16,
        pixel_y: u16,
        mode: BgMode,
        frame: DisplayFrame,
        vram: &[u8],
        bg_palette: &[Rgb555],
    ) -> Option<Rgb555> {
        match mode {
            BgMode::Mode0 => {
                let x = pixel_x + self.get_text_x_offset();
                let y = pixel_y + self.get_text_y_offset();

                let map_data_base = self.bg_map_data_base() as usize;
                let tile_data_base = self.bg_tile_data_base() as usize;

                let map_data_x = usize::from(x / 8) % 64;
                let map_data_y = usize::from(y / 8) % 64;

                let map_block_offset = ((map_data_y % 32) * 32) + (map_data_x % 32);
                let map_data_offset = match self.get_text_screen_size() {
                    TextScreenSize::Size32x32 => map_block_offset,
                    TextScreenSize::Size32x64 => {
                        if map_data_y >= 32 {
                            map_block_offset + 1024
                        } else {
                            map_block_offset
                        }
                    }
                    TextScreenSize::Size64x32 => {
                        if map_data_x >= 32 {
                            map_block_offset + 1024
                        } else {
                            map_block_offset
                        }
                    }
                    TextScreenSize::Size64x64 => {
                        let x_block_offset = if map_data_x >= 32 { 1024 } else { 0 };

                        let y_block_offset = if map_data_y >= 32 { 2048 } else { 0 };

                        map_block_offset + x_block_offset + y_block_offset
                    }
                };

                let map_data_idx = map_data_base + (usize::from(map_data_offset) * 2);

                let map_data_low = vram[map_data_idx];
                let map_data_high = vram[map_data_idx + 1];
                let map_data = u16::from_le_bytes([map_data_low, map_data_high]);

                let tile_number = map_data.get_bit_range(0..=9);
                let horizontal_flip = map_data.get_bit(10);
                let vertical_flip = map_data.get_bit(11);
                let palette_number = map_data.get_bit_range(12..=15) as u8;

                let tile_data_x = if vertical_flip {
                    7 - usize::from(x % 8)
                } else {
                    usize::from(x % 8)
                };

                let tile_data_y = if horizontal_flip {
                    7 - usize::from(y % 8)
                } else {
                    usize::from(y % 8)
                };

                let palette_idx = match self.get_palette_depth() {
                    PaletteDepth::EightBit => todo!(),
                    PaletteDepth::FourBit => {
                        let tile_idx = tile_data_base
                            + (32 * usize::from(tile_number))
                            + (tile_data_y * 4)
                            + (tile_data_x / 2);
                        let tile_data = vram[tile_idx];

                        let palette_idx_low = if tile_data_x % 2 == 0 {
                            tile_data.get_bit_range(0..=3)
                        } else {
                            tile_data.get_bit_range(4..=7)
                        };

                        palette_idx_low.set_bit_range(palette_number, 4..=7)
                    }
                };

                Some(bg_palette[usize::from(palette_idx)])
            }
            BgMode::Mode1 | BgMode::Mode2 => {
                todo!()
            }
            BgMode::Mode3 => {
                let pixel_idx = (usize::from(pixel_y) * super::LCD_WIDTH) + usize::from(pixel_x);
                let pixel_offset = pixel_idx * 2;

                let pixel_low = vram[pixel_offset];
                let pixel_high = vram[pixel_offset + 1];
                let pixel_int = u16::from_le_bytes([pixel_low, pixel_high]);

                Some(Rgb555::from_int(pixel_int))
            }
            BgMode::Mode4 => {
                const FRAME_SIZE: u32 = 0xA000;

                let pixel_idx = (usize::from(pixel_y) * super::LCD_WIDTH) + usize::from(pixel_x);
                let pixel_offset = match frame {
                    DisplayFrame::Frame0 => pixel_idx,
                    DisplayFrame::Frame1 => pixel_idx + (FRAME_SIZE as usize),
                };

                let pixel_palette_idx = vram[pixel_offset];

                Some(bg_palette[usize::from(pixel_palette_idx)])
            }
            BgMode::Mode5 => {
                const MODE_WIDTH: u16 = 160;
                const MODE_HEIGHT: u16 = 128;

                if pixel_x >= MODE_WIDTH || pixel_y >= MODE_HEIGHT {
                    return Some(Rgb555::default());
                }

                let pixel_idx =
                    (usize::from(pixel_y) * usize::from(MODE_WIDTH)) + usize::from(pixel_x);
                let pixel_offset = pixel_idx * 2;

                let pixel_low = vram[pixel_offset];
                let pixel_high = vram[pixel_offset + 1];
                let pixel_int = u16::from_le_bytes([pixel_low, pixel_high]);

                Some(Rgb555::from_int(pixel_int))
            }
            _ => None,
        }
    }
}

impl Layer2 {
    pub fn read_bg_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.bg_control.get_data(index)
    }

    pub fn write_bg_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.bg_control = self.bg_control.set_data(value, index)
    }

    pub fn read_text_x_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.text_x_offset.get_data(index)
    }

    pub fn write_text_x_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.text_x_offset = self.text_x_offset.set_data(value, index)
    }

    pub fn read_text_y_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.text_y_offset.get_data(index)
    }

    pub fn write_text_y_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.text_y_offset = self.text_y_offset.set_data(value, index)
    }

    pub fn write_affine_x_offset<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.affine_x_offset = self.affine_x_offset.set_data(value, index)
    }

    pub fn read_affine_x_offset<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.affine_x_offset.get_data(index)
    }

    pub fn write_affine_y_offset<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.affine_y_offset = self.affine_y_offset.set_data(value, index)
    }

    pub fn read_affine_y_offset<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.affine_y_offset.get_data(index)
    }

    pub fn read_affine_param_a<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.affine_a.get_data(index)
    }

    pub fn write_affine_param_a<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.affine_a = self.affine_a.set_data(value, index)
    }

    pub fn read_affine_param_b<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.affine_b.get_data(index)
    }

    pub fn write_affine_param_b<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.affine_b = self.affine_b.set_data(value, index)
    }

    pub fn read_affine_param_c<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.affine_c.get_data(index)
    }

    pub fn write_affine_param_c<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.affine_c = self.affine_c.set_data(value, index)
    }

    pub fn read_affine_param_d<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.affine_d.get_data(index)
    }

    pub fn write_affine_param_d<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.affine_d = self.affine_d.set_data(value, index)
    }
}

impl Layer2 {
    pub fn get_priority(&self) -> u16 {
        const BG_PRIORITY_BIT_RANGE: RangeInclusive<usize> = 0..=1;

        self.bg_control.get_bit_range(BG_PRIORITY_BIT_RANGE)
    }

    pub fn bg_tile_data_base(&self) -> u32 {
        const CHARACTER_BASE_BLOCK_BIT_RANGE: RangeInclusive<usize> = 2..=3;
        const CHARACTER_BASE_BLOCK_SIZE: u32 = 0x4000;

        let character_base_block_idx = self
            .bg_control
            .get_bit_range(CHARACTER_BASE_BLOCK_BIT_RANGE);

        u32::from(character_base_block_idx) * CHARACTER_BASE_BLOCK_SIZE
    }

    pub fn get_mosaic(&self) -> bool {
        const MOSAIC_BIT_INDEX: usize = 6;

        self.bg_control.get_bit(MOSAIC_BIT_INDEX)
    }

    pub fn get_palette_depth(&self) -> PaletteDepth {
        const BIT_DEPTH_BIT_INDEX: usize = 7;

        if self.bg_control.get_bit(BIT_DEPTH_BIT_INDEX) {
            PaletteDepth::EightBit
        } else {
            PaletteDepth::FourBit
        }
    }

    fn bg_map_data_base(&self) -> u32 {
        const SCREEN_BASE_BLOCK_BIT_RANGE: RangeInclusive<usize> = 8..=12;
        const SCREEN_BASE_BLOCK_SIZE: u32 = 0x800;

        let screen_base_block_idx = self.bg_control.get_bit_range(SCREEN_BASE_BLOCK_BIT_RANGE);
        u32::from(screen_base_block_idx) * SCREEN_BASE_BLOCK_SIZE
    }

    fn get_text_x_offset(&self) -> u16 {
        const X_OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=8;
        self.text_x_offset.get_bit_range(X_OFFSET_BIT_RANGE)
    }

    fn get_text_y_offset(&self) -> u16 {
        const Y_OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=8;
        self.text_y_offset.get_bit_range(Y_OFFSET_BIT_RANGE)
    }

    fn get_affine_x_offset(&self) -> f64 {
        Self::word_fixed_point_to_float(self.affine_x_offset)
    }

    fn get_affine_y_offset(&self) -> f64 {
        Self::word_fixed_point_to_float(self.affine_y_offset)
    }

    fn get_affine_param_a(&self) -> f64 {
        Self::half_word_fixed_point_to_float(self.affine_a)
    }

    fn get_affine_param_b(&self) -> f64 {
        Self::half_word_fixed_point_to_float(self.affine_b)
    }

    fn get_affine_param_c(&self) -> f64 {
        Self::half_word_fixed_point_to_float(self.affine_c)
    }

    fn get_affine_param_d(&self) -> f64 {
        Self::half_word_fixed_point_to_float(self.affine_d)
    }

    fn half_word_fixed_point_to_float(val: u16) -> f64 {
        const FRACTIONAL_BIT_RANGE: RangeInclusive<usize> = 0..=7;
        const INTEGER_BIT_RANGE: RangeInclusive<usize> = 8..=14;
        const SIGN_BIT_INDEX: usize = 15;

        let fractional_part = (val.get_bit_range(FRACTIONAL_BIT_RANGE) as f64) / 256.0;
        let integer_part = val.get_bit_range(INTEGER_BIT_RANGE) as f64;
        if val.get_bit(SIGN_BIT_INDEX) {
            -(fractional_part + integer_part)
        } else {
            fractional_part + integer_part
        }
    }

    fn word_fixed_point_to_float(val: u32) -> f64 {
        const FRACTIONAL_BIT_RANGE: RangeInclusive<usize> = 0..=7;
        const INTEGER_BIT_RANGE: RangeInclusive<usize> = 8..=26;
        const SIGN_BIT_INDEX: usize = 27;

        let fractional_part = (val.get_bit_range(FRACTIONAL_BIT_RANGE) as f64) / 256.0;
        let integer_part = val.get_bit_range(INTEGER_BIT_RANGE) as f64;
        if val.get_bit(SIGN_BIT_INDEX) {
            -(fractional_part + integer_part)
        } else {
            fractional_part + integer_part
        }
    }

    fn get_text_screen_size(&self) -> TextScreenSize {
        const SCREEN_SIZE_BIT_RANGE: RangeInclusive<usize> = 14..=15;

        match self.bg_control.get_bit_range(SCREEN_SIZE_BIT_RANGE) {
            0 => TextScreenSize::Size32x32,
            1 => TextScreenSize::Size64x32,
            2 => TextScreenSize::Size32x64,
            3 => TextScreenSize::Size64x64,
            _ => unreachable!(),
        }
    }
}
