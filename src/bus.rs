use std::ops::RangeInclusive;

use crate::apu::Apu;
use crate::keypad::Keypad;
use crate::lcd::{Lcd, LcdInterruptInfo};
use crate::BitManipulation;
use crate::DataAccess;

const BIOS: &[u8] = include_bytes!("../gba_bios.bin");
const ROM: &[u8] = include_bytes!("../sbb_aff.gba");

#[derive(Debug)]
pub struct Bus {
    chip_wram: Box<[u8; 0x8000]>,
    pub board_wram: Box<[u8; 0x40000]>,
    cycle_count: usize,
    interrupt_master_enable: u16,
    interrupt_enable: u16,
    interrupt_request: u16,
    dma_infos: [DmaInfo; 4],
    pub lcd: Lcd,
    pub apu: Apu,
    pub keypad: Keypad,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            chip_wram: Box::new([0; 0x8000]),
            board_wram: Box::new([0; 0x40000]),
            cycle_count: 0,
            interrupt_master_enable: 0,
            interrupt_enable: 0,
            interrupt_request: 0,
            dma_infos: [DmaInfo::default(); 4],
            lcd: Lcd::default(),
            apu: Apu::default(),
            keypad: Keypad::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DmaAddrControl {
    Increment,
    Decrement,
    Fixed,
    IncrementReload,
}

#[derive(Clone, Copy, Debug)]
enum DmaTransferType {
    Bit16,
    Bit32,
}

#[derive(Clone, Copy, Debug)]
enum DmaStartTiming {
    Immediately,
    VBlank,
    HBlank,
    Special,
}

#[derive(Clone, Copy, Debug, Default)]
struct DmaInfo {
    source_addr: u32,
    dest_addr: u32,
    word_count: u16,
    dma_control: u16,
}

impl DmaInfo {
    fn read_source_addr<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.source_addr.get_data(index)
    }

    fn write_source_addr<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.source_addr = self.source_addr.set_data(value, index);
    }

    fn read_dest_addr<T>(&self, index: u32) -> T
    where
        u32: DataAccess<T>,
    {
        self.dest_addr.get_data(index)
    }

    fn write_dest_addr<T>(&mut self, value: T, index: u32)
    where
        u32: DataAccess<T>,
    {
        self.dest_addr = self.dest_addr.set_data(value, index);
    }

    fn read_word_count<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.word_count.get_data(index)
    }

    fn write_word_count<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.word_count = self.word_count.set_data(value, index);
    }

    fn read_dma_control<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.dma_control.get_data(index)
    }

    fn write_dma_control<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.dma_control = self.dma_control.set_data(value, index);
        // println!(
        //     "new start timing: {:?}, enabled: {}, repeat: {}",
        //     self.get_dma_start_timing(),
        //     self.get_dma_enable(),
        //     self.get_dma_repeat()
        // );
        // println!("---------");
    }
}

impl DmaInfo {
    const INCREMENT_ADDR_CONTROL: u16 = 0;
    const DECREMENT_ADDR_CONTROL: u16 = 1;
    const FIXED_ADDR_CONTROL: u16 = 2;
    const INCREMENT_RELOAD_ADDR_CONTROL: u16 = 3;

    fn get_dest_addr_control(&self) -> DmaAddrControl {
        const DEST_ADDR_CONTROL_BIT_RANGE: RangeInclusive<usize> = 5..=6;

        match self.dma_control.get_bit_range(DEST_ADDR_CONTROL_BIT_RANGE) {
            Self::INCREMENT_ADDR_CONTROL => DmaAddrControl::Increment,
            Self::DECREMENT_ADDR_CONTROL => DmaAddrControl::Decrement,
            Self::FIXED_ADDR_CONTROL => DmaAddrControl::Fixed,
            Self::INCREMENT_RELOAD_ADDR_CONTROL => DmaAddrControl::IncrementReload,
            _ => unreachable!(),
        }
    }

    fn get_source_addr_control(&self) -> DmaAddrControl {
        const DEST_ADDR_CONTROL_BIT_RANGE: RangeInclusive<usize> = 5..=6;

        match self.dma_control.get_bit_range(DEST_ADDR_CONTROL_BIT_RANGE) {
            Self::INCREMENT_ADDR_CONTROL => DmaAddrControl::Increment,
            Self::DECREMENT_ADDR_CONTROL => DmaAddrControl::Decrement,
            Self::FIXED_ADDR_CONTROL => DmaAddrControl::Fixed,
            Self::INCREMENT_RELOAD_ADDR_CONTROL => DmaAddrControl::IncrementReload,
            _ => unreachable!(),
        }
    }

    fn get_dma_repeat(&self) -> bool {
        const DMA_REPEAT_BIT_INDEX: usize = 9;

        self.dma_control.get_bit(DMA_REPEAT_BIT_INDEX)
    }

    fn get_dma_transfer_type(&self) -> DmaTransferType {
        const DMA_TRANSFER_TYPE_BIT_INDEX: usize = 8;

        if self.dma_control.get_bit(DMA_TRANSFER_TYPE_BIT_INDEX) {
            DmaTransferType::Bit32
        } else {
            DmaTransferType::Bit16
        }
    }

    fn get_dma_start_timing(&self) -> DmaStartTiming {
        const DMA_START_TIMING_BIT_RANGE: RangeInclusive<usize> = 12..=13;

        const IMMEDIATELY_START_TIMING: u16 = 0;
        const VBLANK_START_TIMING: u16 = 1;
        const HBLANK_START_TIMING: u16 = 2;
        const SPECIAL_START_TIMING: u16 = 3;

        match self.dma_control.get_bit_range(DMA_START_TIMING_BIT_RANGE) {
            IMMEDIATELY_START_TIMING => DmaStartTiming::Immediately,
            VBLANK_START_TIMING => DmaStartTiming::VBlank,
            HBLANK_START_TIMING => DmaStartTiming::HBlank,
            SPECIAL_START_TIMING => DmaStartTiming::Special,
            _ => unreachable!(),
        }
    }

