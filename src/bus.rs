use std::fmt::{Debug, UpperHex};
use std::ops::RangeInclusive;

use crate::apu::Apu;
use crate::cartridge::Cartridge;
use crate::keypad::Keypad;
use crate::lcd::{Lcd, LcdStateChangeInfo};
use crate::timer::Timer;
use crate::BitManipulation;
use crate::DataAccess;

const BIOS: &[u8] = include_bytes!("../gba_bios.bin");

pub struct Bus {
    chip_wram: [u8; 0x8000],
    board_wram: [u8; 0x40000],
    cycle_count: usize,
    interrupt_master_enable: u16,
    interrupt_enable: u16,
    interrupt_request: u16,
    dma_infos: [DmaInfo; 4],
    timers: [Timer; 4],
    pub lcd: Lcd,
    pub apu: Apu,
    pub keypad: Keypad,
    pub cartridge: Cartridge,
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            chip_wram: [0; 0x8000],
            board_wram: [0; 0x40000],
            cycle_count: 0,
            interrupt_master_enable: 0,
            interrupt_enable: 0,
            interrupt_request: 0,
            dma_infos: [
                DmaInfo::default(),
                DmaInfo::default(),
                DmaInfo::default(),
                DmaInfo::default(),
            ],
            timers: [
                Timer::default(),
                Timer::default(),
                Timer::default(),
                Timer::default(),
            ],
            lcd: Lcd::default(),
            apu: Apu::default(),
            keypad: Keypad::default(),
            cartridge,
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
    dma_ongoing: bool,
}

#[derive(Clone, Copy, Debug)]
enum InterruptType {
    VBlank,
    HBlank,
    VCount,
    Timer0,
    Timer1,
    Timer2,
    Timer3,
    Serial,
    Dma0,
    Dma1,
    Dma2,
    Dma3,
    Keypad,
    Gamepak,
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
        T: UpperHex,
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
        let old_dma_enable = self.get_dma_enable();
        self.dma_control = self.dma_control.set_data(value, index);

        // DMA immedietly is started on rising edge of DMA enable.
        if !old_dma_enable
            && self.get_dma_enable()
            && matches!(self.get_dma_start_timing(), DmaStartTiming::Immediately)
        {
            self.dma_ongoing = true;
        }
    }
}

impl DmaInfo {
    fn get_dest_addr_control(&self) -> DmaAddrControl {
        const DEST_ADDR_CONTROL_BIT_RANGE: RangeInclusive<usize> = 5..=6;

        match self.dma_control.get_bit_range(DEST_ADDR_CONTROL_BIT_RANGE) {
            0 => DmaAddrControl::Increment,
            1 => DmaAddrControl::Decrement,
            2 => DmaAddrControl::Fixed,
            3 => DmaAddrControl::IncrementReload,
            _ => unreachable!(),
        }
    }

    fn get_source_addr_control(&self) -> DmaAddrControl {
        const SOURCE_ADDR_CONTROL_BIT_RANGE: RangeInclusive<usize> = 7..=8;

        match self
            .dma_control
            .get_bit_range(SOURCE_ADDR_CONTROL_BIT_RANGE)
        {
            0 => DmaAddrControl::Increment,
            1 => DmaAddrControl::Decrement,
            2 => DmaAddrControl::Fixed,
            3 => unreachable!("increment reload illegal for source control"),
            _ => unreachable!(),
        }
    }

    fn get_dma_repeat(&self) -> bool {
        const DMA_REPEAT_BIT_INDEX: usize = 9;

        self.dma_control.get_bit(DMA_REPEAT_BIT_INDEX)
    }

