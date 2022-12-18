mod layer_0;
mod layer_1;
mod layer_2;
mod layer_3;

use layer_0::Layer0;
use layer_1::Layer1;
use layer_2::Layer2;
use layer_3::Layer3;

use crate::{BitManipulation, DataAccess};

use std::{cmp::Ordering, fmt::Debug, ops::RangeInclusive};

#[derive(Clone, Debug)]
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
enum BgModeType {
    TileMode,
    BitmapMode,
    Invalid,
}

#[derive(Clone, Copy, Debug)]
enum BgMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
    Mode4,
    Mode5,
    Invalid,
}

impl BgMode {
    fn get_type(self) -> BgModeType {
        match self {
            Self::Mode0 | Self::Mode1 | Self::Mode2 => BgModeType::TileMode,
            Self::Mode3 | Self::Mode4 | Self::Mode5 => BgModeType::BitmapMode,
            Self::Invalid => BgModeType::Invalid,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct PixelInfo {
    priority: u16,
    color: Rgb555,
    pixel_type: PixelType,
}

#[derive(Copy, Clone, Debug)]
struct SpritePixelInfo {
    pixel_info: PixelInfo,
    semi_transparent: bool,
}

#[derive(Copy, Clone, Debug)]
struct SpritePixelQueryInfo {
    sprite_pixel_info: Option<SpritePixelInfo>,
    obj_window: bool,
}

#[derive(Clone, Copy, Debug)]
enum PixelType {
    Layer0,
    Layer1,
    Layer2,
    Layer3,
    Sprite,
    Backdrop,
}

#[derive(Clone, Copy, Debug)]
enum ColorSpecialEffect {
    None,
    AlphaBlending,
    BrightnessIncrease,
    BrightnessDecrease,
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

impl Default for PaletteDepth {
    fn default() -> Self {
        PaletteDepth::EightBit
    }
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
    Prohibited,
}

impl Default for ObjectShape {
    fn default() -> Self {
        ObjectShape::Square
    }
}

#[derive(Clone, Copy, Debug)]
enum ObjMode {
    Normal,
    SemiTransparent,
    ObjWindow,
}

impl Default for ObjMode {
    fn default() -> Self {
        ObjMode::Normal
    }
}

#[derive(Clone, Copy, Debug)]
enum ObjectTileMapping {
    OneDimensional,
    TwoDimensional,
}

#[derive(Clone, Copy, Debug)]
struct DisplayedSelectionInfo {
    bg0_displayed: bool,
    bg1_displayed: bool,
    bg2_displayed: bool,
    bg3_displayed: bool,
    obj_displayed: bool,
    effects_displayed: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Rgb555(u16);

impl Rgb555 {
    const RED_INTENSITY_BIT_RANGE: RangeInclusive<usize> = 0..=4;
    const GREEN_INTENSITY_BIT_RANGE: RangeInclusive<usize> = 5..=9;
    const BLUE_INTENSITY_BIT_RANGE: RangeInclusive<usize> = 10..=14;

    fn new(red: u8, green: u8, blue: u8) -> Self {
        let inner = 0
            .set_bit_range(u16::from(red), Self::RED_INTENSITY_BIT_RANGE)
            .set_bit_range(u16::from(green), Self::GREEN_INTENSITY_BIT_RANGE)
            .set_bit_range(u16::from(blue), Self::BLUE_INTENSITY_BIT_RANGE);

        Self(inner)
    }

    fn to_int(self) -> u16 {
        self.0
    }

    fn from_int(val: u16) -> Self {
        Self(val)
    }

    pub fn red(&self) -> u8 {
        self.0.get_bit_range(Rgb555::RED_INTENSITY_BIT_RANGE) as u8
    }

    pub fn green(&self) -> u8 {
        self.0.get_bit_range(Rgb555::GREEN_INTENSITY_BIT_RANGE) as u8
    }

    pub fn blue(&self) -> u8 {
        self.0.get_bit_range(Rgb555::BLUE_INTENSITY_BIT_RANGE) as u8
    }

    const MAX_VALUE: u8 = 31;

    fn blend(self, coeff_self: f64, other: Rgb555, coeff_other: f64) -> Self {
        let new_red =
            ((f64::from(self.red()) * coeff_self) + (f64::from(other.red()) * coeff_other)) as u8;
        let new_green = ((f64::from(self.green()) * coeff_self)
            + (f64::from(other.green()) * coeff_other)) as u8;
        let new_blue =
            ((f64::from(self.blue()) * coeff_self) + (f64::from(other.blue()) * coeff_other)) as u8;

        Self::new(
            new_red.min(Self::MAX_VALUE),
            new_green.min(Self::MAX_VALUE),
            new_blue.min(Self::MAX_VALUE),
        )
    }
}

impl Debug for Rgb555 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rgb555")
            .field("red", &self.red())
            .field("green", &self.green())
            .field("blue", &self.blue())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ObjectAttributeInfo {
    attribute_0: u16,
    attribute_1: u16,
    attribute_2: u16,
    tile_dims_cache: Option<(u16, u16)>,
}

// attribute 0
impl ObjectAttributeInfo {
    const Y_COORDINATE_BIT_RANGE: RangeInclusive<usize> = 0..=7;
    const ROTATION_SCALING_FLAG_BIT_INDEX: usize = 8;
    const DOUBLE_SIZE_OBJ_DISABLE_BIT_INDEX: usize = 9;
    const OBJ_MODE_BIT_RANGE: RangeInclusive<usize> = 10..=11;
    const OBJ_MOSIAIC_BIT_INDEX: usize = 12;
    const PALETTE_DEPTH_BIT_INDEX: usize = 13;
    const OBJ_SHAPE_BIT_RANGE: RangeInclusive<usize> = 14..=15;

    const OBJ_MODE_NORMAL: u16 = 0;
    const OBJ_MODE_SEMI_TRANSPARENT: u16 = 1;
    const OBJ_MODE_OBJ_WINDOW: u16 = 2;
    const OBJ_MODE_PROHIBITED: u16 = 3;

    const OBJ_SHAPE_SQUARE: u16 = 0;
    const OBJ_SHAPE_HORIZONTAL: u16 = 1;
    const OBJ_SHAPE_VERTICAL: u16 = 2;
    const OBJ_SHAPE_PROHIBITED: u16 = 3;

    fn read_attribute_0<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.attribute_0.get_data(index)
    }

    fn write_attribute_0<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.attribute_0 = self.attribute_0.set_data(value, index);
        self.update_cache();
    }
}

// attribute 1
impl ObjectAttributeInfo {
    const X_COORDINATE_BIT_RANGE: RangeInclusive<usize> = 0..=8;
    const ROTATION_SCALING_INDEX_BIT_RANGE: RangeInclusive<usize> = 9..=13;
    const OBJ_SIZE_BIT_RANGE: RangeInclusive<usize> = 14..=15;

    fn read_attribute_1<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.attribute_1.get_data(index)
    }

    fn write_attribute_1<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.attribute_1 = self.attribute_1.set_data(value, index);
        self.update_cache();
    }
}

//attribute 2
impl ObjectAttributeInfo {
    const TILE_NUMBER_BIT_RANGE: RangeInclusive<usize> = 0..=9;
    const BG_PRIORITY_BIT_RANGE: RangeInclusive<usize> = 10..=11;
    const PALETTE_NUMBER_BIT_RANGE: RangeInclusive<usize> = 12..=15;