    fn get_irq_at_end(&self) -> bool {
        const IRQ_AT_END_BIT_INDEX: usize = 14;

        self.dma_control.get_bit(IRQ_AT_END_BIT_INDEX)
    }

    const DMA_ENABLE_BIT_INDEX: usize = 15;

    fn get_dma_enable(&self) -> bool {
        self.dma_control.get_bit(Self::DMA_ENABLE_BIT_INDEX)
    }

    fn set_dma_enable(&mut self, set: bool) {
        self.dma_control = self.dma_control.set_bit(Self::DMA_ENABLE_BIT_INDEX, set);
    }
}

impl Bus {
    pub fn step(&mut self) {
        let lcd_interrupts = self.lcd.poll_pending_interrupts();

        if lcd_interrupts.vblank && self.lcd.get_vblank_irq_enable() {
            self.interrupt_request = self
                .interrupt_request
                .set_bit(Self::LCD_VBLANK_INTERRUPT_BIT_INDEX, true);
        }

        if lcd_interrupts.hblank && self.lcd.get_hblank_irq_enable() {
            self.interrupt_request = self
                .interrupt_request
                .set_bit(Self::LCD_HBLANK_INTERRUPT_BIT_INDEX, true);
        }

        if lcd_interrupts.vcount && self.lcd.get_vcount_irq_enable() {
            self.interrupt_request = self
                .interrupt_request
                .set_bit(Self::LCD_VCOUNT_INTERRUPT_BIT_INDEX, true);
        }

        self.step_dma(lcd_interrupts);

        if self.keypad.poll_pending_interrupts() {
            self.interrupt_request = self
                .interrupt_request
                .set_bit(Self::KEYPAD_INTERRUPT_BIT_INDEX, true);
        }

        if self.cycle_count % 4 == 0 {
            self.lcd.step();
        }

        self.cycle_count += 1;
    }
}

impl Bus {
    const BIOS_BASE: u32 = 0x00000000;
    const BIOS_END: u32 = 0x00003FFF;

    const BOARD_WRAM_BASE: u32 = 0x02000000;
    const BOARD_WRAM_END: u32 = 0x02FFFFFF;
    const BOARD_WRAM_SIZE: u32 = 0x00040000;

    const CHIP_WRAM_BASE: u32 = 0x03000000;
    const CHIP_WRAM_END: u32 = 0x03FFFFFF;
    const CHIP_WRAM_SIZE: u32 = 0x00008000;

    const LCD_CONTROL_BASE: u32 = 0x04000000;
    const LCD_CONTROL_END: u32 = Self::LCD_CONTROL_BASE + 1;

    const GREEN_SWAP_BASE: u32 = 0x04000002;
    const GREEP_SWAP_END: u32 = Self::GREEN_SWAP_BASE + 1;

    const LCD_STATUS_BASE: u32 = 0x04000004;
    const LCD_STATUS_END: u32 = Self::LCD_STATUS_BASE + 1;

    const LCD_VERTICAL_COUNTER_BASE: u32 = 0x04000006;
    const LCD_VERTICAL_COUNTER_END: u32 = Self::LCD_VERTICAL_COUNTER_BASE + 1;

    const BG0_CONTROL_BASE: u32 = 0x04000008;
    const BG0_CONTROL_END: u32 = Self::BG0_CONTROL_BASE + 1;

    const BG2_CONTROL_BASE: u32 = 0x0400000C;
    const BG2_CONTROL_END: u32 = Self::BG2_CONTROL_BASE + 1;

    const BG3_CONTROL_BASE: u32 = 0x0400000E;
    const BG3_CONTROL_END: u32 = Self::BG3_CONTROL_BASE + 1;

    const BG0_X_OFFSET_BASE: u32 = 0x04000010;
    const BG0_X_OFFSET_END: u32 = Self::BG0_X_OFFSET_BASE + 1;

    const BG0_Y_OFFSET_BASE: u32 = 0x04000012;
    const BG0_Y_OFFSET_END: u32 = Self::BG0_Y_OFFSET_BASE + 1;

    const BG2_TEXT_X_OFFSET_BASE: u32 = 0x04000018;
    const BG2_TEXT_X_OFFSET_END: u32 = Self::BG2_TEXT_X_OFFSET_BASE + 1;

    const BG2_TEXT_Y_OFFSET_BASE: u32 = 0x0400001A;
    const BG2_TEXT_Y_OFFSET_END: u32 = Self::BG2_TEXT_Y_OFFSET_BASE + 1;

    const BG3_TEXT_X_OFFSET_BASE: u32 = 0x0400001C;
    const BG3_TEXT_X_OFFSET_END: u32 = Self::BG3_TEXT_X_OFFSET_BASE + 1;

    const BG3_TEXT_Y_OFFSET_BASE: u32 = 0x0400001E;
    const BG3_TEXT_Y_OFFSET_END: u32 = Self::BG3_TEXT_Y_OFFSET_BASE + 1;

    const BG2_AFFINE_PARAM_A_BASE: u32 = 0x04000020;
    const BG2_AFFINE_PARAM_A_END: u32 = Self::BG2_AFFINE_PARAM_A_BASE + 1;

    const BG2_AFFINE_PARAM_B_BASE: u32 = 0x04000022;
    const BG2_AFFINE_PARAM_B_END: u32 = Self::BG2_AFFINE_PARAM_B_BASE + 1;

    const BG2_AFFINE_PARAM_C_BASE: u32 = 0x04000024;
    const BG2_AFFINE_PARAM_C_END: u32 = Self::BG2_AFFINE_PARAM_C_BASE + 1;

    const BG2_AFFINE_PARAM_D_BASE: u32 = 0x04000026;
    const BG2_AFFINE_PARAM_D_END: u32 = Self::BG2_AFFINE_PARAM_D_BASE + 1;

