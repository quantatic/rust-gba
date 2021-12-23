use crate::apu::Apu;
use crate::bit_manipulation::BitManipulation;
use crate::lcd::Lcd;

const BIOS: &[u8] = include_bytes!("../gba_bios.bin");
const ROM: &[u8] = include_bytes!("../arm.gba");

pub trait DataAccess<DataAccessType> {
    fn set_data(self, value: DataAccessType, index: u32) -> Self;

    fn get_data(self, index: u32) -> DataAccessType;
}

impl<T> DataAccess<T> for T {
    fn set_data(self, value: T, index: u32) -> Self {
        assert!(index == 0);

        value
    }

    fn get_data(self, index: u32) -> Self {
        assert!(index == 0);

        self
    }
}

impl DataAccess<u8> for u16 {
    fn set_data(self, value: u8, index: u32) -> Self {
        assert!(index < 2);

        let shift = 8 * (index & 0b1);
        (self & (!(0xFF << shift))) | (u16::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u8 {
        assert!(index < 2);

        let shift = 8 * (index & 0b1);
        (self >> shift) as u8
    }
}

impl DataAccess<u8> for u32 {
    fn set_data(self, value: u8, index: u32) -> Self {
        assert!(index < 4);

        let shift = 8 * (index & 0b11);
        (self & (!(0xFF << shift))) | (u32::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u8 {
        assert!(index < 4);

        let shift = 8 * (index & 0b11);
        (self >> shift) as u8
    }
}

impl DataAccess<u16> for u32 {
    fn set_data(self, value: u16, index: u32) -> Self {
        assert!(index < 2);

        let shift = 16 * (index & 0b1);
        (self & (!(0xFF_FF << shift))) | (u32::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u16 {
        assert!(index < 2);

        let shift = 16 * (index & 0b1);
        (self >> shift) as u16
    }
}

impl DataAccess<u32> for u64 {
    fn set_data(self, value: u32, index: u32) -> Self {
        assert!(index < 2);

        let shift = 32 * (index & 0b1);
        (self & (!(0xFF_FF_FF_FF << shift))) | (u64::from(value) << shift)
    }

    fn get_data(self, index: u32) -> u32 {
        assert!(index < 2);

        let shift = 32 * (index & 0b1);
        (self >> shift) as u32
    }
}

#[derive(Debug)]
pub struct Bus {
    chip_wram: Box<[u8; 0x8000]>,
    pub board_wram: Box<[u8; 0x40000]>,
    cycle_count: usize,
    interrupt_master_enable: u16,
    interrupt_enable: u16,
    interrupt_request: u16,
    lcd: Lcd,
    apu: Apu,
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
            lcd: Lcd::default(),
            apu: Apu::default(),
        }
    }
}

impl Bus {
    pub fn step(&mut self) {
        if self.cycle_count % 4 == 0 {
            self.lcd.step();
        }

        self.poll_lcd_interrupts();

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

    const LCD_STATUS_BASE: u32 = 0x04000004;
    const LCD_STATUS_END: u32 = Self::LCD_STATUS_BASE + 1;

    const LCD_VERTICAL_COUNTER_BASE: u32 = 0x04000006;
    const LCD_VERTICAL_COUNTER_END: u32 = Self::LCD_VERTICAL_COUNTER_BASE + 1;

    const SOUND_PWM_CONTROL_BASE: u32 = 0x04000088;
    const SOUND_PWM_CONTROL_END: u32 = Self::SOUND_PWM_CONTROL_BASE + 1;

    const KEY_STATUS_BASE: u32 = 0x04000130;
    const KEY_STATUS_END: u32 = Self::KEY_STATUS_BASE + 1;

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
            Self::LCD_STATUS_BASE..=Self::LCD_STATUS_END => self.lcd.read_lcd_status(address & 0b1),
            Self::LCD_VERTICAL_COUNTER_BASE..=Self::LCD_VERTICAL_COUNTER_END => {
                self.lcd.read_vcount(address & 0b1)
            }
            Self::SOUND_PWM_CONTROL_BASE..=Self::SOUND_PWM_CONTROL_END => {
                self.apu.read_sound_bias(address & 0b1)
            }
            Self::KEY_STATUS_BASE..=Self::KEY_STATUS_END => {
                // println!("reading from stubbed KEY_STATUS");
                0xFF
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
            _ => todo!("byte read 0x{:08x}", address),
        }
    }

    pub fn read_halfword_address(&self, address: u32) -> u16 {
        assert!(address & 0b1 == 0b0);

        let low_byte = self.read_byte_address(address);
        let high_byte = self.read_byte_address(address + 1);

        u16::from_le_bytes([low_byte, high_byte])
    }

    pub fn read_word_address(&self, address: u32) -> u32 {
        assert!(address & 0b11 == 0b00);

        let low_halfword = self.read_halfword_address(address);
        let high_halfword = self.read_halfword_address(address + 2);
        u32::from(low_halfword) | (u32::from(high_halfword) << 16)
    }

    pub fn write_byte_address(&mut self, value: u8, address: u32) {
        match address % Self::MEMORY_SIZE {
            0x00000000..=0x00003FFF => unreachable!("BIOS write"),
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
            Self::LCD_STATUS_BASE..=Self::LCD_STATUS_END => {
                self.lcd.write_lcd_status(value, address & 0b1)
            }
            Self::LCD_VERTICAL_COUNTER_BASE..=Self::LCD_VERTICAL_COUNTER_END => {}
            Self::SOUND_PWM_CONTROL_BASE..=Self::SOUND_PWM_CONTROL_END => {
                self.apu.write_sound_bias(value, address & 0b1)
            }
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
        assert!(address & 0b1 == 0b0);
        let low_byte = value.get_data(0);
        let high_byte = value.get_data(1);

        self.write_byte_address(low_byte, address);
        self.write_byte_address(high_byte, address + 1);
    }

    pub fn write_word_address(&mut self, value: u32, address: u32) {
        assert!(address & 0b11 == 0b00);

        let low_halfword = value as u16;
        let high_halfword = (value >> 16) as u16;

        self.write_halfword_address(low_halfword, address);
        self.write_halfword_address(high_halfword, address + 2);
    }
}

impl Bus {
    fn read_interrupt_enable<DataAccessType>(&self, index: u32) -> DataAccessType
    where
        u16: DataAccess<DataAccessType>,
    {
        self.interrupt_enable.get_data(index)
    }

    fn write_interrupt_enable<DataAccessType>(&mut self, value: DataAccessType, index: u32)
    where
        u16: DataAccess<DataAccessType>,
    {
        self.interrupt_enable = self.interrupt_enable.set_data(value, index);
    }

    fn read_interrupt_master_enable<DataAccessType>(&self, index: u32) -> DataAccessType
    where
        u16: DataAccess<DataAccessType>,
    {
        self.interrupt_master_enable.get_data(index)
    }

    fn write_interrupt_master_enable<DataAccessType>(&mut self, value: DataAccessType, index: u32)
    where
        u16: DataAccess<DataAccessType>,
    {
        self.interrupt_master_enable = self.interrupt_master_enable.set_data(value, index);
    }

    fn read_interrupt_request<DataAccessType>(&self, index: u32) -> DataAccessType
    where
        u16: DataAccess<DataAccessType>,
    {
        self.interrupt_request.get_data(index)
    }

    fn write_interrupt_acknowledge<DataAccessType>(&mut self, value: DataAccessType, index: u32)
    where
        u16: DataAccess<DataAccessType>,
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

    fn get_interrupts_enabled(&self) -> bool {
        const INTERRUPT_MASTER_ENABLE_BIT_INDEX: usize = 0;
        self.interrupt_master_enable
            .get_bit(INTERRUPT_MASTER_ENABLE_BIT_INDEX)
    }

    fn poll_lcd_interrupts(&mut self) {
        let lcd_interrupts = self.lcd.poll_pending_interrupts();
        self.lcd.clear_pending_interrupts();

        if lcd_interrupts.vblank {
            self.interrupt_request = self
                .interrupt_request
                .set_bit(Self::LCD_VBLANK_INTERRUPT_BIT_INDEX, true);
        }

        if lcd_interrupts.hblank {
            self.interrupt_request = self
                .interrupt_request
                .set_bit(Self::LCD_HBLANK_INTERRUPT_BIT_INDEX, true);
        }

        if lcd_interrupts.vcount {
            self.interrupt_request = self
                .interrupt_request
                .set_bit(Self::LCD_VCOUNT_INTERRUPT_BIT_INDEX, true);
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
