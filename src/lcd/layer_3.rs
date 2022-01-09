use std::ops::RangeInclusive;

use crate::{BitManipulation, DataAccess};

use super::{
    half_word_fixed_point_to_float, word_fixed_point_to_float, AffineDisplayOverflow,
    AffineScreenSize, BgMode, DisplayFrame, PaletteDepth, Rgb555, TextScreenSize,
};

#[derive(Debug, Default)]
pub(super) struct Layer3 {
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

impl Layer3 {
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
            BgMode::Mode0 => {
                // text mode

                let mut x = pixel_x + self.get_text_x_offset();
                let mut y = pixel_y + self.get_text_y_offset();

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
            BgMode::Mode2 => {
                // affine mode

                let x = f64::from(pixel_x);
                let y = f64::from(pixel_y);

                let a = self.get_affine_param_a();
                let b = self.get_affine_param_b();
                let c = self.get_affine_param_c();
                let d = self.get_affine_param_d();

                let actual_x = (x * a) + (y * b) + self.get_affine_x_offset();
                let actual_y = (x * c) + (y * d) + self.get_affine_y_offset();

                let map_data_base = self.bg_map_data_base() as usize;
                let tile_data_base = self.bg_tile_data_base() as usize;

                let map_data_x = actual_x / 8.0;
                let map_data_y = actual_y / 8.0;

                let map_tiles: u16 = match self.get_affine_screen_size() {
                    AffineScreenSize::Size16x16 => 16,
                    AffineScreenSize::Size32x32 => 32,
                    AffineScreenSize::Size64x64 => 64,
                    AffineScreenSize::Size128x128 => 128,
                };

                let (actual_map_data_x, actual_map_data_y) =
                    match self.get_affine_display_area_overflow() {
                        AffineDisplayOverflow::Transparent => {
                            if map_data_x < 0.0
                                || map_data_x >= f64::from(map_tiles)
                                || map_data_y < 0.0
                                || map_data_y >= f64::from(map_tiles)
                            {
                                return Some(Rgb555::default());
                            } else {
                                (map_data_x as usize, map_data_y as usize)
                            }
                        }
                        AffineDisplayOverflow::Wraparound => {
                            let wrapped_map_data_x = map_data_x.rem_euclid(f64::from(map_tiles));
                            let wrapped_map_data_y = map_data_y.rem_euclid(f64::from(map_tiles));

                            (wrapped_map_data_x as usize, wrapped_map_data_y as usize)
                        }
                    };

                let map_data_offset =
                    (actual_map_data_y * usize::from(map_tiles)) + actual_map_data_x;

                let map_data_idx = map_data_base + map_data_offset;

                let map_data = vram[map_data_idx];
                let tile_number = map_data.get_bit_range(0..=7);

                let tile_data_x = actual_x.rem_euclid(8.0) as usize;
                let tile_data_y = actual_y.rem_euclid(8.0) as usize;

                let tile_idx = tile_data_base
                    + (usize::from(tile_number) * 64)
                    + (tile_data_y * 8)
                    + tile_data_x;

                let palette_idx = vram[tile_idx];

                if palette_idx == 0 {
                    return None;
                }

                Some(bg_palette[usize::from(palette_idx)])
            }
            _ => None,
        }
    }
}

impl Layer3 {
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

impl Layer3 {
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

    fn get_affine_display_area_overflow(&self) -> AffineDisplayOverflow {
        const AFFINE_DISPLAY_AREA_OVERFLOW_BIT_INDEX: usize = 13;

        if self
            .bg_control
            .get_bit(AFFINE_DISPLAY_AREA_OVERFLOW_BIT_INDEX)
        {
            AffineDisplayOverflow::Wraparound
        } else {
            AffineDisplayOverflow::Transparent
        }
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
        word_fixed_point_to_float(self.affine_x_offset)
    }

    fn get_affine_y_offset(&self) -> f64 {
        word_fixed_point_to_float(self.affine_y_offset)
    }

    fn get_affine_param_a(&self) -> f64 {
        half_word_fixed_point_to_float(self.affine_a)
    }

    fn get_affine_param_b(&self) -> f64 {
        half_word_fixed_point_to_float(self.affine_b)
    }

    fn get_affine_param_c(&self) -> f64 {
        half_word_fixed_point_to_float(self.affine_c)
    }

    fn get_affine_param_d(&self) -> f64 {
        half_word_fixed_point_to_float(self.affine_d)
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

    fn get_affine_screen_size(&self) -> AffineScreenSize {
        const SCREEN_SIZE_BIT_RANGE: RangeInclusive<usize> = 14..=15;

        match self.bg_control.get_bit_range(SCREEN_SIZE_BIT_RANGE) {
            0 => AffineScreenSize::Size16x16,
            1 => AffineScreenSize::Size32x32,
            2 => AffineScreenSize::Size64x64,
            3 => AffineScreenSize::Size128x128,
            _ => unreachable!(),
        }
    }
}