    const BG2_AFFINE_X_OFFSET_BASE: u32 = 0x04000028;
    const BG2_AFFINE_X_OFFSET_END: u32 = Self::BG2_AFFINE_X_OFFSET_BASE + 3;

    const BG2_AFFINE_Y_OFFSET_BASE: u32 = 0x0400002C;
    const BG2_AFFINE_Y_OFFSET_END: u32 = Self::BG2_AFFINE_Y_OFFSET_BASE + 3;

    const BG3_AFFINE_PARAM_A_BASE: u32 = 0x04000030;
    const BG3_AFFINE_PARAM_A_END: u32 = Self::BG3_AFFINE_PARAM_A_BASE + 1;

    const BG3_AFFINE_PARAM_B_BASE: u32 = 0x04000032;
    const BG3_AFFINE_PARAM_B_END: u32 = Self::BG3_AFFINE_PARAM_B_BASE + 1;

    const BG3_AFFINE_PARAM_C_BASE: u32 = 0x04000034;
    const BG3_AFFINE_PARAM_C_END: u32 = Self::BG3_AFFINE_PARAM_C_BASE + 1;

    const BG3_AFFINE_PARAM_D_BASE: u32 = 0x04000036;
    const BG3_AFFINE_PARAM_D_END: u32 = Self::BG3_AFFINE_PARAM_D_BASE + 1;

    const BG3_AFFINE_X_OFFSET_BASE: u32 = 0x04000038;
    const BG3_AFFINE_X_OFFSET_END: u32 = Self::BG3_AFFINE_X_OFFSET_BASE + 3;

    const BG3_AFFINE_Y_OFFSET_BASE: u32 = 0x0400003C;
    const BG3_AFFINE_Y_OFFSET_END: u32 = Self::BG3_AFFINE_Y_OFFSET_BASE + 3;

    const SOUND_PWM_CONTROL_BASE: u32 = 0x04000088;
    const SOUND_PWM_CONTROL_END: u32 = Self::SOUND_PWM_CONTROL_BASE + 1;

    const DMA_0_SOURCE_BASE: u32 = 0x040000B0;
    const DMA_0_SOURCE_END: u32 = Self::DMA_0_SOURCE_BASE + 3;

    const DMA_0_DEST_BASE: u32 = 0x040000B4;
    const DMA_0_DEST_END: u32 = Self::DMA_0_DEST_BASE + 3;

    const DMA_0_WORD_COUNT_BASE: u32 = 0x040000B8;
    const DMA_0_WORD_COUNT_END: u32 = Self::DMA_0_WORD_COUNT_BASE + 1;

    const DMA_0_CONTROL_BASE: u32 = 0x040000BA;
    const DMA_0_CONTROL_END: u32 = Self::DMA_0_CONTROL_BASE + 1;

    const DMA_1_SOURCE_BASE: u32 = 0x040000BC;
    const DMA_1_SOURCE_END: u32 = Self::DMA_1_SOURCE_BASE + 3;

    const DMA_1_DEST_BASE: u32 = 0x040000C0;
    const DMA_1_DEST_END: u32 = Self::DMA_1_DEST_BASE + 3;

    const DMA_1_WORD_COUNT_BASE: u32 = 0x040000C4;
    const DMA_1_WORD_COUNT_END: u32 = Self::DMA_1_WORD_COUNT_BASE + 1;

    const DMA_1_CONTROL_BASE: u32 = 0x040000C6;
    const DMA_1_CONTROL_END: u32 = Self::DMA_1_CONTROL_BASE + 1;

    const DMA_2_SOURCE_BASE: u32 = 0x040000C8;
    const DMA_2_SOURCE_END: u32 = Self::DMA_2_SOURCE_BASE + 3;

    const DMA_2_DEST_BASE: u32 = 0x040000CC;
    const DMA_2_DEST_END: u32 = Self::DMA_2_DEST_BASE + 3;

    const DMA_2_WORD_COUNT_BASE: u32 = 0x040000D0;
    const DMA_2_WORD_COUNT_END: u32 = Self::DMA_2_WORD_COUNT_BASE + 1;

    const DMA_2_CONTROL_BASE: u32 = 0x040000D2;
    const DMA_2_CONTROL_END: u32 = Self::DMA_2_CONTROL_BASE + 1;

    const DMA_3_SOURCE_BASE: u32 = 0x040000D4;
    const DMA_3_SOURCE_END: u32 = Self::DMA_3_SOURCE_BASE + 3;

    const DMA_3_DEST_BASE: u32 = 0x040000D8;
    const DMA_3_DEST_END: u32 = Self::DMA_3_DEST_BASE + 3;

    const DMA_3_WORD_COUNT_BASE: u32 = 0x040000DC;
    const DMA_3_WORD_COUNT_END: u32 = Self::DMA_3_WORD_COUNT_BASE + 1;

    const DMA_3_CONTROL_BASE: u32 = 0x040000DE;
    const DMA_3_CONTROL_END: u32 = Self::DMA_3_CONTROL_BASE + 1;

    const KEY_STATUS_BASE: u32 = 0x04000130;
    const KEY_STATUS_END: u32 = Self::KEY_STATUS_BASE + 1;

    const KEY_CONTROL_BASE: u32 = 0x04000132;
    const KEY_CONTROL_END: u32 = Self::KEY_CONTROL_BASE + 1;

    const SIO_JOY_RECV_BASE: u32 = 0x04000150;
    const SIO_JOY_RECV_END: u32 = Self::SIO_JOY_RECV_BASE + 3;

    const INTERRUPT_ENABLE_BASE: u32 = 0x04000200;
    const INTERRUPT_ENABLE_END: u32 = Self::INTERRUPT_ENABLE_BASE + 1;

    const INTERRUPT_REQUEST_BASE: u32 = 0x04000202;
    const INTERRUPT_REQUEST_END: u32 = Self::INTERRUPT_REQUEST_BASE + 1;