    fn read_attribute_2<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.attribute_2.get_data(index)
    }

    fn write_attribute_2<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.attribute_2 = self.attribute_2.set_data(value, index);
        self.update_cache();
    }
}

impl ObjectAttributeInfo {
    fn get_obj_shape(&self) -> ObjectShape {
        match self.attribute_0.get_bit_range(Self::OBJ_SHAPE_BIT_RANGE) {
            Self::OBJ_SHAPE_SQUARE => ObjectShape::Square,
            Self::OBJ_SHAPE_HORIZONTAL => ObjectShape::Horizontal,
            Self::OBJ_SHAPE_VERTICAL => ObjectShape::Vertical,
            Self::OBJ_SHAPE_PROHIBITED => ObjectShape::Prohibited,
            _ => unreachable!(),
        }
    }

    fn get_y_coordinate(&self) -> u16 {
        self.attribute_0.get_bit_range(Self::Y_COORDINATE_BIT_RANGE)
    }

    fn get_rotation_scaling_flag(&self) -> bool {
        self.attribute_0
            .get_bit(Self::ROTATION_SCALING_FLAG_BIT_INDEX)
    }

    fn get_double_size_flag(&self) -> bool {
        self.attribute_0
            .get_bit(Self::DOUBLE_SIZE_OBJ_DISABLE_BIT_INDEX)
    }

    fn get_obj_disable_flag(&self) -> bool {
        self.attribute_0
            .get_bit(Self::DOUBLE_SIZE_OBJ_DISABLE_BIT_INDEX)
    }

    fn get_obj_mode(&self) -> ObjMode {
        match self.attribute_0.get_bit_range(Self::OBJ_MODE_BIT_RANGE) {
            Self::OBJ_MODE_NORMAL => ObjMode::Normal,
            Self::OBJ_MODE_SEMI_TRANSPARENT => ObjMode::SemiTransparent,
            Self::OBJ_MODE_OBJ_WINDOW => ObjMode::ObjWindow,
            Self::OBJ_MODE_PROHIBITED => unreachable!("prohibited object mode"),
            _ => unreachable!(),
        }
    }

    fn get_obj_mosaic(&self) -> bool {
        self.attribute_0.get_bit(Self::OBJ_MOSIAIC_BIT_INDEX)
    }

    fn get_palette_depth(&self) -> PaletteDepth {
        if self.attribute_0.get_bit(Self::PALETTE_DEPTH_BIT_INDEX) {
            PaletteDepth::EightBit
        } else {
            PaletteDepth::FourBit
        }
    }

    fn get_x_coordinate(&self) -> u16 {
        self.attribute_1.get_bit_range(Self::X_COORDINATE_BIT_RANGE)
    }

    fn get_rotation_scaling_index(&self) -> u16 {
        self.attribute_1
            .get_bit_range(Self::ROTATION_SCALING_INDEX_BIT_RANGE)
    }

    fn get_horizontal_flip(&self) -> bool {
        const HORIZONTAL_FLIP_BIT_INDEX: usize = 12;
        self.attribute_1.get_bit(HORIZONTAL_FLIP_BIT_INDEX)
    }

    fn get_vertical_flip(&self) -> bool {
        const VERTICAL_FLIP_BIT_INDEX: usize = 13;
        self.attribute_1.get_bit(VERTICAL_FLIP_BIT_INDEX)
    }

    fn get_tile_number(&self) -> u16 {
        self.attribute_2.get_bit_range(Self::TILE_NUMBER_BIT_RANGE)
    }

    fn get_bg_priority(&self) -> u16 {
        self.attribute_2.get_bit_range(Self::BG_PRIORITY_BIT_RANGE)
    }

    fn get_palette_number(&self) -> u8 {
        self.attribute_2
            .get_bit_range(Self::PALETTE_NUMBER_BIT_RANGE) as u8
    }

    fn get_obj_tile_dims(&self) -> Option<(u16, u16)> {
        self.tile_dims_cache
    }
}

impl ObjectAttributeInfo {
    // This MUST be called whenever any attribute values are updated.
    fn update_cache(&mut self) {
        let obj_size_index = self.attribute_1.get_bit_range(Self::OBJ_SIZE_BIT_RANGE);

        self.tile_dims_cache = match (obj_size_index, self.get_obj_shape()) {
            (0, ObjectShape::Square) => Some((1, 1)),
            (1, ObjectShape::Square) => Some((2, 2)),
            (2, ObjectShape::Square) => Some((4, 4)),
            (3, ObjectShape::Square) => Some((8, 8)),
            (0, ObjectShape::Horizontal) => Some((2, 1)),
            (1, ObjectShape::Horizontal) => Some((4, 1)),
            (2, ObjectShape::Horizontal) => Some((4, 2)),
            (3, ObjectShape::Horizontal) => Some((8, 4)),
            (0, ObjectShape::Vertical) => Some((1, 2)),
            (1, ObjectShape::Vertical) => Some((1, 4)),
            (2, ObjectShape::Vertical) => Some((2, 4)),
            (3, ObjectShape::Vertical) => Some((4, 8)),
            (_, ObjectShape::Prohibited) => {
                log::warn!("found a prohibited object shape");
                None
            }
            _ => unreachable!(),
        };
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ObjectRotationScalingInfo {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub d: u16,
}

#[derive(Clone, Debug)]
pub struct Lcd {
    dot: u16,
    vcount: u16,
    lcd_control: u16,
    lcd_status: u16,
    mosaic_size: u32,
    color_effects_selection: u16,
    alpha_coefficients: u16,
    brightness_coefficient: u16,
    window_0_horizontal: u16,
    window_1_horizontal: u16,
    window_0_vertical: u16,
    window_1_vertical: u16,
    window_in_control: u16,
    window_out_control: u16,
    state: LcdState,
    bg_palette_ram: Box<[Rgb555; 0x100]>,
    obj_palette_ram: Box<[Rgb555; 0x100]>,
    vram: Box<[u8; 0x18000]>,
    obj_attributes: Box<[ObjectAttributeInfo; 0x80]>,
    obj_rotations: Box<[ObjectRotationScalingInfo; 0x20]>,
    buffer: Box<[[Rgb555; Self::LCD_WIDTH]; Self::LCD_HEIGHT]>, // access as buffer[y][x]
    back_buffer: Box<[[Rgb555; Self::LCD_WIDTH]; Self::LCD_HEIGHT]>,
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
            color_effects_selection: 0,
            alpha_coefficients: 0,
            brightness_coefficient: 0,
            window_0_horizontal: 0,
            window_1_horizontal: 0,
            window_0_vertical: 0,
            window_1_vertical: 0,
            window_in_control: 0,
            window_out_control: 0,
            state: LcdState::Visible,
            bg_palette_ram: Box::new([Rgb555::default(); 0x100]),
            obj_palette_ram: Box::new([Rgb555::default(); 0x100]),
            vram: Box::new([0; 0x18000]),
            obj_attributes: Box::new([ObjectAttributeInfo::default(); 0x80]),
            obj_rotations: Box::new([ObjectRotationScalingInfo::default(); 0x20]),
            buffer: Box::new([[Rgb555::default(); Self::LCD_WIDTH]; Self::LCD_HEIGHT]),
            back_buffer: Box::new([[Rgb555::default(); Self::LCD_WIDTH]; Self::LCD_HEIGHT]),
            layer_0: Layer0::default(),
            layer_1: Layer1::default(),
            layer_2: Layer2::default(),
            layer_3: Layer3::default(),
        }
    }
}

