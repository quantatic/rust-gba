mod layer_0;
mod layer_1;
mod layer_2;
mod layer_3;

use layer_0::Layer0;
use layer_1::Layer1;
use layer_2::Layer2;
use layer_3::Layer3;

use crate::{BitManipulation, DataAccess};

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
pub struct LcdStateChangeInfo {
    pub vblank_entered: bool,
    pub hblank_entered: bool,
    pub vcount_matched: bool,
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
enum PixelType {
    Layer0,
    Layer1,
    Layer2,
    Layer3,
    Sprite,
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

#[derive(Clone, Copy, Debug)]
enum AffineScreenSize {
    Size16x16,
    Size32x32,
    Size64x64,
    Size128x128,
}

#[derive(Clone, Copy, Debug)]
enum AffineDisplayOverflow {
    Transparent,
    Wraparound,
}

#[derive(Clone, Copy, Debug)]
enum ObjectShape {
    Square,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug)]
enum ObjectSize {
    Size8x8,
    Size16x16,
    Size32x32,
    Size64x64,
    Size16x8,
    Size32x8,
    Size32x16,
    Size64x32,
    Size8x16,
    Size8x32,
    Size16x32,
    Size32x64,
}

#[derive(Clone, Copy, Debug)]
enum ObjectTileMapping {
    OneDimensional,
    TwoDimensional,
}

impl ObjectSize {
    fn get_dimensions(self) -> (u16, u16) {
        match self {
            ObjectSize::Size8x8 => (1, 1),
            ObjectSize::Size16x16 => (2, 2),
            ObjectSize::Size32x32 => (4, 4),
            ObjectSize::Size64x64 => (8, 8),
            ObjectSize::Size16x8 => (2, 1),
            ObjectSize::Size32x8 => (4, 1),
            ObjectSize::Size32x16 => (4, 2),
            ObjectSize::Size64x32 => (8, 4),
            ObjectSize::Size8x16 => (1, 2),
            ObjectSize::Size8x32 => (1, 4),
            ObjectSize::Size16x32 => (2, 4),
            ObjectSize::Size32x64 => (4, 8),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rgb555 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone, Copy, Debug, Default)]
struct ObjectAttributeInfo {
    pub attribute_0: u16,
    pub attribute_1: u16,
    pub attribute_2: u16,
}

// attribute 0
impl ObjectAttributeInfo {
    fn get_y_coordinate(&self) -> u16 {
        const Y_COORDINATE_BIT_RANGE: RangeInclusive<usize> = 0..=7;

        self.attribute_0.get_bit_range(Y_COORDINATE_BIT_RANGE)
    }

    fn get_rotation_scaling_flag(&self) -> bool {
        const ROTATION_SCALING_FLAG_BIT_INDEX: usize = 8;

        self.attribute_0.get_bit(ROTATION_SCALING_FLAG_BIT_INDEX)
    }

    const DOUBLE_SIZE_OBJ_DISABLE_BIT_INDEX: usize = 9;

    fn get_double_size_flag(&self) -> bool {
        assert!(self.get_rotation_scaling_flag());

        self.attribute_0
            .get_bit(Self::DOUBLE_SIZE_OBJ_DISABLE_BIT_INDEX)
    }

    fn get_obj_disable_flag(&self) -> bool {
        assert!(!self.get_rotation_scaling_flag());

        self.attribute_0
            .get_bit(Self::DOUBLE_SIZE_OBJ_DISABLE_BIT_INDEX)
    }

    fn get_obj_mode(&self) -> () {
        todo!()
    }

    fn get_obj_mosaic(&self) -> bool {
        const OBJ_MOSIAIC_BIT_INDEX: usize = 12;

        self.attribute_0.get_bit(OBJ_MOSIAIC_BIT_INDEX)
    }

    fn get_palette_depth(&self) -> PaletteDepth {
        const PALETTE_DEPTH_BIT_INDEX: usize = 13;

        if self.attribute_0.get_bit(PALETTE_DEPTH_BIT_INDEX) {
            PaletteDepth::EightBit
        } else {
            PaletteDepth::FourBit
        }
    }