    const GAME_PAK_WAITSTATE_BASE: u32 = 0x04000204;
    const GAME_PAK_WAITSTATE_END: u32 = Self::GAME_PAK_WAITSTATE_BASE + 1;

    const INTERRUPT_MASTER_ENABLE_BASE: u32 = 0x04000208;
    const INTERRUPT_MASTER_ENABLE_END: u32 = Self::INTERRUPT_MASTER_ENABLE_BASE + 1;

    const POSTFLG_ADDR: u32 = 0x04000300;
    const HALTCNT_ADDR: u32 = 0x04000301;

    const PALETTE_RAM_BASE: u32 = 0x05000000;
    const PALETTE_RAM_END: u32 = 0x050003FF;

    const VRAM_BASE: u32 = 0x06000000;
    const VRAM_END: u32 = 0x06017FFF;

    const OAM_BASE: u32 = 0x07000000;
    const OAM_END: u32 = 0x070003FF;

    const WAIT_STATE_1_ROM_BASE: u32 = 0x08000000;
    const WAIT_STATE_1_ROM_END: u32 = 0x09FFFFFF;

    const WAIT_STATE_2_ROM_BASE: u32 = 0x0A000000;
    const WAIT_STATE_2_ROM_END: u32 = 0x0BFFFFFF;

    const WAIT_STATE_3_ROM_BASE: u32 = 0x0C000000;
    const WAIT_STATE_3_ROM_END: u32 = 0x0DFFFFFF;

    const MEMORY_SIZE: u32 = 0x10000000;