impl Lcd {
    pub const LCD_WIDTH: usize = 240;
    pub const LCD_HEIGHT: usize = 160;

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

            let obj_mosaic_horizontal = self.get_obj_mosaic_horizontal();
            let obj_mosaic_vertical = self.get_obj_mosaic_vertical();
            let sprite_pixel_query_info =
                self.get_sprite_pixel(pixel_x, pixel_y, obj_mosaic_horizontal, obj_mosaic_vertical);
            let displayed_selection =
                self.get_displayed_selection(pixel_x, pixel_y, sprite_pixel_query_info.obj_window);

            let bg_mosaic_horizontal = self.get_bg_mosaic_horizontal();
            let bg_mosaic_vertical = self.get_bg_mosaic_vertical();

            let layer_0_pixel_info = if displayed_selection.bg0_displayed {
                self.layer_0
                    .get_pixel(
                        (pixel_x, pixel_y),
                        (bg_mosaic_horizontal, bg_mosaic_vertical),
                        current_mode,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|color| PixelInfo {
                        color,
                        priority: self.layer_0.get_priority(),
                        pixel_type: PixelType::Layer0,
                    })
            } else {
                None
            };

            let layer_1_pixel_info = if displayed_selection.bg1_displayed {
                self.layer_1
                    .get_pixel(
                        (pixel_x, pixel_y),
                        (bg_mosaic_horizontal, bg_mosaic_vertical),
                        current_mode,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|color| PixelInfo {
                        color,
                        priority: self.layer_1.get_priority(),
                        pixel_type: PixelType::Layer1,
                    })
            } else {
                None
            };

            let layer_2_pixel_info = if displayed_selection.bg2_displayed {
                self.layer_2
                    .get_pixel(
                        (pixel_x, pixel_y),
                        (bg_mosaic_horizontal, bg_mosaic_vertical),
                        current_mode,
                        display_frame,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|color| PixelInfo {
                        color,
                        priority: self.layer_2.get_priority(),
                        pixel_type: PixelType::Layer2,
                    })
            } else {
                None
            };

            let layer_3_pixel_info = if displayed_selection.bg3_displayed {
                self.layer_3
                    .get_pixel(
                        (pixel_x, pixel_y),
                        (bg_mosaic_horizontal, bg_mosaic_vertical),
                        current_mode,
                        self.vram.as_slice(),
                        self.bg_palette_ram.as_slice(),
                    )
                    .map(|color| PixelInfo {
                        color,
                        priority: self.layer_3.get_priority(),
                        pixel_type: PixelType::Layer3,
                    })
            } else {
                None
            };

            let sprite_pixel_info = if displayed_selection.obj_displayed {
                sprite_pixel_query_info
                    .sprite_pixel_info
                    .map(|sprite_pixel_info| sprite_pixel_info.pixel_info)
            } else {
                None
            };

            let mut pixels = [
                sprite_pixel_info,
                layer_0_pixel_info,
                layer_1_pixel_info,
                layer_2_pixel_info,
                layer_3_pixel_info,
            ];
            pixels.sort_by(|pixel_one, pixel_two| match (pixel_one, pixel_two) {
                (
                    Some(PixelInfo {
                        priority: priority_one,
                        ..
                    }),
                    Some(PixelInfo {
                        priority: priority_two,
                        ..
                    }),
                ) => Ord::cmp(&priority_one, &priority_two),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            });
            let pixels = pixels;