    fn get_obj_shape(&self) -> ObjectShape {
        const OBJ_SHAPE_BIT_RANGE: RangeInclusive<usize> = 14..=15;

        match self.attribute_0.get_bit_range(OBJ_SHAPE_BIT_RANGE) {
            0 => ObjectShape::Square,
            1 => ObjectShape::Horizontal,
            2 => ObjectShape::Vertical,
            _ => unreachable!(),
        }
    }
}

// attribute 1
impl ObjectAttributeInfo {
    fn get_x_coordinate(&self) -> u16 {
        const X_COORDINATE_BIT_RANGE: RangeInclusive<usize> = 0..=8;

        self.attribute_1.get_bit_range(X_COORDINATE_BIT_RANGE)
    }

    fn get_rotation_scaling_index(&self) -> u16 {
        assert!(self.get_rotation_scaling_flag());

        const ROTATION_SCALING_INDEX_BIT_RANGE: RangeInclusive<usize> = 9..=13;

        self.attribute_1
            .get_bit_range(ROTATION_SCALING_INDEX_BIT_RANGE)
    }

    fn get_horizontal_flip(&self) -> bool {
        assert!(!self.get_rotation_scaling_flag());

        const HORIZONTAL_FLIP_BIT_INDEX: usize = 12;

        self.attribute_1.get_bit(HORIZONTAL_FLIP_BIT_INDEX)
    }

    fn get_vertical_flip(&self) -> bool {
        assert!(!self.get_rotation_scaling_flag());

        const VERTICAL_FLIP_BIT_INDEX: usize = 13;

        self.attribute_1.get_bit(VERTICAL_FLIP_BIT_INDEX)
    }

    fn get_obj_size(&self) -> ObjectSize {
        const OBJ_SIZE_BIT_RANGE: RangeInclusive<usize> = 14..=15;

        match (
            self.get_obj_shape(),
            self.attribute_1.get_bit_range(OBJ_SIZE_BIT_RANGE),
        ) {
            (ObjectShape::Square, 0) => ObjectSize::Size8x8,
            (ObjectShape::Square, 1) => ObjectSize::Size16x16,
            (ObjectShape::Square, 2) => ObjectSize::Size32x32,
            (ObjectShape::Square, 3) => ObjectSize::Size64x64,
            (ObjectShape::Horizontal, 0) => ObjectSize::Size16x8,
            (ObjectShape::Horizontal, 1) => ObjectSize::Size32x8,
            (ObjectShape::Horizontal, 2) => ObjectSize::Size32x16,
            (ObjectShape::Horizontal, 3) => ObjectSize::Size64x32,
            (ObjectShape::Vertical, 0) => ObjectSize::Size8x16,
            (ObjectShape::Vertical, 1) => ObjectSize::Size8x32,
            (ObjectShape::Vertical, 2) => ObjectSize::Size16x32,
            (ObjectShape::Vertical, 3) => ObjectSize::Size32x64,
            _ => unreachable!(),
        }
    }
}

//attribute 2
impl ObjectAttributeInfo {
    fn get_tile_number(&self) -> u16 {
        const TILE_NUMBER_BIT_RANGE: RangeInclusive<usize> = 0..=9;

        self.attribute_2.get_bit_range(TILE_NUMBER_BIT_RANGE)
    }

    fn get_bg_priority(&self) -> u16 {
        const BG_PRIORITY_BIT_RANGE: RangeInclusive<usize> = 10..=11;

        self.attribute_2.get_bit_range(BG_PRIORITY_BIT_RANGE)
    }