    pub fn read_byte_address(&self, address: u32) -> u8 {
        match address % Self::MEMORY_SIZE {
            Self::BIOS_BASE..=Self::BIOS_END => BIOS[(address % Self::MEMORY_SIZE) as usize],
            Self::BOARD_WRAM_BASE..=Self::BOARD_WRAM_END => {
                let actual_offset = (address - Self::BOARD_WRAM_BASE) % Self::BOARD_WRAM_SIZE;
                self.board_wram[actual_offset as usize]
            }
            Self::CHIP_WRAM_BASE..=Self::CHIP_WRAM_END => {
                let actual_offset = (address - Self::CHIP_WRAM_BASE) % Self::CHIP_WRAM_SIZE;
                self.chip_wram[actual_offset as usize]
            }
            Self::LCD_CONTROL_BASE..=Self::LCD_CONTROL_END => {
                self.lcd.read_lcd_control(address & 0b1)
            }
            Self::GREEN_SWAP_BASE..=Self::GREEP_SWAP_END => {
                // println!("STUBBED READ FROM GREEN SWAP");
                0x00
            }
            Self::LCD_STATUS_BASE..=Self::LCD_STATUS_END => self.lcd.read_lcd_status(address & 0b1),
            Self::LCD_VERTICAL_COUNTER_BASE..=Self::LCD_VERTICAL_COUNTER_END => {
                self.lcd.read_vcount(address & 0b1)
            }

            Self::BG0_CONTROL_BASE..=Self::BG0_CONTROL_END => {
                self.lcd.read_layer0_bg_control(address & 0b1)
            }
            Self::BG0_X_OFFSET_BASE..=Self::BG0_X_OFFSET_END => {
                self.lcd.read_layer0_x_offset(address & 0b1)
            }
            Self::BG0_Y_OFFSET_BASE..=Self::BG0_Y_OFFSET_END => {
                self.lcd.read_layer0_y_offset(address & 0b1)
            }

            Self::BG2_CONTROL_BASE..=Self::BG2_CONTROL_END => {
                self.lcd.read_layer2_bg_control(address & 0b1)
            }
            Self::BG2_TEXT_X_OFFSET_BASE..=Self::BG2_TEXT_X_OFFSET_END => {
                self.lcd.read_layer2_text_x_offset(address & 0b1)
            }
            Self::BG2_TEXT_Y_OFFSET_BASE..=Self::BG2_TEXT_Y_OFFSET_END => {
                self.lcd.read_layer2_text_y_offset(address & 0b1)
            }
            Self::BG2_AFFINE_X_OFFSET_BASE..=Self::BG2_AFFINE_X_OFFSET_END => {
                self.lcd.read_layer2_affine_x_offset(address & 0b11)
            }
            Self::BG2_AFFINE_Y_OFFSET_BASE..=Self::BG2_AFFINE_Y_OFFSET_END => {
                self.lcd.read_layer2_affine_y_offset(address & 0b11)
            }
            Self::BG2_AFFINE_PARAM_A_BASE..=Self::BG2_AFFINE_PARAM_A_END => {
                self.lcd.read_layer2_affine_param_a(address & 0b1)
            }
            Self::BG2_AFFINE_PARAM_B_BASE..=Self::BG2_AFFINE_PARAM_B_END => {
                self.lcd.read_layer2_affine_param_b(address & 0b1)
            }
            Self::BG2_AFFINE_PARAM_C_BASE..=Self::BG2_AFFINE_PARAM_C_END => {
                self.lcd.read_layer2_affine_param_c(address & 0b1)
            }
            Self::BG2_AFFINE_PARAM_D_BASE..=Self::BG2_AFFINE_PARAM_D_END => {
                self.lcd.read_layer2_affine_param_d(address & 0b1)
            }

            Self::BG3_CONTROL_BASE..=Self::BG3_CONTROL_END => {
                self.lcd.read_layer3_bg_control(address & 0b1)
            }
            Self::BG3_TEXT_X_OFFSET_BASE..=Self::BG3_TEXT_X_OFFSET_END => {
                self.lcd.read_layer3_text_x_offset(address & 0b1)
            }
            Self::BG3_TEXT_Y_OFFSET_BASE..=Self::BG3_TEXT_Y_OFFSET_END => {
                self.lcd.read_layer3_text_y_offset(address & 0b1)
            }
            Self::BG3_AFFINE_X_OFFSET_BASE..=Self::BG3_AFFINE_X_OFFSET_END => {
                self.lcd.read_layer3_affine_x_offset(address & 0b11)
            }
            Self::BG3_AFFINE_Y_OFFSET_BASE..=Self::BG3_AFFINE_Y_OFFSET_END => {
                self.lcd.read_layer3_affine_y_offset(address & 0b11)
            }
            Self::BG3_AFFINE_PARAM_A_BASE..=Self::BG3_AFFINE_PARAM_A_END => {
                self.lcd.read_layer3_affine_param_a(address & 0b1)
            }
            Self::BG3_AFFINE_PARAM_B_BASE..=Self::BG3_AFFINE_PARAM_B_END => {
                self.lcd.read_layer3_affine_param_b(address & 0b1)
            }
            Self::BG3_AFFINE_PARAM_C_BASE..=Self::BG3_AFFINE_PARAM_C_END => {
                self.lcd.read_layer3_affine_param_c(address & 0b1)
            }
            Self::BG3_AFFINE_PARAM_D_BASE..=Self::BG3_AFFINE_PARAM_D_END => {
                self.lcd.read_layer3_affine_param_d(address & 0b1)
            }

            Self::SOUND_PWM_CONTROL_BASE..=Self::SOUND_PWM_CONTROL_END => {
                self.apu.read_sound_bias(address & 0b1)
            }

            Self::DMA_0_SOURCE_BASE..=Self::DMA_0_SOURCE_END => {
                self.dma_infos[0].read_source_addr(address & 0b11)
            }
            Self::DMA_0_DEST_BASE..=Self::DMA_0_DEST_END => {
                self.dma_infos[0].read_dest_addr(address & 0b11)
            }
            Self::DMA_0_WORD_COUNT_BASE..=Self::DMA_0_WORD_COUNT_END => {
                self.dma_infos[0].read_word_count(address & 0b1)
            }
            Self::DMA_0_CONTROL_BASE..=Self::DMA_0_CONTROL_END => {
                self.dma_infos[0].read_dma_control(address & 0b1)
            }

            Self::DMA_1_SOURCE_BASE..=Self::DMA_1_SOURCE_END => {
                self.dma_infos[1].read_source_addr(address & 0b11)
            }
            Self::DMA_1_DEST_BASE..=Self::DMA_1_DEST_END => {
                self.dma_infos[1].read_dest_addr(address & 0b11)
            }
            Self::DMA_1_WORD_COUNT_BASE..=Self::DMA_1_WORD_COUNT_END => {
                self.dma_infos[1].read_word_count(address & 0b1)
            }
            Self::DMA_1_CONTROL_BASE..=Self::DMA_1_CONTROL_END => {
                self.dma_infos[1].read_dma_control(address & 0b1)
            }

            Self::DMA_2_SOURCE_BASE..=Self::DMA_2_SOURCE_END => {
                self.dma_infos[2].read_source_addr(address & 0b11)
            }
            Self::DMA_2_DEST_BASE..=Self::DMA_2_DEST_END => {
                self.dma_infos[2].read_dest_addr(address & 0b11)
            }
            Self::DMA_2_WORD_COUNT_BASE..=Self::DMA_2_WORD_COUNT_END => {
                self.dma_infos[2].read_word_count(address & 0b1)
            }
            Self::DMA_2_CONTROL_BASE..=Self::DMA_2_CONTROL_END => {
                self.dma_infos[2].read_dma_control(address & 0b1)
            }

            Self::DMA_3_SOURCE_BASE..=Self::DMA_3_SOURCE_END => {
                self.dma_infos[3].read_source_addr(address & 0b11)
            }
            Self::DMA_3_DEST_BASE..=Self::DMA_3_DEST_END => {
                self.dma_infos[3].read_dest_addr(address & 0b11)
            }
            Self::DMA_3_WORD_COUNT_BASE..=Self::DMA_3_WORD_COUNT_END => {
                self.dma_infos[3].read_word_count(address & 0b1)
            }
            Self::DMA_3_CONTROL_BASE..=Self::DMA_3_CONTROL_END => {
                self.dma_infos[3].read_dma_control(address & 0b1)
            }

            Self::KEY_STATUS_BASE..=Self::KEY_STATUS_END => {
                self.keypad.read_key_status(address & 0b1)
            }
            Self::KEY_CONTROL_BASE..=Self::KEY_CONTROL_END => {
                self.keypad.read_key_interrupt_control(address & 0b1)
            }

            Self::SIO_JOY_RECV_BASE..=Self::SIO_JOY_RECV_END => {
                // println!("read from stubbed SIO_JOY_RECV");
                0
            }
            Self::INTERRUPT_ENABLE_BASE..=Self::INTERRUPT_ENABLE_END => {
                self.read_interrupt_enable(address & 0b1)
            }
            Self::INTERRUPT_REQUEST_BASE..=Self::INTERRUPT_REQUEST_END => {
                self.read_interrupt_request(address & 0b1)
            }
            Self::GAME_PAK_WAITSTATE_BASE..=Self::GAME_PAK_WAITSTATE_END => {
                println!("stubbed read game_pak[{}]", address & 0b1);
                0
            }
            Self::INTERRUPT_MASTER_ENABLE_BASE..=Self::INTERRUPT_MASTER_ENABLE_END => {
                self.read_interrupt_master_enable(address & 0b1)
            }
            Self::POSTFLG_ADDR => {
                println!("UNIMPLEMENTED POSTFLG");
                0
            }
            Self::VRAM_BASE..=Self::VRAM_END => self.lcd.read_vram(address - Self::VRAM_BASE),
            Self::OAM_BASE..=Self::OAM_END => self.lcd.read_oam(address - Self::OAM_BASE),
            Self::WAIT_STATE_1_ROM_BASE..=Self::WAIT_STATE_1_ROM_END => {
                self.read_gamepak(address - Self::WAIT_STATE_1_ROM_BASE)
            }
            Self::WAIT_STATE_2_ROM_BASE..=Self::WAIT_STATE_2_ROM_END => {
                self.read_gamepak(address - Self::WAIT_STATE_2_ROM_BASE)
            }
            Self::WAIT_STATE_3_ROM_BASE..=Self::WAIT_STATE_3_ROM_END => {
                self.read_gamepak(address - Self::WAIT_STATE_3_ROM_BASE)
            }
            0x0400020a..=0x0400020b => 0,
            _ => todo!("byte read 0x{:08x}", address),
        }
    }