            let drawn_pixel = match (
                displayed_selection.effects_displayed,
                self.get_color_special_effect(),
            ) {
                (true, ColorSpecialEffect::AlphaBlending) => {
                    let first_pixel = pixels[0];
                    let second_pixel = pixels[1];

                    // sanity check to ensure array was properly sorted.
                    assert!(first_pixel.is_some() || second_pixel.is_none());

                    let backdrop_info = (self.bg_palette_ram[0], PixelType::Backdrop);

                    let first_pixel_info = if let Some(PixelInfo {
                        color, pixel_type, ..
                    }) = first_pixel
                    {
                        (color, pixel_type)
                    } else {
                        backdrop_info
                    };

                    let second_pixel_info = if let Some(PixelInfo {
                        color, pixel_type, ..
                    }) = second_pixel
                    {
                        (color, pixel_type)
                    } else {
                        backdrop_info
                    };

                    if self.special_effect_first_pixel(first_pixel_info.1)
                        && self.special_effect_second_pixel(second_pixel_info.1)
                    {
                        first_pixel_info.0.blend(
                            self.get_alpha_first_target_coefficient(),
                            second_pixel_info.0,
                            self.get_alpha_second_target_coefficient(),
                        )
                    } else {
                        first_pixel_info.0
                    }
                }
                (true, ColorSpecialEffect::BrightnessIncrease) => {
                    let pixel = pixels[0];

                    let backdrop_info = (self.bg_palette_ram[0], PixelType::Backdrop);

                    let (pixel_color, pixel_type) =
                        if let Some(PixelInfo {
                            color, pixel_type, ..
                        }) = pixel
                        {
                            (color, pixel_type)
                        } else {
                            backdrop_info
                        };

                    if self.special_effect_first_pixel(pixel_type) {
                        let new_red = pixel_color.red()
                            + ((f64::from(31 - pixel_color.red())
                                * self.get_brightness_coefficient())
                                as u8);
                        let new_green = pixel_color.green()
                            + ((f64::from(31 - pixel_color.green())
                                * self.get_brightness_coefficient())
                                as u8);
                        let new_blue = pixel_color.blue()
                            + ((f64::from(31 - pixel_color.blue())
                                * self.get_brightness_coefficient())
                                as u8);

                        Rgb555::new(new_red, new_green, new_blue)
                    } else {
                        pixel_color
                    }
                }
                (true, ColorSpecialEffect::BrightnessDecrease) => {
                    let pixel = pixels[0];

                    let backdrop_info = (self.bg_palette_ram[0], PixelType::Backdrop);

                    let (pixel_color, pixel_type) =
                        if let Some(PixelInfo {
                            color, pixel_type, ..
                        }) = pixel
                        {
                            (color, pixel_type)
                        } else {
                            backdrop_info
                        };

                    if self.special_effect_first_pixel(pixel_type) {
                        let new_red = pixel_color.red()
                            - ((f64::from(pixel_color.red()) * self.get_brightness_coefficient())
                                as u8);
                        let new_green = pixel_color.green()
                            - ((f64::from(pixel_color.green()) * self.get_brightness_coefficient())
                                as u8);
                        let new_blue = pixel_color.blue()
                            - ((f64::from(pixel_color.blue()) * self.get_brightness_coefficient())
                                as u8);

                        Rgb555::new(new_red, new_green, new_blue)
                    } else {
                        pixel_color
                    }
                }
                (true, ColorSpecialEffect::None) | (false, _) => match pixels[0] {
                    Some(PixelInfo { color, .. }) => color,
                    None => self.bg_palette_ram[0],
                },
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
    ) -> SpritePixelQueryInfo {
        const OBJ_TILE_DATA_VRAM_BASE: usize = 0x10000;
        const TILE_SIZE: u16 = 8;
        const WORLD_WIDTH: u16 = 512;
        const WORLD_HEIGHT: u16 = 256;

        let mut sprite_pixel_info: Option<SpritePixelInfo> = None;

        let mut obj_window = false;

        for obj in self.obj_attributes.iter() {
            let (sprite_tile_width, sprite_tile_height) = match obj.get_obj_tile_dims() {
                Some(dims) => dims,
                None => continue,
            };

            let sprite_width = sprite_tile_width * TILE_SIZE;
            let sprite_height = sprite_tile_height * TILE_SIZE;

            let sprite_x = obj.get_x_coordinate();
            let sprite_y = obj.get_y_coordinate();

            let mut base_corner_offset_x = (pixel_x + WORLD_WIDTH - sprite_x) % WORLD_WIDTH;
            let mut base_corner_offset_y = (pixel_y + WORLD_HEIGHT - sprite_y) % WORLD_HEIGHT;

            if base_corner_offset_x >= (sprite_width * 2)
                || base_corner_offset_y >= (sprite_height * 2)
                || ((!obj.get_rotation_scaling_flag() || !obj.get_double_size_flag())
                    && (base_corner_offset_x >= sprite_width
                        || base_corner_offset_y >= sprite_height))
            {
                continue;
            }

            let (sprite_offset_x, sprite_offset_y) = if obj.get_rotation_scaling_flag() {
                let base_corner_offset_x = base_corner_offset_x;
                let base_corner_offset_y = base_corner_offset_y;

                let rotation_info_idx = obj.get_rotation_scaling_index();
                let rotation_info = self.obj_rotations[usize::from(rotation_info_idx)];

                let a = half_word_fixed_point_to_float(rotation_info.a);
                let b = half_word_fixed_point_to_float(rotation_info.b);
                let c = half_word_fixed_point_to_float(rotation_info.c);
                let d = half_word_fixed_point_to_float(rotation_info.d);

                let center_offset_adjustment_x = sprite_width / 2;
                let center_offset_adjustment_y = sprite_height / 2;

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
                        f64::from(base_corner_offset_x) - f64::from(sprite_width),
                        f64::from(base_corner_offset_y) - f64::from(sprite_height),
                    )
                } else {
                    (
                        f64::from(base_corner_offset_x) - (f64::from(sprite_width) / 2.0),
                        f64::from(base_corner_offset_y) - (f64::from(sprite_height) / 2.0),
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
                if obj.get_obj_disable_flag() {
                    continue;
                }

                if obj.get_obj_mosaic() {
                    base_corner_offset_x -= base_corner_offset_x % obj_mosaic_horizontal;
                    base_corner_offset_y -= base_corner_offset_y % obj_mosaic_vertical;
                }

                let base_corner_offset_x = base_corner_offset_x;
                let base_corner_offset_y = base_corner_offset_y;

                let offset_x = if obj.get_horizontal_flip() {
                    sprite_width - 1 - base_corner_offset_x
                } else {
                    base_corner_offset_x
                };

                let offset_y = if obj.get_vertical_flip() {
                    sprite_height - 1 - base_corner_offset_y
                } else {
                    base_corner_offset_y
                };

                (offset_x, offset_y)
            };

            assert!(sprite_offset_x < sprite_width);
            assert!(sprite_offset_y < sprite_height);

            let sprite_tile_x = sprite_offset_x / 8;
            let sprite_tile_y = sprite_offset_y / 8;

            let tile_offset_x = sprite_offset_x % 8;
            let tile_offset_y = sprite_offset_y % 8;

            let base_tile_number = obj.get_tile_number();

            let palette_idx = match obj.get_palette_depth() {
                PaletteDepth::EightBit => {
                    let tile_number = match self.get_obj_tile_mapping() {
                        ObjectTileMapping::OneDimensional => {
                            base_tile_number
                                + (sprite_tile_y * sprite_tile_width * 2)
                                + (sprite_tile_x * 2)
                        }
                        ObjectTileMapping::TwoDimensional => {
                            base_tile_number + (sprite_tile_y * 32) + (sprite_tile_x * 2)
                        }
                    };

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
                    let tile_number = match self.get_obj_tile_mapping() {
                        ObjectTileMapping::OneDimensional => {
                            base_tile_number + (sprite_tile_y * sprite_tile_width) + sprite_tile_x
                        }
                        ObjectTileMapping::TwoDimensional => {
                            base_tile_number + (sprite_tile_y * 32) + sprite_tile_x
                        }
                    };

                    let tile_idx = OBJ_TILE_DATA_VRAM_BASE
                        + (usize::from(tile_number) * 32)
                        + (usize::from(tile_offset_y) * 4)
                        + (usize::from(tile_offset_x) / 2);

                    let tile_data = self.vram[tile_idx];

                    let palette_idx_low = if tile_offset_x % 2 == 0 {
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

            let semi_transparent = match obj.get_obj_mode() {
                ObjMode::Normal => false,
                ObjMode::SemiTransparent => true,
                ObjMode::ObjWindow => {
                    obj_window = true;
                    continue;
                }
            };

            let priority = obj.get_bg_priority();

            // If we've already found a pixel and our new pixel has lower priority (keeping)
            // in mind that values closer to zero are considered higher priority, then don't
            // bother recording this pixel.
            if let Some(info) = sprite_pixel_info {
                if info.pixel_info.priority <= priority {
                    continue;
                };
            }

            let new_pixel_info = PixelInfo {
                color: self.obj_palette_ram[usize::from(palette_idx)],
                priority,
                pixel_type: PixelType::Sprite,
            };

            let new_sprite_pixel_info = SpritePixelInfo {
                pixel_info: new_pixel_info,
                semi_transparent,
            };

            sprite_pixel_info = Some(new_sprite_pixel_info);
        }

        SpritePixelQueryInfo {
            sprite_pixel_info,
            obj_window,
        }
    }

    fn get_displayed_selection(
        &self,
        pixel_x: u16,
        pixel_y: u16,
        in_obj_window: bool,
    ) -> DisplayedSelectionInfo {
        let mut bg0_displayed = self.get_screen_display_bg_0();
        let mut bg1_displayed = self.get_screen_display_bg_1();
        let mut bg2_displayed = self.get_screen_display_bg_2();
        let mut bg3_displayed = self.get_screen_display_bg_3();
        let mut obj_displayed = self.get_screen_display_obj();

        let mut effects_displayed = true;

        if self.get_display_window_0()
            || self.get_display_window_1()
            || self.get_display_obj_window()
        {
            let in_window_0 = self.get_display_window_0()
                && pixel_x >= self.get_window_0_left()
                && pixel_x < self.get_window_0_right()
                && pixel_y >= self.get_window_0_top()
                && pixel_y < self.get_window_0_bottom();
            let in_window_1 = self.get_display_window_1()
                && pixel_x >= self.get_window_1_left()
                && pixel_x < self.get_window_1_right()
                && pixel_y >= self.get_window_1_top()
                && pixel_y < self.get_window_1_bottom();

            if in_window_0 {
                bg0_displayed &= self.get_window_0_bg_0_enable();
                bg1_displayed &= self.get_window_0_bg_1_enable();
                bg2_displayed &= self.get_window_0_bg_2_enable();
                bg3_displayed &= self.get_window_0_bg_3_enable();
                obj_displayed &= self.get_window_0_obj_enable();
                effects_displayed &= self.get_window_0_special_effects_enable();
            } else if in_window_1 {
                bg0_displayed &= self.get_window_1_bg_0_enable();
                bg1_displayed &= self.get_window_1_bg_1_enable();
                bg2_displayed &= self.get_window_1_bg_2_enable();
                bg3_displayed &= self.get_window_1_bg_3_enable();
                obj_displayed &= self.get_window_1_obj_enable();
                effects_displayed &= self.get_window_1_special_effects_enable();
            } else if in_obj_window {
                bg0_displayed &= self.get_obj_window_bg_0_enable();
                bg1_displayed &= self.get_obj_window_bg_1_enable();
                bg2_displayed &= self.get_obj_window_bg_2_enable();
                bg3_displayed &= self.get_obj_window_bg_3_enable();
                obj_displayed &= self.get_obj_window_obj_enable();
                effects_displayed &= self.get_obj_window_special_effects_enable();
            } else {
                bg0_displayed &= self.get_outside_window_bg_0_enable();
                bg1_displayed &= self.get_outside_window_bg_1_enable();
                bg2_displayed &= self.get_outside_window_bg_2_enable();
                bg3_displayed &= self.get_outside_window_bg_3_enable();
                obj_displayed &= self.get_outside_window_obj_enable();
                effects_displayed &= self.get_outside_window_special_effects_enable();
            }
        }

        DisplayedSelectionInfo {
            bg0_displayed,
            bg1_displayed,
            bg2_displayed,
            bg3_displayed,
            obj_displayed,
            effects_displayed,
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

    pub fn read_color_effects_selection<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.color_effects_selection.get_data(index)
    }

    pub fn write_color_effects_selection<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const COLOR_EFFECTS_SELECTION_WRITE_MASK: u16 = 0b0011_1111_1111_1111;
        self.color_effects_selection = self.color_effects_selection.set_data(value, index);
        self.color_effects_selection &= COLOR_EFFECTS_SELECTION_WRITE_MASK;
    }

    pub fn read_alpha_blending_coefficients<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.alpha_coefficients.get_data(index)
    }

    pub fn write_alpha_blending_coefficients<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const ALPHA_BLENDING_COEFFICIENT_WRITE_MASK: u16 = 0b0001_1111_0001_1111;
        self.alpha_coefficients = self.alpha_coefficients.set_data(value, index);
        self.alpha_coefficients &= ALPHA_BLENDING_COEFFICIENT_WRITE_MASK;
    }

    pub fn read_brightness_coefficient<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.brightness_coefficient.get_data(index)
    }