    fn get_dma_transfer_type(&self) -> DmaTransferType {
        const DMA_TRANSFER_TYPE_BIT_INDEX: usize = 10;

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

    fn get_dma_ongoing(&self) -> bool {
        self.dma_ongoing
    }

    fn set_dma_ongoing(&mut self, set: bool) {
        self.dma_ongoing = set;
    }
}

impl Bus {
    pub fn step(&mut self) {
        if self.keypad.poll_pending_interrupts() {
            self.request_interrupt(InterruptType::Keypad);
        }

        self.step_timers();

        if self.cycle_count % 4 == 0 {
            let state_changes = self.lcd.step();

            self.step_dma(Some(state_changes));

            if state_changes.vblank_entered && self.lcd.get_vblank_irq_enable() {
                self.request_interrupt(InterruptType::VBlank);
            }

            if state_changes.hblank_entered && self.lcd.get_hblank_irq_enable() {
                self.request_interrupt(InterruptType::HBlank);
            }

            if state_changes.vcount_matched && self.lcd.get_vcount_irq_enable() {
                self.request_interrupt(InterruptType::VCount);
            }
        } else {
            self.step_dma(None);
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

    const BG1_CONTROL_BASE: u32 = 0x0400000A;
    const BG1_CONTROL_END: u32 = Self::BG1_CONTROL_BASE + 1;

    const BG2_CONTROL_BASE: u32 = 0x0400000C;
    const BG2_CONTROL_END: u32 = Self::BG2_CONTROL_BASE + 1;

    const BG3_CONTROL_BASE: u32 = 0x0400000E;
    const BG3_CONTROL_END: u32 = Self::BG3_CONTROL_BASE + 1;

    const BG0_X_OFFSET_BASE: u32 = 0x04000010;
    const BG0_X_OFFSET_END: u32 = Self::BG0_X_OFFSET_BASE + 1;

    const BG0_Y_OFFSET_BASE: u32 = 0x04000012;
    const BG0_Y_OFFSET_END: u32 = Self::BG0_Y_OFFSET_BASE + 1;

    const BG1_X_OFFSET_BASE: u32 = 0x04000014;
    const BG1_X_OFFSET_END: u32 = Self::BG1_X_OFFSET_BASE + 1;

    const BG1_Y_OFFSET_BASE: u32 = 0x04000016;
    const BG1_Y_OFFSET_END: u32 = Self::BG1_Y_OFFSET_BASE + 1;

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

    const WINDOW_0_HORIZONTAL_BASE: u32 = 0x04000040;
    const WINDOW_0_HORIZONTAL_END: u32 = Self::WINDOW_0_HORIZONTAL_BASE + 1;

    const WINDOW_1_HORIZONTAL_BASE: u32 = 0x04000042;
    const WINDOW_1_HORIZONTAL_END: u32 = Self::WINDOW_1_HORIZONTAL_BASE + 1;

    const WINDOW_0_VERTICAL_BASE: u32 = 0x04000044;
    const WINDOW_0_VERTICAL_END: u32 = Self::WINDOW_0_VERTICAL_BASE + 1;

    const WINDOW_1_VERTICAL_BASE: u32 = 0x04000046;
    const WINDOW_1_VERTICAL_END: u32 = Self::WINDOW_1_VERTICAL_BASE + 1;

    const WINDOW_IN_CONTROL_BASE: u32 = 0x04000048;
    const WINDOW_IN_CONTROL_END: u32 = Self::WINDOW_IN_CONTROL_BASE + 1;

    const WINDOW_OUT_CONTROL_BASE: u32 = 0x0400004A;
    const WINDOW_OUT_CONTROL_END: u32 = Self::WINDOW_OUT_CONTROL_BASE + 1;

    const MOSAIC_SIZE_BASE: u32 = 0x0400004C;
    const MOSAIC_SIZE_END: u32 = Self::MOSAIC_SIZE_BASE + 3;

    const BLEND_CONTROL_BASE: u32 = 0x04000050;
    const BLEND_CONTROL_END: u32 = Self::BLEND_CONTROL_BASE + 1;

    const BLEND_ALPHA_BASE: u32 = 0x04000052;
    const BLEND_ALPHA_END: u32 = Self::BLEND_ALPHA_BASE + 1;

    const BLEND_BRIGHTNESS_BASE: u32 = 0x04000054;
    const BLEND_BRIGHTNESS_END: u32 = Self::BLEND_BRIGHTNESS_BASE + 1;

    const SOUND_BASE: u32 = 0x04000060;
    const SOUND_END: u32 = 0x040000A8;

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

    const TIMER_0_COUNTER_RELOAD_BASE: u32 = 0x04000100;
    const TIMER_0_COUNTER_RELOAD_END: u32 = Self::TIMER_0_COUNTER_RELOAD_BASE + 1;

    const TIMER_0_CONTROL_BASE: u32 = 0x04000102;
    const TIMER_0_CONTROL_END: u32 = Self::TIMER_0_CONTROL_BASE + 1;

    const TIMER_1_COUNTER_RELOAD_BASE: u32 = 0x04000104;
    const TIMER_1_COUNTER_RELOAD_END: u32 = Self::TIMER_1_COUNTER_RELOAD_BASE + 1;

    const TIMER_1_CONTROL_BASE: u32 = 0x04000106;
    const TIMER_1_CONTROL_END: u32 = Self::TIMER_1_CONTROL_BASE + 1;

    const TIMER_2_COUNTER_RELOAD_BASE: u32 = 0x04000108;
    const TIMER_2_COUNTER_RELOAD_END: u32 = Self::TIMER_2_COUNTER_RELOAD_BASE + 1;

    const TIMER_2_CONTROL_BASE: u32 = 0x0400010A;
    const TIMER_2_CONTROL_END: u32 = Self::TIMER_2_CONTROL_BASE + 1;

    const TIMER_3_COUNTER_RELOAD_BASE: u32 = 0x0400010C;
    const TIMER_3_COUNTER_RELOAD_END: u32 = Self::TIMER_3_COUNTER_RELOAD_BASE + 1;

    const TIMER_3_CONTROL_BASE: u32 = 0x0400010E;
    const TIMER_3_CONTROL_END: u32 = Self::TIMER_3_CONTROL_BASE + 1;

    const SERIAL_BASE: u32 = 0x04000120;
    const SERIAL_END: u32 = 0x0400015B;

    const SIO_CONTROL_BASE: u32 = 0x04000128;
    const SIO_CONTROL_END: u32 = Self::SIO_CONTROL_BASE + 1;

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

    const UNUSED_IO_BASE: u32 = 0x04000302;
    const UNUSED_IO_END: u32 = 0x04FFFFFF;

    const PALETTE_RAM_BASE: u32 = 0x05000000;
    const PALETTE_RAM_END: u32 = 0x05FFFFFF;
    const PALETTER_RAM_SIZE: u32 = 0x400;

    const VRAM_BASE: u32 = 0x06000000;
    const VRAM_END: u32 = 0x06FFFFFF;
    const VRAM_FULL_SIZE: u32 = 0x20000;
    const VRAM_OFFSET_FIRST_BASE: u32 = 0x00000;
    const VRAM_OFFSET_FIRST_END: u32 = 0x0FFFF;
    const VRAM_OFFSET_SECOND_BASE: u32 = 0x10000;
    const VRAM_OFFSET_SECOND_END: u32 = 0x1FFFF;
    const VRAM_SECOND_SIZE: u32 = 0x8000;

    const OAM_BASE: u32 = 0x07000000;
    const OAM_END: u32 = 0x07FFFFFF;
    const OAM_SIZE: u32 = 0x00000400;

    const WAIT_STATE_1_ROM_BASE: u32 = 0x08000000;
    const WAIT_STATE_1_ROM_END: u32 = 0x09FFFFFF;

    const WAIT_STATE_2_ROM_BASE: u32 = 0x0A000000;
    const WAIT_STATE_2_ROM_END: u32 = 0x0BFFFFFF;

    const WAIT_STATE_3_ROM_BASE: u32 = 0x0C000000;
    const WAIT_STATE_3_ROM_END: u32 = 0x0DFFFFFF;

    const GAME_PAK_SRAM_BASE: u32 = 0x0E000000;
    const GAME_PAK_SRAM_END: u32 = 0x0FFFFFFF;

    pub fn read_byte_address(&self, address: u32) -> u8 {
        match address {
            Self::BIOS_BASE..=Self::BIOS_END => BIOS[address as usize],
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

            Self::BG1_CONTROL_BASE..=Self::BG1_CONTROL_END => {
                self.lcd.read_layer1_bg_control(address & 0b1)
            }
            Self::BG1_X_OFFSET_BASE..=Self::BG1_X_OFFSET_END => {
                self.lcd.read_layer1_x_offset(address & 0b1)
            }
            Self::BG1_Y_OFFSET_BASE..=Self::BG1_Y_OFFSET_END => {
                self.lcd.read_layer1_y_offset(address & 0b1)
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

            Self::WINDOW_0_HORIZONTAL_BASE..=Self::WINDOW_0_HORIZONTAL_END => {
                self.lcd.read_window_0_horizontal(address & 0b1)
            }
            Self::WINDOW_1_HORIZONTAL_BASE..=Self::WINDOW_1_HORIZONTAL_END => {
                self.lcd.read_window_1_horizontal(address & 0b1)
            }
            Self::WINDOW_0_VERTICAL_BASE..=Self::WINDOW_0_VERTICAL_END => {
                self.lcd.read_window_0_vertical(address & 0b1)
            }
            Self::WINDOW_1_VERTICAL_BASE..=Self::WINDOW_1_VERTICAL_END => {
                self.lcd.read_window_1_vertical(address & 0b1)
            }
            Self::WINDOW_IN_CONTROL_BASE..=Self::WINDOW_IN_CONTROL_END => {
                self.lcd.read_window_in_control(address & 0b1)
            }
            Self::WINDOW_OUT_CONTROL_BASE..=Self::WINDOW_OUT_CONTROL_END => {
                self.lcd.read_window_out_control(address & 0b1)
            }

            Self::MOSAIC_SIZE_BASE..=Self::MOSAIC_SIZE_END => {
                self.lcd.read_mosaic_size(address & 0b1)
            }
            Self::BLEND_CONTROL_BASE..=Self::BLEND_CONTROL_END => {
                self.lcd.read_color_effects_selection(address & 0b1)
            }
            Self::BLEND_ALPHA_BASE..=Self::BLEND_ALPHA_END => {
                self.lcd.read_alpha_blending_coefficients(address & 0b1)
            }
            Self::BLEND_BRIGHTNESS_BASE..=Self::BLEND_BRIGHTNESS_END => {
                self.lcd.read_brightness_coefficient(address & 0b1)
            }

            Self::SOUND_PWM_CONTROL_BASE..=Self::SOUND_PWM_CONTROL_END => {
                self.apu.read_sound_bias(address & 0b1)
            }

            Self::SOUND_BASE..=Self::SOUND_END => 0,

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

            Self::TIMER_0_CONTROL_BASE..=Self::TIMER_0_CONTROL_END => {
                self.timers[0].read_timer_control(address & 0b1)
            }
            Self::TIMER_0_COUNTER_RELOAD_BASE..=Self::TIMER_0_COUNTER_RELOAD_END => {
                self.timers[0].read_timer_counter_reload(address & 0b1)
            }

            Self::TIMER_1_CONTROL_BASE..=Self::TIMER_1_CONTROL_END => {
                self.timers[1].read_timer_control(address & 0b1)
            }
            Self::TIMER_1_COUNTER_RELOAD_BASE..=Self::TIMER_1_COUNTER_RELOAD_END => {
                self.timers[1].read_timer_counter_reload(address & 0b1)
            }

            Self::TIMER_2_CONTROL_BASE..=Self::TIMER_2_CONTROL_END => {
                self.timers[2].read_timer_control(address & 0b1)
            }
            Self::TIMER_2_COUNTER_RELOAD_BASE..=Self::TIMER_2_COUNTER_RELOAD_END => {
                self.timers[2].read_timer_counter_reload(address & 0b1)
            }

            Self::TIMER_3_CONTROL_BASE..=Self::TIMER_3_CONTROL_END => {
                self.timers[3].read_timer_control(address & 0b1)
            }
            Self::TIMER_3_COUNTER_RELOAD_BASE..=Self::TIMER_3_COUNTER_RELOAD_END => {
                self.timers[3].read_timer_counter_reload(address & 0b1)
            }

            Self::SIO_CONTROL_BASE..=Self::SIO_CONTROL_END => {
                // println!("read from stubbed SIOCNT");
                0
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
            Self::UNUSED_IO_BASE..=Self::UNUSED_IO_END => {
                // println!("READ from {:08X}", address);
                0
            }
            Self::GAME_PAK_WAITSTATE_BASE..=Self::GAME_PAK_WAITSTATE_END => {
                // println!("stubbed read game_pak[{}]", address & 0b1);
                0
            }
            Self::INTERRUPT_MASTER_ENABLE_BASE..=Self::INTERRUPT_MASTER_ENABLE_END => {
                self.read_interrupt_master_enable(address & 0b1)
            }
            Self::POSTFLG_ADDR => {
                println!("UNIMPLEMENTED POSTFLG");
                0
            }
            Self::PALETTE_RAM_BASE..=Self::PALETTE_RAM_END => {
                let offset = (address - Self::PALETTE_RAM_BASE) % Self::PALETTER_RAM_SIZE;
                self.lcd.read_palette_ram(offset)
            }
            Self::VRAM_BASE..=Self::VRAM_END => {
                let vram_offset = (address - Self::VRAM_BASE) % Self::VRAM_FULL_SIZE;
                let offset = match vram_offset {
                    Self::VRAM_OFFSET_FIRST_BASE..=Self::VRAM_OFFSET_FIRST_END => vram_offset,
                    Self::VRAM_OFFSET_SECOND_BASE..=Self::VRAM_OFFSET_SECOND_END => {
                        ((vram_offset - Self::VRAM_OFFSET_SECOND_BASE) % Self::VRAM_SECOND_SIZE)
                            + Self::VRAM_OFFSET_SECOND_BASE
                    }
                    _ => unreachable!(),
                };
                self.lcd.read_vram(offset)
            }
            Self::OAM_BASE..=Self::OAM_END => {
                let offset = (address - Self::OAM_BASE) % Self::OAM_SIZE;
                self.lcd.read_oam(offset)
            }
            Self::WAIT_STATE_1_ROM_BASE..=Self::WAIT_STATE_1_ROM_END => self
                .cartridge
                .read_rom_byte(address - Self::WAIT_STATE_1_ROM_BASE),
            Self::WAIT_STATE_2_ROM_BASE..=Self::WAIT_STATE_2_ROM_END => self
                .cartridge
                .read_rom_byte(address - Self::WAIT_STATE_2_ROM_BASE),
            Self::WAIT_STATE_3_ROM_BASE..=Self::WAIT_STATE_3_ROM_END => self
                .cartridge
                .read_rom_byte(address - Self::WAIT_STATE_3_ROM_BASE),
            Self::GAME_PAK_SRAM_BASE..=Self::GAME_PAK_SRAM_END => self
                .cartridge
                .read_sram_byte(address - Self::GAME_PAK_SRAM_BASE),
            Self::SERIAL_BASE..=Self::SERIAL_END => {
                // println!("read from stubbed serial {:08X}", address);
                0
            }
            _ => 0xFF,
        }
    }

    pub fn read_halfword_address(&mut self, address: u32) -> u16 {
        assert!(address & 0b1 == 0);

        match address {
            Self::WAIT_STATE_1_ROM_BASE..=Self::WAIT_STATE_1_ROM_END => self
                .cartridge
                .read_rom_hword(address - Self::WAIT_STATE_1_ROM_BASE),
            Self::WAIT_STATE_2_ROM_BASE..=Self::WAIT_STATE_2_ROM_END => self
                .cartridge
                .read_rom_hword(address - Self::WAIT_STATE_2_ROM_BASE),
            Self::WAIT_STATE_3_ROM_BASE..=Self::WAIT_STATE_3_ROM_END => self
                .cartridge
                .read_rom_hword(address - Self::WAIT_STATE_3_ROM_BASE),
            Self::GAME_PAK_SRAM_BASE..=Self::GAME_PAK_SRAM_END => self
                .cartridge
                .read_sram_hword(address - Self::GAME_PAK_SRAM_BASE),
            _ => {
                let low_byte = self.read_byte_address(address);
                let high_byte = self.read_byte_address(address + 1);

                u16::from_le_bytes([low_byte, high_byte])
            }
        }
    }

    pub fn read_word_address(&self, address: u32) -> u32 {
        assert!(address & 0b11 == 0);

        match address {
            Self::WAIT_STATE_1_ROM_BASE..=Self::WAIT_STATE_1_ROM_END => self
                .cartridge
                .read_rom_word(address - Self::WAIT_STATE_1_ROM_BASE),
            Self::WAIT_STATE_2_ROM_BASE..=Self::WAIT_STATE_2_ROM_END => self
                .cartridge
                .read_rom_word(address - Self::WAIT_STATE_2_ROM_BASE),
            Self::WAIT_STATE_3_ROM_BASE..=Self::WAIT_STATE_3_ROM_END => self
                .cartridge
                .read_rom_word(address - Self::WAIT_STATE_3_ROM_BASE),
            Self::GAME_PAK_SRAM_BASE..=Self::GAME_PAK_SRAM_END => self
                .cartridge
                .read_sram_word(address - Self::GAME_PAK_SRAM_BASE),
            _ => {
                let le_bytes = [
                    self.read_byte_address(address),
                    self.read_byte_address(address + 1),
                    self.read_byte_address(address + 2),
                    self.read_byte_address(address + 3),
                ];

                u32::from_le_bytes(le_bytes)
            }
        }
    }

    pub fn write_byte_address(&mut self, value: u8, address: u32) {
        match address {
            Self::BIOS_BASE..=Self::BIOS_END => {} // println!("{:02X} -> ignored BIOS write", value),
            Self::BOARD_WRAM_BASE..=Self::BOARD_WRAM_END => {
                let actual_offset = (address - Self::BOARD_WRAM_BASE) % Self::BOARD_WRAM_SIZE;
                self.board_wram[actual_offset as usize] = value;
            }
            Self::CHIP_WRAM_BASE..=Self::CHIP_WRAM_END => {
                let actual_offset = (address - Self::CHIP_WRAM_BASE) % Self::CHIP_WRAM_SIZE;
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

            Self::BG1_CONTROL_BASE..=Self::BG1_CONTROL_END => {
                self.lcd.write_layer1_bg_control(value, address & 0b1)
            }
            Self::BG1_X_OFFSET_BASE..=Self::BG1_X_OFFSET_END => {
                self.lcd.write_layer1_x_offset(value, address & 0b1)
            }
            Self::BG1_Y_OFFSET_BASE..=Self::BG1_Y_OFFSET_END => {
                self.lcd.write_layer1_y_offset(value, address & 0b1)
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

            Self::WINDOW_0_HORIZONTAL_BASE..=Self::WINDOW_0_HORIZONTAL_END => self
                .lcd
                .write_window_0_horizontal(value, address.get_bit_range(0..=0)),
            Self::WINDOW_1_HORIZONTAL_BASE..=Self::WINDOW_1_HORIZONTAL_END => self
                .lcd
                .write_window_1_horizontal(value, address.get_bit_range(0..=0)),
            Self::WINDOW_0_VERTICAL_BASE..=Self::WINDOW_0_VERTICAL_END => self
                .lcd
                .write_window_0_vertical(value, address.get_bit_range(0..=0)),
            Self::WINDOW_1_VERTICAL_BASE..=Self::WINDOW_1_VERTICAL_END => self
                .lcd
                .write_window_1_vertical(value, address.get_bit_range(0..=0)),
            Self::WINDOW_IN_CONTROL_BASE..=Self::WINDOW_IN_CONTROL_END => self
                .lcd
                .write_window_in_control(value, address.get_bit_range(0..=0)),
            Self::WINDOW_OUT_CONTROL_BASE..=Self::WINDOW_OUT_CONTROL_END => self
                .lcd
                .write_window_out_control(value, address.get_bit_range(0..=0)),

            Self::MOSAIC_SIZE_BASE..=Self::MOSAIC_SIZE_END => self
                .lcd
                .write_mosaic_size(value, address.get_bit_range(0..=1)),
            Self::BLEND_CONTROL_BASE..=Self::BLEND_CONTROL_END => self
                .lcd
                .write_color_effects_selection(value, address.get_bit_range(0..=0)),
            Self::BLEND_ALPHA_BASE..=Self::BLEND_ALPHA_END => self
                .lcd
                .write_alpha_blending_coefficients(value, address.get_bit_range(0..=0)),
            Self::BLEND_BRIGHTNESS_BASE..=Self::BLEND_BRIGHTNESS_END => self
                .lcd
                .write_brightness_coefficient(value, address.get_bit_range(0..=0)),

            Self::SOUND_PWM_CONTROL_BASE..=Self::SOUND_PWM_CONTROL_END => {
                self.apu.write_sound_bias(value, address & 0b1)
            }
            Self::SOUND_BASE..=Self::SOUND_END => {
                // println!("stubbed sound write {:02X} -> [{:08X}]", value, address)
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

            Self::TIMER_0_CONTROL_BASE..=Self::TIMER_0_CONTROL_END => {
                self.timers[0].write_timer_control(value, address & 0b1)
            }
            Self::TIMER_0_COUNTER_RELOAD_BASE..=Self::TIMER_0_COUNTER_RELOAD_END => {
                self.timers[0].write_timer_counter_reload(value, address & 0b1)
            }

            Self::TIMER_1_CONTROL_BASE..=Self::TIMER_1_CONTROL_END => {
                self.timers[1].write_timer_control(value, address & 0b1)
            }
            Self::TIMER_1_COUNTER_RELOAD_BASE..=Self::TIMER_1_COUNTER_RELOAD_END => {
                self.timers[1].write_timer_counter_reload(value, address & 0b1)
            }

            Self::TIMER_2_CONTROL_BASE..=Self::TIMER_2_CONTROL_END => {
                self.timers[2].write_timer_control(value, address & 0b1)
            }
            Self::TIMER_2_COUNTER_RELOAD_BASE..=Self::TIMER_2_COUNTER_RELOAD_END => {
                self.timers[2].write_timer_counter_reload(value, address & 0b1)
            }

            Self::TIMER_3_CONTROL_BASE..=Self::TIMER_3_CONTROL_END => {
                self.timers[3].write_timer_control(value, address & 0b1)
            }
            Self::TIMER_3_COUNTER_RELOAD_BASE..=Self::TIMER_3_COUNTER_RELOAD_END => {
                self.timers[3].write_timer_counter_reload(value, address & 0b1)
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
            Self::UNUSED_IO_BASE..=Self::UNUSED_IO_END => {
                // println!("WRITE of {:02X} -> {:08X}", value, address)
            }
            Self::GAME_PAK_WAITSTATE_BASE..=Self::GAME_PAK_WAITSTATE_END => {
                // println!("game_pak[{}] = 0x{:02x}", address & 0b1, value)
            }
            Self::INTERRUPT_MASTER_ENABLE_BASE..=Self::INTERRUPT_MASTER_ENABLE_END => {
                self.write_interrupt_master_enable(value, address & 0b1)
            }
            Self::VRAM_BASE..=Self::VRAM_END => {
                let vram_offset = (address - Self::VRAM_BASE) % Self::VRAM_FULL_SIZE;
                let offset = match vram_offset {
                    Self::VRAM_OFFSET_FIRST_BASE..=Self::VRAM_OFFSET_FIRST_END => vram_offset,
                    Self::VRAM_OFFSET_SECOND_BASE..=Self::VRAM_OFFSET_SECOND_END => {
                        ((vram_offset - Self::VRAM_OFFSET_SECOND_BASE) % Self::VRAM_SECOND_SIZE)
                            + Self::VRAM_OFFSET_SECOND_BASE
                    }
                    _ => unreachable!(),
                };
                self.lcd.write_vram_byte(value, offset)
            }
            Self::PALETTE_RAM_BASE..=Self::PALETTE_RAM_END => {
                let offset = (address - Self::PALETTE_RAM_BASE) % Self::PALETTER_RAM_SIZE;
                self.lcd.write_palette_ram_byte(value, offset)
            }
            Self::OAM_BASE..=Self::OAM_END => {
                let offset = (address - Self::OAM_BASE) % Self::OAM_SIZE;
                self.lcd.write_oam_byte(value, offset);
            }
            0x04000008..=0x40001FF => {
                // println!("stubbed write 0x{:02x} -> 0x{:08x}", value, address)
            }
            0x04000206..=0x04000207 | 0x0400020A..=0x040002FF => {
                // println!(
                //     "ignoring unused byte write of 0x{:02x} to 0x{:08x}",
                //     value, address
                // )
            }
            Self::WAIT_STATE_1_ROM_BASE..=Self::WAIT_STATE_1_ROM_END => {
                self.cartridge
                    .write_rom_byte(value, address - Self::WAIT_STATE_1_ROM_BASE);
            }
            Self::WAIT_STATE_2_ROM_BASE..=Self::WAIT_STATE_2_ROM_END => {
                self.cartridge
                    .write_rom_byte(value, address - Self::WAIT_STATE_2_ROM_BASE);
            }
            Self::WAIT_STATE_3_ROM_BASE..=Self::WAIT_STATE_3_ROM_END => {
                self.cartridge
                    .write_rom_byte(value, address - Self::WAIT_STATE_3_ROM_BASE);
            }
            Self::GAME_PAK_SRAM_BASE..=Self::GAME_PAK_SRAM_END => {
                self.cartridge
                    .write_sram_byte(value, address - Self::GAME_PAK_SRAM_BASE);
            }
            _ => todo!("0x{:02x} -> 0x{:08x}", value, address),
        }
    }

    pub fn write_halfword_address(&mut self, value: u16, address: u32) {
        assert!(address & 0b1 == 0);

        match address {
            Self::OAM_BASE..=Self::OAM_END => {
                let offset = (address - Self::OAM_BASE) % Self::OAM_SIZE;

                self.lcd.write_oam_hword(value, offset);
            }
            Self::PALETTE_RAM_BASE..=Self::PALETTE_RAM_END => {
                let offset = (address - Self::PALETTE_RAM_BASE) % Self::PALETTER_RAM_SIZE;
                self.lcd.write_palette_ram_hword(value, offset)
            }
            Self::VRAM_BASE..=Self::VRAM_END => {
                let vram_offset = (address - Self::VRAM_BASE) % Self::VRAM_FULL_SIZE;
                let offset = match vram_offset {
                    Self::VRAM_OFFSET_FIRST_BASE..=Self::VRAM_OFFSET_FIRST_END => vram_offset,
                    Self::VRAM_OFFSET_SECOND_BASE..=Self::VRAM_OFFSET_SECOND_END => {
                        ((vram_offset - Self::VRAM_OFFSET_SECOND_BASE) % Self::VRAM_SECOND_SIZE)
                            + Self::VRAM_OFFSET_SECOND_BASE
                    }
                    _ => unreachable!(),
                };
                self.lcd.write_vram_hword(value, offset)
            }
            Self::WAIT_STATE_1_ROM_BASE..=Self::WAIT_STATE_1_ROM_END => {
                self.cartridge
                    .write_rom_hword(value, address - Self::WAIT_STATE_1_ROM_BASE);
            }
            Self::WAIT_STATE_2_ROM_BASE..=Self::WAIT_STATE_2_ROM_END => {
                self.cartridge
                    .write_rom_hword(value, address - Self::WAIT_STATE_2_ROM_BASE);
            }
            Self::WAIT_STATE_3_ROM_BASE..=Self::WAIT_STATE_3_ROM_END => {
                self.cartridge
                    .write_rom_hword(value, address - Self::WAIT_STATE_3_ROM_BASE);
            }
            Self::GAME_PAK_SRAM_BASE..=Self::GAME_PAK_SRAM_END => {
                self.cartridge
                    .write_sram_hword(value, address - Self::GAME_PAK_SRAM_BASE);
            }
            _ => {
                let [low_byte, high_byte] = value.to_le_bytes();

                self.write_byte_address(low_byte, address);
                self.write_byte_address(high_byte, address + 1);
            }
        }
    }

    pub fn write_word_address(&mut self, value: u32, address: u32) {
        assert!(address & 0b11 == 0);

        match address {
            Self::OAM_BASE..=Self::OAM_END => {
                let offset = (address - Self::OAM_BASE) % Self::OAM_SIZE;

                self.lcd.write_oam_word(value, offset);
            }
            Self::PALETTE_RAM_BASE..=Self::PALETTE_RAM_END => {
                let offset = (address - Self::PALETTE_RAM_BASE) % Self::PALETTER_RAM_SIZE;
                self.lcd.write_palette_ram_word(value, offset)
            }
            Self::VRAM_BASE..=Self::VRAM_END => {
                let vram_offset = (address - Self::VRAM_BASE) % Self::VRAM_FULL_SIZE;
                let offset = match vram_offset {
                    Self::VRAM_OFFSET_FIRST_BASE..=Self::VRAM_OFFSET_FIRST_END => vram_offset,
                    Self::VRAM_OFFSET_SECOND_BASE..=Self::VRAM_OFFSET_SECOND_END => {
                        ((vram_offset - Self::VRAM_OFFSET_SECOND_BASE) % Self::VRAM_SECOND_SIZE)
                            + Self::VRAM_OFFSET_SECOND_BASE
                    }
                    _ => unreachable!(),
                };
                self.lcd.write_vram_word(value, offset)
            }
            Self::WAIT_STATE_1_ROM_BASE..=Self::WAIT_STATE_1_ROM_END => {
                self.cartridge
                    .write_rom_word(value, address - Self::WAIT_STATE_1_ROM_BASE);
            }
            Self::WAIT_STATE_2_ROM_BASE..=Self::WAIT_STATE_2_ROM_END => {
                self.cartridge
                    .write_rom_word(value, address - Self::WAIT_STATE_2_ROM_BASE);
            }
            Self::WAIT_STATE_3_ROM_BASE..=Self::WAIT_STATE_3_ROM_END => {
                self.cartridge
                    .write_rom_word(value, address - Self::WAIT_STATE_3_ROM_BASE);
            }
            Self::GAME_PAK_SRAM_BASE..=Self::GAME_PAK_SRAM_END => {
                self.cartridge
                    .write_sram_word(value, address - Self::GAME_PAK_SRAM_BASE);
            }
            _ => {
                for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
                    let offset = offset as u32;

                    self.write_byte_address(byte, address + offset);
                }
            }
        }
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
}

impl Bus {
    const LCD_VBLANK_INTERRUPT_BIT_INDEX: usize = 0;
    const LCD_HBLANK_INTERRUPT_BIT_INDEX: usize = 1;
    const LCD_VCOUNT_INTERRUPT_BIT_INDEX: usize = 2;
    const TIMER_0_OVERFLOW_INTERRUPT_BIT_INDEX: usize = 3;
    const TIMER_1_OVERFLOW_INTERRUPT_BIT_INDEX: usize = 4;
    const TIMER_2_OVERFLOW_INTERRUPT_BIT_INDEX: usize = 5;
    const TIMER_3_OVERFLOW_INTERRUPT_BIT_INDEX: usize = 6;
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

    fn step_dma(&mut self, state_changes: Option<LcdStateChangeInfo>) {
        for dma_idx in 0..self.dma_infos.len() {
            let original_dma_info = self.dma_infos[dma_idx];

            let dma_triggered = if original_dma_info.get_dma_enable() {
                match original_dma_info.get_dma_start_timing() {
                    DmaStartTiming::Immediately => false,
                    DmaStartTiming::VBlank => {
                        state_changes.map(|s| s.vblank_entered).unwrap_or(false)
                    }
                    DmaStartTiming::HBlank => {
                        state_changes.map(|s| s.hblank_entered).unwrap_or(false)
                    }
                    DmaStartTiming::Special => false,
                }
            } else {
                false
            };

            let dma = &mut self.dma_infos[dma_idx];
            if dma_triggered {
                dma.set_dma_ongoing(true);
            }

            if original_dma_info.get_dma_ongoing() {
                // println!("{:?}", original_dma_info.get_dma_start_timing());
                // println!("performing dma transfer");
                // println!("{:#08X?}", original_dma_info);
                // println!("dma idx: {}", dma_idx);
                // println!("---------------");

                let mut dma_source = original_dma_info.source_addr;
                let mut dma_dest = original_dma_info.dest_addr;
                let original_dest = dma_dest;
                let dma_length = usize::from(original_dma_info.word_count);

                for _ in 0..dma_length {
                    let transfer_size = match original_dma_info.get_dma_transfer_type() {
                        DmaTransferType::Bit16 => 2,
                        DmaTransferType::Bit32 => 4,
                    };

                    match original_dma_info.get_dma_transfer_type() {
                        DmaTransferType::Bit16 => {
                            let align_addr = |address| address & (!0b1);
                            let source_data = self.read_halfword_address(align_addr(dma_source));
                            self.write_halfword_address(source_data, align_addr(dma_dest));
                            // println!("u16 {:04X} -> [{:08X}]", source_data, dma_dest)
                        }
                        DmaTransferType::Bit32 => {
                            let align_addr = |address| address & (!0b11);
                            let source_data = self.read_word_address(align_addr(dma_source));
                            self.write_word_address(source_data, align_addr(dma_dest));
                            // println!("u32 {:08X} -> [{:08X}]", source_data, dma_dest)
                        }
                    }

                    match original_dma_info.get_source_addr_control() {
                        DmaAddrControl::Fixed => {}
                        DmaAddrControl::Decrement => dma_source -= transfer_size,
                        DmaAddrControl::Increment | DmaAddrControl::IncrementReload => {
                            dma_source += transfer_size
                        }
                    };

                    match original_dma_info.get_dest_addr_control() {
                        DmaAddrControl::Fixed => {}
                        DmaAddrControl::Decrement => dma_dest -= transfer_size,
                        DmaAddrControl::Increment | DmaAddrControl::IncrementReload => {
                            dma_dest += transfer_size
                        }
                    };
                }

                if matches!(
                    original_dma_info.get_dest_addr_control(),
                    DmaAddrControl::IncrementReload
                ) {
                    dma_dest = original_dest;
                }

                let dma = &mut self.dma_infos[dma_idx];
                dma.source_addr = dma_source;
                dma.dest_addr = dma_dest;

                if !dma.get_dma_repeat() {
                    dma.set_dma_enable(false);
                }
                dma.set_dma_ongoing(false);

                if dma.get_irq_at_end() {
                    let interrupt_type = match dma_idx {
                        0 => InterruptType::Dma0,
                        1 => InterruptType::Dma1,
                        2 => InterruptType::Dma2,
                        3 => InterruptType::Dma3,
                        _ => unreachable!(),
                    };

                    self.request_interrupt(interrupt_type);
                }
            }
        }
    }

    fn step_timers(&mut self) {
        let mut timer_overflow = false;
        let mut interrupt_requests = [false; 4];

        for (i, timer) in self.timers.iter_mut().enumerate() {
            timer_overflow = timer.step(timer_overflow);

            if timer_overflow && timer.get_timer_irq_enable() {
                interrupt_requests[i] = true;
            }
        }

        for (i, requested) in interrupt_requests.into_iter().enumerate() {
            if requested {
                let interrupt_type = match i {
                    0 => InterruptType::Timer0,
                    1 => InterruptType::Timer1,
                    2 => InterruptType::Timer2,
                    3 => InterruptType::Timer3,
                    _ => unreachable!(),
                };

                self.request_interrupt(interrupt_type);
            }
        }
    }

    fn request_interrupt(&mut self, interrupt: InterruptType) {
        let bit_index = match interrupt {
            InterruptType::VBlank => Self::LCD_VBLANK_INTERRUPT_BIT_INDEX,
            InterruptType::HBlank => Self::LCD_HBLANK_INTERRUPT_BIT_INDEX,
            InterruptType::VCount => Self::LCD_VCOUNT_INTERRUPT_BIT_INDEX,
            InterruptType::Timer0 => Self::TIMER_0_OVERFLOW_INTERRUPT_BIT_INDEX,
            InterruptType::Timer1 => Self::TIMER_1_OVERFLOW_INTERRUPT_BIT_INDEX,
            InterruptType::Timer2 => Self::TIMER_2_OVERFLOW_INTERRUPT_BIT_INDEX,
            InterruptType::Timer3 => Self::TIMER_3_OVERFLOW_INTERRUPT_BIT_INDEX,
            InterruptType::Dma0 => Self::DMA_0_INTERRUPT_BIT_INDEX,
            InterruptType::Dma1 => Self::DMA_1_INTERRUPT_BIT_INDEX,
            InterruptType::Dma2 => Self::DMA_2_INTERRUPT_BIT_INDEX,
            InterruptType::Dma3 => Self::DMA_3_INTERRUPT_BIT_INDEX,
            InterruptType::Keypad => Self::KEYPAD_INTERRUPT_BIT_INDEX,
            _ => todo!(),
        };

        self.interrupt_request = self.interrupt_request.set_bit(bit_index, true);
    }

    pub fn get_irq_pending(&mut self) -> bool {
        if !self.get_interrupts_enabled() {
            false
        } else {
            (self.interrupt_enable & self.interrupt_request) != 0
        }
    }
}