    fn get_palette_number(&self) -> u8 {
        const PALETTE_NUMBER_BIT_RANGE: RangeInclusive<usize> = 12..=15;

        self.attribute_2.get_bit_range(PALETTE_NUMBER_BIT_RANGE) as u8
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ObjectRotationScalingInfo {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub d: u16,
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
    mosaic_size: u32,
    state: LcdState,
    bg_palette_ram: Box<[Rgb555; 0x100]>,
    obj_palette_ram: Box<[Rgb555; 0x100]>,
    vram: Box<[u8; 0x18000]>,
    obj_attributes: Box<[ObjectAttributeInfo; 0x100]>,
    obj_rotations: Box<[ObjectRotationScalingInfo; 0x40]>,
    buffer: Box<[[Rgb555; LCD_WIDTH]; LCD_HEIGHT]>, // access as buffer[y][x]
    back_buffer: Box<[[Rgb555; LCD_WIDTH]; LCD_HEIGHT]>,
    layer_0: Layer0,
    layer_1: Layer1,
    layer_2: Layer2,
    layer_3: Layer3,
}

fn half_word_fixed_point_to_float(val: u16) -> f64 {
    const VALUE_DIVIDED: f64 = 256.0;

    ((val as i16) as f64) / VALUE_DIVIDED
}

fn word_fixed_point_to_float(val: u32) -> f64 {
    const VALUE_BIT_RANGE: RangeInclusive<usize> = 0..=27;
    const VALUE_DIVIDED: f64 = 256.0;

    let raw_value = val.get_bit_range(VALUE_BIT_RANGE);
    // sign extend MSB by LSL sign bit to MSB, then ASR back to where we were before
    let signed_value = ((raw_value as i32) << 4) >> 4;

    (signed_value as f64) / VALUE_DIVIDED
}

impl Default for Lcd {
    fn default() -> Self {
        Self {
            dot: 0,
            vcount: 0,
            lcd_control: 0,
            lcd_status: 0,
            mosaic_size: 0,
            state: LcdState::Visible,
            bg_palette_ram: Box::new([Rgb555::default(); 0x100]),
            obj_palette_ram: Box::new([Rgb555::default(); 0x100]),
            vram: Box::new([0; 0x18000]),
            obj_attributes: Box::new([ObjectAttributeInfo::default(); 0x100]),
            obj_rotations: Box::new([ObjectRotationScalingInfo::default(); 0x40]),
            buffer: Box::new([[Rgb555::default(); LCD_WIDTH]; LCD_HEIGHT]),
            back_buffer: Box::new([[Rgb555::default(); LCD_WIDTH]; LCD_HEIGHT]),
            layer_0: Layer0::default(),
            layer_1: Layer1::default(),
            layer_2: Layer2::default(),
            layer_3: Layer3::default(),
        }
    }
}

impl Lcd {
    pub fn step(&mut self) -> LcdStateChangeInfo {
        let mut vblank_entered = false;
        let mut hblank_entered = false;
        let mut vcount_matched = false;

        if self.vcount < 160 {
            if self.dot == 0 {
                self.set_vblank_flag(false);
                self.set_hblank_flag(false);
                self.state = LcdState::Visible;
            } else if self.dot == 240 {
                hblank_entered = true;
                self.set_hblank_flag(true);
                self.state = LcdState::HBlank;
            }
        } else if self.vcount == 160 && self.dot == 0 {
            vblank_entered = true;
            self.set_vblank_flag(true);
            self.state = LcdState::VBlank;
            std::mem::swap(&mut self.buffer, &mut self.back_buffer);
        }

        if matches!(self.state, LcdState::Visible) {
            let pixel_x = self.dot;
            let pixel_y = self.vcount;

            let current_mode = self.get_bg_mode();
            let display_frame = self.get_display_frame();

            // if pixel_x == 0 && pixel_y == 0 {
            //     println!("{:?}", current_mode);
            //     println!(
            //         "{}, {}, {}, {}",
            //         self.get_screen_display_bg_0(),
            //         self.get_screen_display_bg_1(),
            //         self.get_screen_display_bg_2(),
            //         self.get_screen_display_bg_3()
            //     );
            // }

            let bg_mosaic_horizontal = self.get_bg_mosaic_horizontal();
            let bg_mosaic_vertical = self.get_bg_mosaic_vertical();

            let layer_0_pixel = if self.get_screen_display_bg_0() {
                self.layer_0
                    .get_pixel(
                        pixel_x,
                        pixel_y,
                        bg_mosaic_horizontal,
                        bg_mosaic_vertical,
                        current_mode,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|pixel| (pixel, self.layer_0.get_priority()))
            } else {
                None
            };
            let layer_0_pixel_info = (layer_0_pixel, PixelType::Layer0);

            let layer_1_pixel = if self.get_screen_display_bg_1() {
                self.layer_1
                    .get_pixel(
                        pixel_x,
                        pixel_y,
                        bg_mosaic_horizontal,
                        bg_mosaic_vertical,
                        current_mode,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|pixel| (pixel, self.layer_1.get_priority()))
            } else {
                None
            };
            let layer_1_pixel_info = (layer_1_pixel, PixelType::Layer1);

            let layer_2_pixel = if self.get_screen_display_bg_2() {
                self.layer_2
                    .get_pixel(
                        pixel_x,
                        pixel_y,
                        bg_mosaic_horizontal,
                        bg_mosaic_vertical,
                        current_mode,
                        display_frame,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|pixel| (pixel, self.layer_2.get_priority()))
            } else {
                None
            };
            let layer_2_pixel_info = (layer_2_pixel, PixelType::Layer2);

            let layer_3_pixel = if self.get_screen_display_bg_3() {
                self.layer_3
                    .get_pixel(
                        pixel_x,
                        pixel_y,
                        bg_mosaic_horizontal,
                        bg_mosaic_vertical,
                        current_mode,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|pixel| (pixel, self.layer_2.get_priority()))
            } else {
                None
            };
            let layer_3_pixel_info = (layer_3_pixel, PixelType::Layer3);

            let obj_mosaic_horizontal = self.get_obj_mosaic_horizontal();
            let obj_mosaic_vertical = self.get_obj_mosaic_vertical();
            let sprite_pixel =
                self.get_sprite_pixel(pixel_x, pixel_y, obj_mosaic_horizontal, obj_mosaic_vertical);
            let sprite_pixel_info = (sprite_pixel, PixelType::Sprite);

            let mut final_pixel = None;
            for pixel in [
                sprite_pixel,
                layer_0_pixel,
                layer_1_pixel,
                layer_2_pixel,
                layer_3_pixel,
            ] {
                final_pixel =
                    final_pixel.map_or(pixel, |(final_pixel_color, final_pixel_priority)| {
                        if let Some((current_pixel_color, current_pixel_priority)) = pixel {
                            if current_pixel_priority < final_pixel_priority {
                                return Some((current_pixel_color, current_pixel_priority));
                            }
                        }

                        Some((final_pixel_color, final_pixel_priority))
                    });
            }

            let drawn_pixel = match final_pixel {
                Some((pixel, _)) => pixel,
                None => self.bg_palette_ram[0],
            };

            self.back_buffer[usize::from(pixel_y)][usize::from(pixel_x)] = drawn_pixel;
        }

        self.dot += 1;

        if self.dot >= 308 {
            self.dot = 0;
            self.vcount += 1;

            if self.vcount >= 228 {
                self.vcount = 0;
            }

            if self.vcount == self.get_vcount_setting() {
                vcount_matched = true;
            }
        }

        LcdStateChangeInfo {
            hblank_entered,
            vblank_entered,
            vcount_matched,
        }
    }

    fn get_sprite_pixel(
        &self,
        pixel_x: u16,
        pixel_y: u16,
        obj_mosaic_horizontal: u16,
        obj_mosaic_vertical: u16,
    ) -> Option<(Rgb555, u16)> {
        const OBJ_TILE_DATA_VRAM_BASE: usize = 0x10000;
        const TILE_SIZE: u16 = 8;
        const WORLD_WIDTH: u16 = 512;
        const WORLD_HEIGHT: u16 = 256;

        for (i, obj) in self.obj_attributes.into_iter().enumerate() {
            let (sprite_tile_width, sprite_tile_height) = obj.get_obj_size().get_dimensions();
            let sprite_width = sprite_tile_width * TILE_SIZE;
            let sprite_height = sprite_tile_height * TILE_SIZE;

            let sprite_x = obj.get_x_coordinate();
            let sprite_y = obj.get_y_coordinate();

            let (sprite_offset_x, sprite_offset_y) = if obj.get_rotation_scaling_flag() {
                let rotation_info_idx = obj.get_rotation_scaling_index();
                let rotation_info = self.obj_rotations[usize::from(rotation_info_idx)];

                let a = half_word_fixed_point_to_float(rotation_info.a);
                let b = half_word_fixed_point_to_float(rotation_info.b);
                let c = half_word_fixed_point_to_float(rotation_info.c);
                let d = half_word_fixed_point_to_float(rotation_info.d);

                let center_offset_adjustment_x = sprite_width / 2;
                let center_offset_adjustment_y = sprite_height / 2;

                let mut base_corner_offset_x = f64::from(pixel_x) - f64::from(sprite_x);
                if base_corner_offset_x < -f64::from(WORLD_WIDTH / 2) {
                    base_corner_offset_x += f64::from(WORLD_WIDTH);
                }
                let base_corner_offset_x = base_corner_offset_x;

                let mut base_corner_offset_y = f64::from(pixel_y) - f64::from(sprite_y);
                if base_corner_offset_y < -f64::from(WORLD_HEIGHT / 2) {
                    base_corner_offset_y += f64::from(WORLD_HEIGHT);
                }
                let base_corner_offset_y = base_corner_offset_y;

                if obj.get_double_size_flag() {
                    if base_corner_offset_x < 0.0
                        || base_corner_offset_x >= (f64::from(sprite_width) * 2.0)
                        || base_corner_offset_y < 0.0
                        || base_corner_offset_y >= (f64::from(sprite_height) * 2.0)
                    {
                        continue;
                    }
                } else {
                    if base_corner_offset_x < 0.0
                        || base_corner_offset_x >= f64::from(sprite_width)
                        || base_corner_offset_y < 0.0
                        || base_corner_offset_y >= f64::from(sprite_height)
                    {
                        continue;
                    }
                }

                // In a double-sized sprite, where each square represents the size of an original sprite,
                //   we use "X" as the central reference point when performing transformations. We don't
                //   move this point all the way back to an offset of WxH, instead only moving it back to
                //   an offset of (W/2)x(H/2), as represented by the period ("."). This means that in
                //   double-sized mode, the effective origin of the drawn sprite is at the dot, instead of
                //   the top left of the below square. This has the effect of moving the drawn sprite over and
                //   down by half the width & height of the sprite.
                //     +---+---+
                //     | . |   |
                //     +---X---+
                //     |   |   |
                //     +---+---+
                let (base_center_offset_x, base_center_offset_y) = if obj.get_double_size_flag() {
                    (
                        f64::from(base_corner_offset_x)
                            - (2.0 * f64::from(center_offset_adjustment_x)),
                        f64::from(base_corner_offset_y)
                            - (2.0 * f64::from(center_offset_adjustment_y)),
                    )
                } else {
                    (
                        f64::from(base_corner_offset_x) - f64::from(center_offset_adjustment_x),
                        f64::from(base_corner_offset_y) - f64::from(center_offset_adjustment_y),
                    )
                };

                let center_offset_x = (base_center_offset_x * a) + (base_center_offset_y * b);
                let center_offset_y = (base_center_offset_x * c) + (base_center_offset_y * d);

                let corner_offset_x = center_offset_x + f64::from(center_offset_adjustment_x);
                let corner_offset_y = center_offset_y + f64::from(center_offset_adjustment_y);

                if corner_offset_x < 0.0
                    || corner_offset_x >= f64::from(sprite_width)
                    || corner_offset_y < 0.0
                    || corner_offset_y >= f64::from(sprite_height)
                {
                    continue;
                }

                (corner_offset_x as u16, corner_offset_y as u16)
            } else {
                let mut base_corner_offset_x = f64::from(pixel_x) - f64::from(sprite_x);
                let mut base_corner_offset_y = f64::from(pixel_y) - f64::from(sprite_y);

                if base_corner_offset_x < 0.0
                    || base_corner_offset_x >= f64::from(sprite_width)
                    || base_corner_offset_y < 0.0
                    || base_corner_offset_y >= f64::from(sprite_height)
                {
                    continue;
                }

                if obj.get_obj_mosaic() {
                    base_corner_offset_x -= base_corner_offset_x % f64::from(obj_mosaic_horizontal);
                    base_corner_offset_y -= base_corner_offset_y % f64::from(obj_mosaic_vertical);
                }
                let base_corner_offset_x = base_corner_offset_x;
                let base_corner_offset_y = base_corner_offset_y;

                let offset_x = if obj.get_horizontal_flip() {
                    f64::from(sprite_width) - 1.0 - base_corner_offset_x
                } else {
                    base_corner_offset_x
                };

                let offset_y = if obj.get_vertical_flip() {
                    f64::from(sprite_height) - 1.0 - base_corner_offset_y
                } else {
                    base_corner_offset_y
                };

                (offset_x as u16, offset_y as u16)
            };

            assert!(sprite_offset_x < sprite_width);
            assert!(sprite_offset_y < sprite_height);

            let palette_depth = obj.get_palette_depth();

            let sprite_tile_x = sprite_offset_x / 8;
            let sprite_tile_y = sprite_offset_y / 8;

            let tile_offset_x = sprite_offset_x % 8;
            let tile_offset_y = sprite_offset_y % 8;

            let base_tile_number = obj.get_tile_number();

            let tile_number = match (self.get_obj_tile_mapping(), palette_depth) {
                (ObjectTileMapping::OneDimensional, PaletteDepth::FourBit) => {
                    base_tile_number + (sprite_tile_y * sprite_tile_width) + sprite_tile_x
                }
                (ObjectTileMapping::OneDimensional, PaletteDepth::EightBit) => {
                    base_tile_number + (sprite_tile_y * sprite_tile_width) + (sprite_tile_x * 2)
                }
                (ObjectTileMapping::TwoDimensional, PaletteDepth::FourBit) => {
                    base_tile_number + (sprite_tile_y * 32) + sprite_tile_x
                }
                (ObjectTileMapping::TwoDimensional, PaletteDepth::EightBit) => {
                    base_tile_number + (sprite_tile_y * 32) + (sprite_tile_x * 2)
                }
            };

            let palette_idx = match obj.get_palette_depth() {
                PaletteDepth::EightBit => {
                    let tile_idx = OBJ_TILE_DATA_VRAM_BASE
                        + (usize::from(tile_number) * 32)
                        + (usize::from(tile_offset_y) * 8)
                        + usize::from(tile_offset_x);

                    let palette_idx = self.vram[tile_idx];

                    if palette_idx == 0 {
                        continue;
                    }

                    palette_idx
                }
                PaletteDepth::FourBit => {
                    let tile_idx = OBJ_TILE_DATA_VRAM_BASE
                        + (usize::from(tile_number) * 32)
                        + (usize::from(tile_offset_y) * 4)
                        + (usize::from(tile_offset_x) / 2);

                    let tile_data = self.vram[tile_idx];

                    let palette_idx_low = if sprite_tile_x % 2 == 0 {
                        tile_data.get_bit_range(0..=3)
                    } else {
                        tile_data.get_bit_range(4..=7)
                    };

                    if palette_idx_low == 0 {
                        continue;
                    }

                    palette_idx_low.set_bit_range(obj.get_palette_number(), 4..=7)
                }
            };

            return Some((
                self.obj_palette_ram[usize::from(palette_idx)],
                obj.get_bg_priority(),
            ));
        }

        None
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

    pub fn read_mosaic_size<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.mosaic_size.get_data(index)
    }

    pub fn write_mosaic_size<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.mosaic_size = self.mosaic_size.set_data(value, index)
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
        let hword_offset = offset / 2;

        let oam_index = (hword_offset / 4) as usize;
        let oam_offset = hword_offset % 4;

        let rotation_group_index = (hword_offset / 16) as usize;
        let rotation_group_offset = (hword_offset / 4) % 4;

        let hword_result = match oam_offset {
            0 => self.obj_attributes[oam_index].attribute_0,
            1 => self.obj_attributes[oam_index].attribute_1,
            2 => self.obj_attributes[oam_index].attribute_2,
            3 => match rotation_group_offset {
                0 => self.obj_rotations[rotation_group_index].a,
                1 => self.obj_rotations[rotation_group_index].b,
                2 => self.obj_rotations[rotation_group_index].c,
                3 => self.obj_rotations[rotation_group_index].d,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        let hword_index = offset % 2;
        hword_result.get_data(hword_index)
    }

    pub fn write_oam(&mut self, value: u8, offset: u32) {
        let hword_offset = offset / 2;

        let oam_index = (hword_offset / 4) as usize;
        let oam_offset = hword_offset % 4;

        let rotation_group_index = (hword_offset / 16) as usize;
        let rotation_group_offset = (hword_offset / 4) % 4;

        let hword_index = offset % 2;

        match oam_offset {
            0 => {
                self.obj_attributes[oam_index].attribute_0 = self.obj_attributes[oam_index]
                    .attribute_0
                    .set_data(value, hword_index)
            }
            1 => {
                self.obj_attributes[oam_index].attribute_1 = self.obj_attributes[oam_index]
                    .attribute_1
                    .set_data(value, hword_index)
            }
            2 => {
                self.obj_attributes[oam_index].attribute_2 = self.obj_attributes[oam_index]
                    .attribute_2
                    .set_data(value, hword_index)
            }
            3 => match rotation_group_offset {
                0 => {
                    self.obj_rotations[rotation_group_index].a = self.obj_rotations
                        [rotation_group_index]
                        .a
                        .set_data(value, hword_index)
                }
                1 => {
                    self.obj_rotations[rotation_group_index].b = self.obj_rotations
                        [rotation_group_index]
                        .b
                        .set_data(value, hword_index)
                }
                2 => {
                    self.obj_rotations[rotation_group_index].c = self.obj_rotations
                        [rotation_group_index]
                        .c
                        .set_data(value, hword_index)
                }
                3 => {
                    self.obj_rotations[rotation_group_index].d = self.obj_rotations
                        [rotation_group_index]
                        .d
                        .set_data(value, hword_index)
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
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

    pub fn read_layer1_bg_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_1.read_bg_control(index)
    }

    pub fn write_layer1_bg_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_1.write_bg_control(value, index);
    }

    pub fn read_layer1_x_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_1.read_x_offset(index)
    }

    pub fn write_layer1_x_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_1.write_x_offset(value, index);
    }

    pub fn read_layer1_y_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_1.read_y_offset(index)
    }

    pub fn write_layer1_y_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_1.write_y_offset(value, index);
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

    pub fn read_layer2_affine_x_offset<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.layer_2.read_affine_x_offset(index)
    }

    pub fn write_layer2_affine_x_offset<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.layer_2.write_affine_x_offset(value, index)
    }