    pub fn write_brightness_coefficient<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.brightness_coefficient = self.brightness_coefficient.set_data(value, index)
    }

    pub fn read_window_0_horizontal<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.window_0_horizontal.get_data(index)
    }

    pub fn write_window_0_horizontal<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.window_0_horizontal = self.window_0_horizontal.set_data(value, index)
    }

    pub fn read_window_1_horizontal<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.window_1_horizontal.get_data(index)
    }

    pub fn write_window_1_horizontal<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.window_1_horizontal = self.window_1_horizontal.set_data(value, index)
    }

    pub fn read_window_0_vertical<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.window_0_vertical.get_data(index)
    }

    pub fn write_window_0_vertical<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.window_0_vertical = self.window_0_vertical.set_data(value, index)
    }

    pub fn read_window_1_vertical<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.window_1_vertical.get_data(index)
    }

    pub fn write_window_1_vertical<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.window_1_vertical = self.window_1_vertical.set_data(value, index)
    }

    pub fn read_window_in_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.window_in_control.get_data(index)
    }

    pub fn write_window_in_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const WIN_IN_WRITE_MASK: u16 = 0b0011_1111_0011_1111;
        self.window_in_control = self.window_in_control.set_data(value, index);
        self.window_in_control &= WIN_IN_WRITE_MASK;
    }

    pub fn read_window_out_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.window_out_control.get_data(index)
    }

    pub fn write_window_out_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        const WIN_OUT_WRITE_MASK: u16 = 0b0011_1111_0011_1111;
        self.window_out_control = self.window_out_control.set_data(value, index);
        self.window_out_control &= WIN_OUT_WRITE_MASK;
    }

    const BG_PALETTE_RAM_OFFSET_START: u32 = 0x000;
    const BG_PALETTE_RAM_OFFSET_END: u32 = 0x1FF;
    const OBJ_PALETTE_RAM_OFFSET_START: u32 = 0x200;
    const OBJ_PALETTE_RAM_OFFSET_END: u32 = 0x3FF;

    pub fn read_palette_ram_byte(&self, offset: u32) -> u8 {
        let hword_index = offset & 0b1;

        let le_bytes = self.read_palette_ram_hword(offset & (!0b1)).to_le_bytes();

        le_bytes[hword_index as usize]
    }

    pub fn read_palette_ram_hword(&self, offset: u32) -> u16 {
        assert!(offset & 0b1 == 0);

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

        color.to_int()
    }

    pub fn read_palette_ram_word(&self, offset: u32) -> u32 {
        assert!(offset & 0b11 == 0);

        let low_hword = self.read_palette_ram_hword(offset);
        let high_hword = self.read_palette_ram_hword(offset + 2);

        u32::from(low_hword) | (u32::from(high_hword) << 0x10)
    }

    pub fn write_palette_ram_byte(&mut self, value: u8, offset: u32) {
        // write byte value to both high and low byte of addressed halfword.

        let actual_offset = offset & (!0b1);

        let actual_value = u16::from_le_bytes([value, value]);

        self.write_palette_ram_hword(actual_value, actual_offset);
    }

    pub fn write_palette_ram_hword(&mut self, value: u16, offset: u32) {
        assert!(offset & 0b1 == 0);

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

        *color = Rgb555::from_int(value);
    }

    pub fn write_palette_ram_word(&mut self, value: u32, offset: u32) {
        assert!(offset & 0b11 == 0);

        let high_hword = (value >> 16) as u16;
        let low_hword = value as u16;

        self.write_palette_ram_hword(low_hword, offset);
        self.write_palette_ram_hword(high_hword, offset + 2);
    }

    pub fn read_vram_byte(&self, offset: u32) -> u8 {
        self.vram[offset as usize]
    }

    pub fn read_vram_hword(&self, offset: u32) -> u16 {
        assert!(offset & 0b1 == 0);

        let low_byte = self.vram[offset as usize];
        let high_byte = self.vram[(offset + 1) as usize];

        u16::from_le_bytes([low_byte, high_byte])
    }

    pub fn read_vram_word(&self, offset: u32) -> u32 {
        assert!(offset & 0b11 == 0);

        let le_bytes = [
            self.vram[offset as usize],
            self.vram[(offset + 1) as usize],
            self.vram[(offset + 2) as usize],
            self.vram[(offset + 3) as usize],
        ];

        u32::from_le_bytes(le_bytes)
    }

    pub fn write_vram_byte(&mut self, value: u8, offset: u32) {
        const TILE_MODE_BG_RANGE_START: u32 = 0x00000;
        const TILE_MODE_BG_RANGE_END: u32 = 0x0FFFF;
        const TILE_MODE_OBJ_RANGE_START: u32 = 0x10000;
        const TILE_MODE_OBJ_RANGE_END: u32 = 0x17FFF;

        const BITMAP_MODE_BG_RANGE_START: u32 = 0x00000;
        const BITMAP_MODE_BG_RANGE_END: u32 = 0x13FFF;
        const BITMAP_MODE_OBJ_RANGE_START: u32 = 0x14000;
        const BITMAP_MODE_OBJ_RANGE_END: u32 = 0x17FFF;

        #[derive(Clone, Copy, Debug)]
        enum WriteBehavior {
            IgnoreWrite,
            WriteUpperLowerByte,
        }

        let write_behavior = match self.get_bg_mode().get_type() {
            BgModeType::TileMode | BgModeType::Invalid => match offset {
                TILE_MODE_BG_RANGE_START..=TILE_MODE_BG_RANGE_END => {
                    WriteBehavior::WriteUpperLowerByte
                }
                TILE_MODE_OBJ_RANGE_START..=TILE_MODE_OBJ_RANGE_END => WriteBehavior::IgnoreWrite,
                _ => unreachable!(),
            },
            BgModeType::BitmapMode => match offset {
                BITMAP_MODE_BG_RANGE_START..=BITMAP_MODE_BG_RANGE_END => {
                    WriteBehavior::WriteUpperLowerByte
                }
                BITMAP_MODE_OBJ_RANGE_START..=BITMAP_MODE_OBJ_RANGE_END => {
                    WriteBehavior::IgnoreWrite
                }
                _ => unreachable!(),
            },
        };

        match write_behavior {
            WriteBehavior::IgnoreWrite => {}
            WriteBehavior::WriteUpperLowerByte => {
                let actual_offset = offset & (!0b1);

                let actual_value = u16::from_le_bytes([value, value]);

                self.write_vram_hword(actual_value, actual_offset);
            }
        }
    }

    pub fn write_vram_hword(&mut self, value: u16, offset: u32) {
        assert!(offset & 0b1 == 0);

        let [low_byte, high_byte] = value.to_le_bytes();

        self.vram[offset as usize] = low_byte;
        self.vram[(offset + 1) as usize] = high_byte;
    }

    pub fn write_vram_word(&mut self, value: u32, offset: u32) {
        assert!(offset & 0b11 == 0);

        for (byte_offset, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.vram[(offset as usize) + byte_offset] = byte;
        }
    }

    pub fn read_oam_byte(&self, offset: u32) -> u8 {
        let hword_index = offset % 2;

        let le_bytes = self.read_oam_hword(offset & (!0b1)).to_le_bytes();

        le_bytes[hword_index as usize]
    }

    pub fn read_oam_hword(&self, offset: u32) -> u16 {
        assert!(offset & 0b1 == 0);

        let hword_offset = offset / 2;

        let oam_index = (hword_offset / 4) as usize;
        let oam_offset = hword_offset % 4;

        let rotation_group_index = (hword_offset / 16) as usize;
        let rotation_group_offset = (hword_offset / 4) % 4;

        match oam_offset {
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
        }
    }

    pub fn read_oam_word(&self, offset: u32) -> u32 {
        assert!(offset & 0b11 == 0);

        let low_hword = self.read_oam_hword(offset);
        let high_hword = self.read_oam_hword(offset + 2);

        u32::from(low_hword) | (u32::from(high_hword) << 0x10)
    }

    pub fn write_oam_byte(&mut self, _value: u8, _offset: u32) {
        // byte write to OAM is ignored
    }

    pub fn write_oam_hword(&mut self, value: u16, offset: u32) {
        assert!(offset & 0b1 == 0);

        let hword_offset = offset / 2;

        let oam_index = (hword_offset / 4) as usize;
        let oam_offset = hword_offset % 4;

        let rotation_group_index = (hword_offset / 16) as usize;
        let rotation_group_offset = (hword_offset / 4) % 4;

        match oam_offset {
            0 => {
                self.obj_attributes[oam_index].write_attribute_0(value, 0);
            }
            1 => {
                self.obj_attributes[oam_index].write_attribute_1(value, 0);
            }
            2 => {
                self.obj_attributes[oam_index].write_attribute_2(value, 0);
            }
            3 => match rotation_group_offset {
                0 => {
                    self.obj_rotations[rotation_group_index].a = self.obj_rotations
                        [rotation_group_index]
                        .a
                        .set_data(value, 0)
                }
                1 => {
                    self.obj_rotations[rotation_group_index].b = self.obj_rotations
                        [rotation_group_index]
                        .b
                        .set_data(value, 0)
                }
                2 => {
                    self.obj_rotations[rotation_group_index].c = self.obj_rotations
                        [rotation_group_index]
                        .c
                        .set_data(value, 0)
                }
                3 => {
                    self.obj_rotations[rotation_group_index].d = self.obj_rotations
                        [rotation_group_index]
                        .d
                        .set_data(value, 0)
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
    }

    pub fn write_oam_word(&mut self, value: u32, offset: u32) {
        assert!(offset & 0b11 == 0);

        let high_hword = (value >> 16) as u16;
        let low_hword = value as u16;

        self.write_oam_hword(low_hword, offset);
        self.write_oam_hword(high_hword, offset + 2);
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
            6 | 7 => BgMode::Invalid,
            _ => unreachable!(),
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

    fn get_screen_display_obj(&self) -> bool {
        const DISPLAY_OBJ_BIT_INDEX: usize = 12;

        self.lcd_control.get_bit(DISPLAY_OBJ_BIT_INDEX)
    }

    fn get_display_window_0(&self) -> bool {
        const DISPLAY_WINDOW_0_BIT_INDEX: usize = 13;

        self.lcd_control.get_bit(DISPLAY_WINDOW_0_BIT_INDEX)
    }

    fn get_display_window_1(&self) -> bool {
        const DISPLAY_WINDOW_1_BIT_INDEX: usize = 14;

        self.lcd_control.get_bit(DISPLAY_WINDOW_1_BIT_INDEX)
    }

    fn get_display_obj_window(&self) -> bool {
        const DISPLAY_OBJ_WINDOW_BIT_INDEX: usize = 15;

        self.lcd_control.get_bit(DISPLAY_OBJ_WINDOW_BIT_INDEX)
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

    fn special_effect_first_pixel(&self, pixel_type: PixelType) -> bool {
        const BG0_FIRST_PIXEL_BIT_INDEX: usize = 0;
        const BG1_FIRST_PIXEL_BIT_INDEX: usize = 1;
        const BG2_FIRST_PIXEL_BIT_INDEX: usize = 2;
        const BG3_FIRST_PIXEL_BIT_INDEX: usize = 3;
        const OBJ_FIRST_PIXEL_BIT_INDEX: usize = 4;
        const BD_FIRST_PIXEL_BIT_INDEX: usize = 5;

        match pixel_type {
            PixelType::Layer0 => self
                .color_effects_selection
                .get_bit(BG0_FIRST_PIXEL_BIT_INDEX),
            PixelType::Layer1 => self
                .color_effects_selection
                .get_bit(BG1_FIRST_PIXEL_BIT_INDEX),
            PixelType::Layer2 => self
                .color_effects_selection
                .get_bit(BG2_FIRST_PIXEL_BIT_INDEX),
            PixelType::Layer3 => self
                .color_effects_selection
                .get_bit(BG3_FIRST_PIXEL_BIT_INDEX),
            PixelType::Sprite => self
                .color_effects_selection
                .get_bit(OBJ_FIRST_PIXEL_BIT_INDEX),
            PixelType::Backdrop => self
                .color_effects_selection
                .get_bit(BD_FIRST_PIXEL_BIT_INDEX),
        }
    }

    fn get_color_special_effect(&self) -> ColorSpecialEffect {
        const SPECIAL_EFFECT_BIT_RANGE: RangeInclusive<usize> = 6..=7;

        match self
            .color_effects_selection
            .get_bit_range(SPECIAL_EFFECT_BIT_RANGE)
        {
            0 => ColorSpecialEffect::None,
            1 => ColorSpecialEffect::AlphaBlending,
            2 => ColorSpecialEffect::BrightnessIncrease,
            3 => ColorSpecialEffect::BrightnessDecrease,
            _ => unreachable!(),
        }
    }

    fn special_effect_second_pixel(&self, pixel_type: PixelType) -> bool {
        const BG0_SECOND_PIXEL_BIT_INDEX: usize = 8;
        const BG1_SECOND_PIXEL_BIT_INDEX: usize = 9;
        const BG2_SECOND_PIXEL_BIT_INDEX: usize = 10;
        const BG3_SECOND_PIXEL_BIT_INDEX: usize = 11;
        const OBJ_SECOND_PIXEL_BIT_INDEX: usize = 12;
        const BD_SECOND_PIXEL_BIT_INDEX: usize = 13;

        match pixel_type {
            PixelType::Layer0 => self
                .color_effects_selection
                .get_bit(BG0_SECOND_PIXEL_BIT_INDEX),
            PixelType::Layer1 => self
                .color_effects_selection
                .get_bit(BG1_SECOND_PIXEL_BIT_INDEX),
            PixelType::Layer2 => self
                .color_effects_selection
                .get_bit(BG2_SECOND_PIXEL_BIT_INDEX),
            PixelType::Layer3 => self
                .color_effects_selection
                .get_bit(BG3_SECOND_PIXEL_BIT_INDEX),
            PixelType::Sprite => self
                .color_effects_selection
                .get_bit(OBJ_SECOND_PIXEL_BIT_INDEX),
            PixelType::Backdrop => self
                .color_effects_selection
                .get_bit(BD_SECOND_PIXEL_BIT_INDEX),
        }
    }

    fn get_alpha_first_target_coefficient(&self) -> f64 {
        const FIRST_TARGET_COEFFICIENT_BIT_RANGE: RangeInclusive<usize> = 0..=4;

        match self
            .alpha_coefficients
            .get_bit_range(FIRST_TARGET_COEFFICIENT_BIT_RANGE)
        {
            base @ 0..=16 => f64::from(base) / 16.0,
            17..=31 => 1.0,
            _ => unreachable!(),
        }
    }

    fn get_alpha_second_target_coefficient(&self) -> f64 {
        const SECOND_TARGET_COEFFICIENT_BIT_RANGE: RangeInclusive<usize> = 8..=12;

        match self
            .alpha_coefficients
            .get_bit_range(SECOND_TARGET_COEFFICIENT_BIT_RANGE)
        {
            base @ 0..=16 => f64::from(base) / 16.0,
            17..=31 => 1.0,
            _ => unreachable!(),
        }
    }

    fn get_brightness_coefficient(&self) -> f64 {
        const BRIGHTNESS_COEFFICIENT_BIT_RANGE: RangeInclusive<usize> = 0..=4;

        match self
            .brightness_coefficient
            .get_bit_range(BRIGHTNESS_COEFFICIENT_BIT_RANGE)
        {
            base @ 0..=16 => f64::from(base) / 16.0,
            17..=31 => 1.0,
            _ => unreachable!(),
        }
    }

    const WINDOW_RIGHT_BIT_RANGE: RangeInclusive<usize> = 0..=7;
    const WINDOW_LEFT_BIT_RANGE: RangeInclusive<usize> = 8..=15;
    const WINDOW_BOTTOM_BIT_RANGE: RangeInclusive<usize> = 0..=7;
    const WINDOW_TOP_BIT_RANGE: RangeInclusive<usize> = 8..=15;

    fn get_window_0_right(&self) -> u16 {
        self.window_0_horizontal
            .get_bit_range(Self::WINDOW_RIGHT_BIT_RANGE)
    }

    fn get_window_0_left(&self) -> u16 {
        self.window_0_horizontal
            .get_bit_range(Self::WINDOW_LEFT_BIT_RANGE)
    }

    fn get_window_0_bottom(&self) -> u16 {
        self.window_0_vertical
            .get_bit_range(Self::WINDOW_BOTTOM_BIT_RANGE)
    }

    fn get_window_0_top(&self) -> u16 {
        self.window_0_vertical
            .get_bit_range(Self::WINDOW_TOP_BIT_RANGE)
    }

    fn get_window_1_right(&self) -> u16 {
        self.window_1_horizontal
            .get_bit_range(Self::WINDOW_RIGHT_BIT_RANGE)
    }

    fn get_window_1_left(&self) -> u16 {
        self.window_1_horizontal
            .get_bit_range(Self::WINDOW_LEFT_BIT_RANGE)
    }

    fn get_window_1_bottom(&self) -> u16 {
        self.window_1_vertical
            .get_bit_range(Self::WINDOW_BOTTOM_BIT_RANGE)
    }

    fn get_window_1_top(&self) -> u16 {
        self.window_1_vertical
            .get_bit_range(Self::WINDOW_TOP_BIT_RANGE)
    }

    fn get_window_0_bg_0_enable(&self) -> bool {
        const WINDOW_0_BG_0_ENABLE_BIT_INDEX: usize = 0;

        self.window_in_control
            .get_bit(WINDOW_0_BG_0_ENABLE_BIT_INDEX)
    }

    fn get_window_0_bg_1_enable(&self) -> bool {
        const WINDOW_0_BG_1_ENABLE_BIT_INDEX: usize = 1;

        self.window_in_control
            .get_bit(WINDOW_0_BG_1_ENABLE_BIT_INDEX)
    }

    fn get_window_0_bg_2_enable(&self) -> bool {
        const WINDOW_0_BG_2_ENABLE_BIT_INDEX: usize = 2;

        self.window_in_control
            .get_bit(WINDOW_0_BG_2_ENABLE_BIT_INDEX)
    }

    fn get_window_0_bg_3_enable(&self) -> bool {
        const WINDOW_0_BG_3_ENABLE_BIT_INDEX: usize = 3;

        self.window_in_control
            .get_bit(WINDOW_0_BG_3_ENABLE_BIT_INDEX)
    }

    fn get_window_0_obj_enable(&self) -> bool {
        const WINDOW_0_OBJ_ENABLE_BIT_INDEX: usize = 4;

        self.window_in_control
            .get_bit(WINDOW_0_OBJ_ENABLE_BIT_INDEX)
    }

    fn get_window_0_special_effects_enable(&self) -> bool {
        const WINDOW_0_SPECIAL_EFFECTS_BIT_INDEX: usize = 5;

        self.window_in_control
            .get_bit(WINDOW_0_SPECIAL_EFFECTS_BIT_INDEX)
    }

    fn get_window_1_bg_0_enable(&self) -> bool {
        const WINDOW_1_BG_0_ENABLE_BIT_INDEX: usize = 8;

        self.window_in_control
            .get_bit(WINDOW_1_BG_0_ENABLE_BIT_INDEX)
    }

    fn get_window_1_bg_1_enable(&self) -> bool {
        const WINDOW_1_BG_1_ENABLE_BIT_INDEX: usize = 9;

        self.window_in_control
            .get_bit(WINDOW_1_BG_1_ENABLE_BIT_INDEX)
    }

    fn get_window_1_bg_2_enable(&self) -> bool {
        const WINDOW_1_BG_2_ENABLE_BIT_INDEX: usize = 10;

        self.window_in_control
            .get_bit(WINDOW_1_BG_2_ENABLE_BIT_INDEX)
    }

    fn get_window_1_bg_3_enable(&self) -> bool {
        const WINDOW_1_BG_3_ENABLE_BIT_INDEX: usize = 11;

        self.window_in_control
            .get_bit(WINDOW_1_BG_3_ENABLE_BIT_INDEX)
    }

    fn get_window_1_obj_enable(&self) -> bool {
        const WINDOW_1_OBJ_ENABLE_BIT_INDEX: usize = 12;

        self.window_in_control
            .get_bit(WINDOW_1_OBJ_ENABLE_BIT_INDEX)
    }

    fn get_window_1_special_effects_enable(&self) -> bool {
        const WINDOW_1_SPECIAL_EFFECTS_BIT_INDEX: usize = 13;

        self.window_in_control
            .get_bit(WINDOW_1_SPECIAL_EFFECTS_BIT_INDEX)
    }

    fn get_outside_window_bg_0_enable(&self) -> bool {
        const WINDOW_OUTSIDE_BG_0_ENABLE_BIT_INDEX: usize = 0;

        self.window_out_control
            .get_bit(WINDOW_OUTSIDE_BG_0_ENABLE_BIT_INDEX)
    }

    fn get_outside_window_bg_1_enable(&self) -> bool {
        const WINDOW_OUTSIDE_BG_1_ENABLE_BIT_INDEX: usize = 1;

        self.window_out_control
            .get_bit(WINDOW_OUTSIDE_BG_1_ENABLE_BIT_INDEX)
    }

    fn get_outside_window_bg_2_enable(&self) -> bool {
        const WINDOW_OUTSIDE_BG_2_ENABLE_BIT_INDEX: usize = 2;

        self.window_out_control
            .get_bit(WINDOW_OUTSIDE_BG_2_ENABLE_BIT_INDEX)
    }

    fn get_outside_window_bg_3_enable(&self) -> bool {
        const WINDOW_OUTSIDE_BG_3_ENABLE_BIT_INDEX: usize = 3;

        self.window_out_control
            .get_bit(WINDOW_OUTSIDE_BG_3_ENABLE_BIT_INDEX)
    }

    fn get_outside_window_obj_enable(&self) -> bool {
        const WINDOW_OUTSIDE_OBJ_ENABLE_BIT_INDEX: usize = 4;

        self.window_out_control
            .get_bit(WINDOW_OUTSIDE_OBJ_ENABLE_BIT_INDEX)
    }

    fn get_outside_window_special_effects_enable(&self) -> bool {
        const WINDOW_OUTSIDE_SPECIAL_EFFECTS_BIT_INDEX: usize = 5;

        self.window_out_control
            .get_bit(WINDOW_OUTSIDE_SPECIAL_EFFECTS_BIT_INDEX)
    }

    fn get_obj_window_bg_0_enable(&self) -> bool {
        const OBJ_WINDOW_BG_0_ENABLE_BIT_INDEX: usize = 8;

        self.window_out_control
            .get_bit(OBJ_WINDOW_BG_0_ENABLE_BIT_INDEX)
    }

    fn get_obj_window_bg_1_enable(&self) -> bool {
        const OBJ_WINDOW_BG_1_ENABLE_BIT_INDEX: usize = 9;

        self.window_out_control
            .get_bit(OBJ_WINDOW_BG_1_ENABLE_BIT_INDEX)
    }

    fn get_obj_window_bg_2_enable(&self) -> bool {
        const OBJ_WINDOW_BG_2_ENABLE_BIT_INDEX: usize = 10;

        self.window_out_control
            .get_bit(OBJ_WINDOW_BG_2_ENABLE_BIT_INDEX)
    }

    fn get_obj_window_bg_3_enable(&self) -> bool {
        const OBJ_WINDOW_BG_3_ENABLE_BIT_INDEX: usize = 11;

        self.window_out_control
            .get_bit(OBJ_WINDOW_BG_3_ENABLE_BIT_INDEX)
    }

    fn get_obj_window_obj_enable(&self) -> bool {
        const OBJ_WINDOW_OBJ_ENABLE_BIT_INDEX: usize = 12;

        self.window_out_control
            .get_bit(OBJ_WINDOW_OBJ_ENABLE_BIT_INDEX)
    }

    fn get_obj_window_special_effects_enable(&self) -> bool {
        const OBJ_WINDOW_SPECIAL_EFFECTS_BIT_INDEX: usize = 13;

        self.window_out_control
            .get_bit(OBJ_WINDOW_SPECIAL_EFFECTS_BIT_INDEX)
    }
}

impl Lcd {
    pub fn get_buffer(&self) -> &[[Rgb555; Self::LCD_WIDTH]; Self::LCD_HEIGHT] {
        &self.buffer
    }
}
