use std::ops::RangeInclusive;

use crate::{BitManipulation, DataAccess};

use super::{BgMode, PaletteDepth, Rgb555, TextScreenSize};

#[derive(Debug, Default)]
pub(super) struct Layer0 {
    bg_control: u16,
    x_offset: u16,
    y_offset: u16,
}

impl Layer0 {
    pub fn get_pixel(
        &self,
        pixel_x: u16,
        pixel_y: u16,
        mosaic_horizontal: u16,
        mosaic_vertical: u16,
        mode: BgMode,
        vram: &[u8],
        bg_palette: &[Rgb555],
    ) -> Option<Rgb555> {
        match mode {
            BgMode::Mode0 | BgMode::Mode1 => {
                let mut x = pixel_x + self.get_x_offset();
                let mut y = pixel_y + self.get_y_offset();

                if self.get_mosaic() {
                    x -= x % mosaic_horizontal;
                    y -= y % mosaic_vertical;
                }
                let x = x;
                let y = y;

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

                let tile_data_x = if horizontal_flip {
                    7 - usize::from(x % 8)
                } else {
                    usize::from(x % 8)
                };

                let tile_data_y = if vertical_flip {
                    7 - usize::from(y % 8)
                } else {
                    usize::from(y % 8)
                };

                let palette_idx = match self.get_palette_depth() {
                    PaletteDepth::EightBit => {
                        let tile_idx = tile_data_base
                            + (64 * usize::from(tile_number))
                            + (tile_data_y * 8)
                            + tile_data_x;

                        let palette_idx = vram[tile_idx];

                        if palette_idx == 0 {
                            return None;
                        }

                        palette_idx
                    }
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

                        if palette_idx_low == 0 {
                            return None;
                        }

                        palette_idx_low.set_bit_range(palette_number, 4..=7)
                    }
                };

                Some(bg_palette[usize::from(palette_idx)])
            }
            _ => None,
        }
    }
}

impl Layer0 {
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

    pub fn read_x_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.x_offset.get_data(index)
    }

    pub fn write_x_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.x_offset = self.x_offset.set_data(value, index)
    }

    pub fn read_y_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.y_offset.get_data(index)
    }

    pub fn write_y_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.y_offset = self.y_offset.set_data(value, index)
    }
}

impl Layer0 {
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

    pub fn bg_map_data_base(&self) -> u32 {
        const SCREEN_BASE_BLOCK_BIT_RANGE: RangeInclusive<usize> = 8..=12;
        const SCREEN_BASE_BLOCK_SIZE: u32 = 0x800;

        let screen_base_block_idx = self.bg_control.get_bit_range(SCREEN_BASE_BLOCK_BIT_RANGE);
        u32::from(screen_base_block_idx) * SCREEN_BASE_BLOCK_SIZE
    }

    pub fn get_x_offset(&self) -> u16 {
        const X_OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=8;
        self.x_offset.get_bit_range(X_OFFSET_BIT_RANGE)
    }

    pub fn get_y_offset(&self) -> u16 {
        const Y_OFFSET_BIT_RANGE: RangeInclusive<usize> = 0..=8;
        self.y_offset.get_bit_range(Y_OFFSET_BIT_RANGE)
    }

    pub fn get_text_screen_size(&self) -> TextScreenSize {
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