    pub fn read_halfword_address(&self, address: u32) -> u16 {
        assert!(address & 0b1 == 0);

        let low_byte = self.read_byte_address(address);
        let high_byte = self.read_byte_address(address + 1);

        u16::from_le_bytes([low_byte, high_byte])
    }

    pub fn read_word_address(&self, address: u32) -> u32 {
        assert!(address & 0b11 == 0);

        let low_halfword = self.read_halfword_address(address);
        let high_halfword = self.read_halfword_address(address + 2);
        u32::from(low_halfword) | (u32::from(high_halfword) << 16)
    }

    pub fn write_byte_address(&mut self, value: u8, address: u32) {
        match address % Self::MEMORY_SIZE {
            0x00000000..=0x00003FFF => {} // println!("{:02X} -> ignored BIOS write", value),
            Self::BOARD_WRAM_BASE..=Self::BOARD_WRAM_END => {
                let actual_offset = (address - Self::BOARD_WRAM_BASE) % Self::BOARD_WRAM_SIZE;
                self.board_wram[actual_offset as usize] = value;
            }
            Self::CHIP_WRAM_BASE..=Self::CHIP_WRAM_END => {
                let actual_offset = (address - Self::CHIP_WRAM_BASE) % Self::CHIP_WRAM_SIZE;
                if (0x7FFC..=0x7FFF).contains(&actual_offset) {
                    println!(
                        "INTERRUPT HANDLER WRITE 0x{:02X} -> 0x{:08X}",
                        value, actual_offset
                    );
                }
                self.chip_wram[actual_offset as usize] = value;
            }
            Self::LCD_CONTROL_BASE..=Self::LCD_CONTROL_END => {
                self.lcd.write_lcd_control(value, address & 0b1)
            }
            Self::GREEN_SWAP_BASE..=Self::GREEP_SWAP_END => {}
            Self::LCD_STATUS_BASE..=Self::LCD_STATUS_END => {
                self.lcd.write_lcd_status(value, address & 0b1)
            }
            Self::LCD_VERTICAL_COUNTER_BASE..=Self::LCD_VERTICAL_COUNTER_END => {}

            Self::BG0_CONTROL_BASE..=Self::BG0_CONTROL_END => {
                self.lcd.write_layer0_bg_control(value, address & 0b1)
            }
            Self::BG0_X_OFFSET_BASE..=Self::BG0_X_OFFSET_END => {
                self.lcd.write_layer0_x_offset(value, address & 0b1)
            }
            Self::BG0_Y_OFFSET_BASE..=Self::BG0_Y_OFFSET_END => {
                self.lcd.write_layer0_y_offset(value, address & 0b1)
            }

            Self::BG2_CONTROL_BASE..=Self::BG2_CONTROL_END => {
                self.lcd.write_layer2_bg_control(value, address & 0b1)
            }
            Self::BG2_TEXT_X_OFFSET_BASE..=Self::BG2_TEXT_X_OFFSET_END => {
                self.lcd.write_layer2_text_x_offset(value, address & 0b1)
            }
            Self::BG2_TEXT_Y_OFFSET_BASE..=Self::BG2_TEXT_Y_OFFSET_END => {
                self.lcd.write_layer2_text_y_offset(value, address & 0b1)
            }
            Self::BG2_AFFINE_X_OFFSET_BASE..=Self::BG2_AFFINE_X_OFFSET_END => {
                self.lcd.write_layer2_affine_x_offset(value, address & 0b11)
            }
            Self::BG2_AFFINE_Y_OFFSET_BASE..=Self::BG2_AFFINE_Y_OFFSET_END => {
                self.lcd.write_layer2_affine_y_offset(value, address & 0b11)
            }
            Self::BG2_AFFINE_PARAM_A_BASE..=Self::BG2_AFFINE_PARAM_A_END => {
                self.lcd.write_layer2_affine_param_a(value, address & 0b1)
            }
            Self::BG2_AFFINE_PARAM_B_BASE..=Self::BG2_AFFINE_PARAM_B_END => {
                self.lcd.write_layer2_affine_param_b(value, address & 0b1)
            }
            Self::BG2_AFFINE_PARAM_C_BASE..=Self::BG2_AFFINE_PARAM_C_END => {
                self.lcd.write_layer2_affine_param_c(value, address & 0b1)
            }
            Self::BG2_AFFINE_PARAM_D_BASE..=Self::BG2_AFFINE_PARAM_D_END => {
                self.lcd.write_layer2_affine_param_d(value, address & 0b1)
            }

            Self::BG3_CONTROL_BASE..=Self::BG3_CONTROL_END => {
                self.lcd.write_layer3_bg_control(value, address & 0b1)
            }
            Self::BG3_TEXT_X_OFFSET_BASE..=Self::BG3_TEXT_X_OFFSET_END => {
                self.lcd.write_layer3_text_x_offset(value, address & 0b1)
            }
            Self::BG3_TEXT_Y_OFFSET_BASE..=Self::BG3_TEXT_Y_OFFSET_END => {
                self.lcd.write_layer3_text_y_offset(value, address & 0b1)
            }
            Self::BG3_AFFINE_X_OFFSET_BASE..=Self::BG3_AFFINE_X_OFFSET_END => {
                self.lcd.write_layer3_affine_x_offset(value, address & 0b11)
            }
            Self::BG3_AFFINE_Y_OFFSET_BASE..=Self::BG3_AFFINE_Y_OFFSET_END => {
                self.lcd.write_layer3_affine_y_offset(value, address & 0b11)
            }
            Self::BG3_AFFINE_PARAM_A_BASE..=Self::BG3_AFFINE_PARAM_A_END => {
                self.lcd.write_layer3_affine_param_a(value, address & 0b1)
            }
            Self::BG3_AFFINE_PARAM_B_BASE..=Self::BG3_AFFINE_PARAM_B_END => {
                self.lcd.write_layer3_affine_param_b(value, address & 0b1)
            }
            Self::BG3_AFFINE_PARAM_C_BASE..=Self::BG3_AFFINE_PARAM_C_END => {
                self.lcd.write_layer3_affine_param_c(value, address & 0b1)
            }
            Self::BG3_AFFINE_PARAM_D_BASE..=Self::BG3_AFFINE_PARAM_D_END => {
                self.lcd.write_layer3_affine_param_d(value, address & 0b1)
            }

            Self::SOUND_PWM_CONTROL_BASE..=Self::SOUND_PWM_CONTROL_END => {
                self.apu.write_sound_bias(value, address & 0b1)
            }

            Self::DMA_0_SOURCE_BASE..=Self::DMA_0_SOURCE_END => {
                self.dma_infos[0].write_source_addr(value, address & 0b11)
            }
            Self::DMA_0_DEST_BASE..=Self::DMA_0_DEST_END => {
                self.dma_infos[0].write_dest_addr(value, address & 0b11)
            }
            Self::DMA_0_WORD_COUNT_BASE..=Self::DMA_0_WORD_COUNT_END => {
                self.dma_infos[0].write_word_count(value, address & 0b1)
            }
            Self::DMA_0_CONTROL_BASE..=Self::DMA_0_CONTROL_END => {
                self.dma_infos[0].write_dma_control(value, address & 0b1)
            }

            Self::DMA_1_SOURCE_BASE..=Self::DMA_1_SOURCE_END => {
                self.dma_infos[1].write_source_addr(value, address & 0b11)
            }
            Self::DMA_1_DEST_BASE..=Self::DMA_1_DEST_END => {
                self.dma_infos[1].write_dest_addr(value, address & 0b11)
            }
            Self::DMA_1_WORD_COUNT_BASE..=Self::DMA_1_WORD_COUNT_END => {
                self.dma_infos[1].write_word_count(value, address & 0b1)
            }
            Self::DMA_1_CONTROL_BASE..=Self::DMA_1_CONTROL_END => {
                self.dma_infos[1].write_dma_control(value, address & 0b1)
            }

            Self::DMA_2_SOURCE_BASE..=Self::DMA_2_SOURCE_END => {
                self.dma_infos[2].write_source_addr(value, address & 0b11)
            }
            Self::DMA_2_DEST_BASE..=Self::DMA_2_DEST_END => {
                self.dma_infos[2].write_dest_addr(value, address & 0b11)
            }
            Self::DMA_2_WORD_COUNT_BASE..=Self::DMA_2_WORD_COUNT_END => {
                self.dma_infos[2].write_word_count(value, address & 0b1)
            }
            Self::DMA_2_CONTROL_BASE..=Self::DMA_2_CONTROL_END => {
                self.dma_infos[2].write_dma_control(value, address & 0b1)
            }

            Self::DMA_3_SOURCE_BASE..=Self::DMA_3_SOURCE_END => {
                self.dma_infos[3].write_source_addr(value, address & 0b11)
            }
            Self::DMA_3_DEST_BASE..=Self::DMA_3_DEST_END => {
                self.dma_infos[3].write_dest_addr(value, address & 0b11)
            }
            Self::DMA_3_WORD_COUNT_BASE..=Self::DMA_3_WORD_COUNT_END => {
                self.dma_infos[3].write_word_count(value, address & 0b1)
            }
            Self::DMA_3_CONTROL_BASE..=Self::DMA_3_CONTROL_END => {
                self.dma_infos[3].write_dma_control(value, address & 0b1)
            }

            Self::KEY_CONTROL_BASE..=Self::KEY_CONTROL_END => self
                .keypad
                .write_key_interrupt_control(value, address & 0b1),

            Self::INTERRUPT_ENABLE_BASE..=Self::INTERRUPT_ENABLE_END => {
                self.write_interrupt_enable(value, address & 0b1)
            }
            Self::INTERRUPT_REQUEST_BASE..=Self::INTERRUPT_REQUEST_END => {
                self.write_interrupt_acknowledge(value, address & 0b1)
            }
            Self::POSTFLG_ADDR => println!("0x{:02x} -> UNIMPLEMENTED POSTFLG", value),
            Self::HALTCNT_ADDR => {} // println!("0x{:02x} -> UNIMPLEMENTED HALTCNT", value),
            Self::GAME_PAK_WAITSTATE_BASE..=Self::GAME_PAK_WAITSTATE_END => {
                println!("game_pak[{}] = 0x{:02x}", address & 0b1, value)
            }
            Self::INTERRUPT_MASTER_ENABLE_BASE..=Self::INTERRUPT_MASTER_ENABLE_END => {
                self.write_interrupt_master_enable(value, address & 0b1)
            }
            Self::PALETTE_RAM_BASE..=Self::PALETTE_RAM_END => self
                .lcd
                .write_palette_ram(value, address - Self::PALETTE_RAM_BASE),
            Self::VRAM_BASE..=Self::VRAM_END => {
                self.lcd.write_vram(value, address - Self::VRAM_BASE)
            }
            Self::OAM_BASE..=Self::OAM_END => self.lcd.write_oam(value, address - Self::OAM_BASE),
            0x04000008..=0x40001FF => {
                // println!("stubbed write 0x{:02x} -> 0x{:08x}", value, address)
            }
            0x04000206..=0x04000207 | 0x0400020A..=0x040002FF | 0x04000410..=0x04000411 => {
                println!(
                    "ignoring unused byte write of 0x{:02x} to 0x{:08x}",
                    value, address
                )
            }
            _ => todo!("0x{:02x} -> 0x{:08x}", value, address),
        }
    }