    pub fn read_layer2_affine_y_offset<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.layer_2.read_affine_y_offset(index)
    }

    pub fn write_layer2_affine_y_offset<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.layer_2.write_affine_y_offset(value, index)
    }

    pub fn read_layer2_affine_param_a<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_2.read_affine_param_a(index)
    }

    pub fn write_layer2_affine_param_a<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_2.write_affine_param_a(value, index)
    }

    pub fn read_layer2_affine_param_b<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_2.read_affine_param_b(index)
    }

    pub fn write_layer2_affine_param_b<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_2.write_affine_param_b(value, index)
    }

    pub fn read_layer2_affine_param_c<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_2.read_affine_param_c(index)
    }

    pub fn write_layer2_affine_param_c<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_2.write_affine_param_c(value, index)
    }

    pub fn read_layer2_affine_param_d<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_2.read_affine_param_d(index)
    }

    pub fn write_layer2_affine_param_d<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_2.write_affine_param_d(value, index)
    }

    pub fn read_layer3_bg_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_3.read_bg_control(index)
    }

    pub fn write_layer3_bg_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_3.write_bg_control(value, index);
    }

    pub fn read_layer3_text_x_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_3.read_text_x_offset(index)
    }

    pub fn write_layer3_text_x_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_3.write_text_x_offset(value, index);
    }

    pub fn read_layer3_text_y_offset<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_3.read_text_y_offset(index)
    }

    pub fn write_layer3_text_y_offset<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_3.write_text_y_offset(value, index);
    }

    pub fn read_layer3_affine_x_offset<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.layer_3.read_affine_x_offset(index)
    }

    pub fn write_layer3_affine_x_offset<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.layer_3.write_affine_x_offset(value, index)
    }

    pub fn read_layer3_affine_y_offset<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.layer_3.read_affine_y_offset(index)
    }

    pub fn write_layer3_affine_y_offset<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.layer_3.write_affine_y_offset(value, index)
    }

    pub fn read_layer3_affine_param_a<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_3.read_affine_param_a(index)
    }

    pub fn write_layer3_affine_param_a<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_3.write_affine_param_a(value, index)
    }

    pub fn read_layer3_affine_param_b<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_3.read_affine_param_b(index)
    }

    pub fn write_layer3_affine_param_b<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_3.write_affine_param_b(value, index)
    }

    pub fn read_layer3_affine_param_c<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_3.read_affine_param_c(index)
    }

    pub fn write_layer3_affine_param_c<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_3.write_affine_param_c(value, index)
    }

    pub fn read_layer3_affine_param_d<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.layer_3.read_affine_param_d(index)
    }

    pub fn write_layer3_affine_param_d<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.layer_3.write_affine_param_d(value, index)
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

    fn get_obj_tile_mapping(&self) -> ObjectTileMapping {
        const TILE_MAPPING_BIT_INDEX: usize = 6;

        if self.lcd_control.get_bit(TILE_MAPPING_BIT_INDEX) {
            ObjectTileMapping::OneDimensional
        } else {
            ObjectTileMapping::TwoDimensional
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

    pub fn get_vblank_irq_enable(&self) -> bool {
        const VBLANK_IRQ_ENABLE_BIT_INDEX: usize = 3;

        self.lcd_status.get_bit(VBLANK_IRQ_ENABLE_BIT_INDEX)
    }

    pub fn get_hblank_irq_enable(&self) -> bool {
        const HBLANK_IRQ_ENABLE_BIT_INDEX: usize = 4;

        self.lcd_status.get_bit(HBLANK_IRQ_ENABLE_BIT_INDEX)
    }

    pub fn get_vcount_irq_enable(&self) -> bool {
        const VCOUNT_IRQ_ENABLE_BIT_INDEX: usize = 5;

        self.lcd_status.get_bit(VCOUNT_IRQ_ENABLE_BIT_INDEX)
    }

    fn get_vcount_setting(&self) -> u16 {
        const VCOUNT_SETTING_BIT_RANGE: RangeInclusive<usize> = 8..=15;

        self.lcd_status.get_bit_range(VCOUNT_SETTING_BIT_RANGE)
    }

    fn get_bg_mosaic_horizontal(&self) -> u16 {
        const BG_MOSAIC_HORIZONTAL_SIZE_BIT_RANGE: RangeInclusive<usize> = 0..=3;

        (self
            .mosaic_size
            .get_bit_range(BG_MOSAIC_HORIZONTAL_SIZE_BIT_RANGE)
            + 1) as u16
    }

    fn get_bg_mosaic_vertical(&self) -> u16 {
        const BG_MOSAIC_VERTICAL_SIZE_BIT_RANGE: RangeInclusive<usize> = 4..=7;

        (self
            .mosaic_size
            .get_bit_range(BG_MOSAIC_VERTICAL_SIZE_BIT_RANGE)
            + 1) as u16
    }

    fn get_obj_mosaic_horizontal(&self) -> u16 {
        const OBJ_MOSAIC_HORIZONTAL_SIZE_BIT_RANGE: RangeInclusive<usize> = 8..=11;

        (self
            .mosaic_size
            .get_bit_range(OBJ_MOSAIC_HORIZONTAL_SIZE_BIT_RANGE)
            + 1) as u16
    }

    fn get_obj_mosaic_vertical(&self) -> u16 {
        const OBJ_MOSAIC_VERTICAL_SIZE_BIT_RANGE: RangeInclusive<usize> = 12..=15;

        (self
            .mosaic_size
            .get_bit_range(OBJ_MOSAIC_VERTICAL_SIZE_BIT_RANGE)
            + 1) as u16
    }
}

impl Lcd {
    pub fn get_buffer(&self) -> &[[Rgb555; LCD_WIDTH]; LCD_HEIGHT] {
        &self.buffer
    }
}