    pub fn write_halfword_address(&mut self, value: u16, address: u32) {
        assert!(address & 0b1 == 0);

        let low_byte = value.get_data(0);
        let high_byte = value.get_data(1);

        self.write_byte_address(low_byte, address);
        self.write_byte_address(high_byte, address + 1);
    }

    pub fn write_word_address(&mut self, value: u32, address: u32) {
        assert!(address & 0b11 == 0);

        let low_halfword = value as u16;
        let high_halfword = (value >> 16) as u16;

        self.write_halfword_address(low_halfword, address);
        self.write_halfword_address(high_halfword, address + 2);
    }
}

impl Bus {
    fn read_interrupt_enable<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.interrupt_enable.get_data(index)
    }

    fn write_interrupt_enable<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.interrupt_enable = self.interrupt_enable.set_data(value, index);
    }

    fn read_interrupt_master_enable<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.interrupt_master_enable.get_data(index)
    }

    fn write_interrupt_master_enable<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        self.interrupt_master_enable = self.interrupt_master_enable.set_data(value, index);
    }

    fn read_interrupt_request<T>(&self, index: u32) -> T
    where
        u16: DataAccess<T>,
    {
        self.interrupt_request.get_data(index)
    }

    fn write_interrupt_acknowledge<T>(&mut self, value: T, index: u32)
    where
        u16: DataAccess<T>,
    {
        let written_value = 0.set_data(value, index);

        // any bits which are high in the acknowledge write clear the corresponding IRQ waiting bit.
        self.interrupt_request &= !written_value;
    }

    fn read_gamepak(&self, offset: u32) -> u8 {
        let offset = offset as usize;
        if offset < ROM.len() {
            ROM[offset]
        } else {
            0
        }
    }
}

impl Bus {
    const LCD_VBLANK_INTERRUPT_BIT_INDEX: usize = 0;
    const LCD_HBLANK_INTERRUPT_BIT_INDEX: usize = 1;
    const LCD_VCOUNT_INTERRUPT_BIT_INDEX: usize = 2;
    const DMA_0_INTERRUPT_BIT_INDEX: usize = 8;
    const DMA_1_INTERRUPT_BIT_INDEX: usize = 9;
    const DMA_2_INTERRUPT_BIT_INDEX: usize = 10;
    const DMA_3_INTERRUPT_BIT_INDEX: usize = 11;
    const KEYPAD_INTERRUPT_BIT_INDEX: usize = 12;

    fn get_interrupts_enabled(&self) -> bool {
        const INTERRUPT_MASTER_ENABLE_BIT_INDEX: usize = 0;
        self.interrupt_master_enable
            .get_bit(INTERRUPT_MASTER_ENABLE_BIT_INDEX)
    }

    fn step_dma(&mut self, interrupts: LcdInterruptInfo) {
        for (dma_idx, dma) in self.dma_infos.into_iter().enumerate() {
            let dma_triggered = if dma.get_dma_enable() {
                match dma.get_dma_start_timing() {
                    DmaStartTiming::Immediately => true,
                    DmaStartTiming::VBlank => interrupts.vblank,
                    DmaStartTiming::HBlank => interrupts.hblank,
                    DmaStartTiming::Special => false,
                }
            } else {
                false
            };

            if dma_triggered {
                println!("{:?}", dma.get_dma_start_timing());
                println!("performing dma transfer");
                println!("{:#08X?}", dma);
                println!("---------------");

                let mut dma_source = dma.source_addr;
                let mut dma_dest = dma.dest_addr;
                let original_dest = dma_dest;
                let dma_length = usize::from(dma.word_count);

                for _ in 0..dma_length {
                    let transfer_size = match dma.get_dma_transfer_type() {
                        DmaTransferType::Bit16 => 2,
                        DmaTransferType::Bit32 => 4,
                    };

                    match dma.get_dma_transfer_type() {
                        DmaTransferType::Bit16 => {
                            let source_data = self.read_halfword_address(dma_source);
                            self.write_halfword_address(source_data, dma_dest);
                        }
                        DmaTransferType::Bit32 => {
                            let source_data = self.read_word_address(dma_source);
                            self.write_word_address(source_data, dma_dest);
                        }
                    }

                    match dma.get_source_addr_control() {
                        DmaAddrControl::Fixed => {}
                        DmaAddrControl::Decrement => dma_source -= transfer_size,
                        DmaAddrControl::Increment | DmaAddrControl::IncrementReload => {
                            dma_source += transfer_size
                        }
                    };

                    match dma.get_dest_addr_control() {
                        DmaAddrControl::Fixed => {}
                        DmaAddrControl::Decrement => dma_dest -= transfer_size,
                        DmaAddrControl::Increment | DmaAddrControl::IncrementReload => {
                            dma_dest += transfer_size
                        }
                    };
                }

                if matches!(dma.get_dest_addr_control(), DmaAddrControl::IncrementReload) {
                    dma_dest = original_dest;
                }

                let dma = &mut self.dma_infos[dma_idx];
                dma.source_addr = dma_source;
                dma.dest_addr = dma_dest;

                if !dma.get_dma_repeat() {
                    dma.set_dma_enable(false);
                }

                if dma.get_irq_at_end() {
                    let irq_bit_index = match dma_idx {
                        0 => Self::DMA_0_INTERRUPT_BIT_INDEX,
                        1 => Self::DMA_1_INTERRUPT_BIT_INDEX,
                        2 => Self::DMA_2_INTERRUPT_BIT_INDEX,
                        3 => Self::DMA_3_INTERRUPT_BIT_INDEX,
                        _ => unreachable!(),
                    };

                    self.interrupt_request = self.interrupt_request.set_bit(irq_bit_index, true);
                }
                return;
            }
        }
    }

    pub fn get_irq_pending(&mut self) -> bool {
        if !self.get_interrupts_enabled() {
            false
        } else {
            (self.interrupt_enable & self.interrupt_request) != 0
        }
    }
}
