use std::cell::Cell;

use lazy_static::lazy_static;
use regex::bytes::Regex;

use crate::bit_manipulation::BitManipulation;

lazy_static! {
    static ref EEPROM_PATTERN: Regex = Regex::new(r"EEPROM_V\w\w\w").unwrap();
    static ref SRAM_PATTERN: Regex = Regex::new(r"SRAM_V\w\w\w").unwrap();
    static ref FLASH_64KB_PATTERN: Regex = Regex::new(r"FLASH_V\w\w\w|FLASH512_V\w\w\w").unwrap();
    static ref FLASH_128KB_PATTERN: Regex = Regex::new(r"FLASH1M_V\w\w\w").unwrap();
}

pub struct Cartridge {
    rom: Vec<u8>,
    eeprom: Option<Eeprom>,
}

impl Cartridge {
    pub fn new(data: &[u8]) -> Self {
        let eeprom_match = EEPROM_PATTERN.is_match(data);
        let sram_match = SRAM_PATTERN.is_match(data);
        let flash64kb_match = FLASH_64KB_PATTERN.is_match(data);
        let flash128kb_match = FLASH_128KB_PATTERN.is_match(data);

        if eeprom_match {
            println!("EEPROM detected");
        }

        if sram_match {
            println!("SRAM detected")
        }

        if flash64kb_match {
            println!("flash64kb detected")
        }

        if flash128kb_match {
            println!("flash128kb detected")
        }

        let rom = data.to_vec();
        let eeprom = eeprom_match.then(Eeprom::default);

        Self { rom, eeprom }
    }

    pub fn read_rom_byte(&self, offset: u32) -> u8 {
        let offset = offset as usize;
        if offset < self.rom.len() {
            self.rom[offset as usize]
        } else {
            println!("OUT OF BOUNDS READ CARTRIDGE OFFSET: {:08X}", offset);
            0
        }
    }

    pub fn read_rom_hword(&mut self, offset: u32) -> u16 {
        match &mut self.eeprom {
            Some(eeprom) if offset > 0x1FFFF00 || (offset as usize) >= self.rom.len() => {
                eeprom.read_hword()
            }
            _ => {
                let low_byte = self.read_rom_byte(offset);
                let high_byte = self.read_rom_byte(offset + 1);

                u16::from_le_bytes([low_byte, high_byte])
            }
        }
    }

    pub fn read_rom_word(&self, offset: u32) -> u32 {
        let le_bytes = [
            self.read_rom_byte(offset),
            self.read_rom_byte(offset + 1),
            self.read_rom_byte(offset + 2),
            self.read_rom_byte(offset + 3),
        ];

        u32::from_le_bytes(le_bytes)
    }

    pub fn write_rom_byte(&mut self, value: u8, offset: u32) {
        todo!()
    }

    pub fn write_rom_hword(&mut self, value: u16, offset: u32) {
        if let Some(eeprom) = &mut self.eeprom {
            if offset > 0x1FFFF00 || (offset as usize) >= self.rom.len() {
                eeprom.write_hword(value);
            }
        }
    }

    pub fn write_rom_word(&mut self, value: u32, offset: u32) {
        todo!()
    }

    pub fn read_sram_byte(&self, offset: u32) -> u8 {
        if offset == 0 {
            0x62
        } else if offset == 1 {
            0x13
        } else {
            0x00
        }
    }

    pub fn read_sram_hword(&self, offset: u32) -> u16 {
        let low_byte = self.read_sram_byte(offset);
        let high_byte = self.read_sram_byte(offset + 1);

        u16::from_le_bytes([low_byte, high_byte])
    }

    pub fn read_sram_word(&self, offset: u32) -> u32 {
        let le_bytes = [
            self.read_sram_byte(offset),
            self.read_sram_byte(offset + 1),
            self.read_sram_byte(offset + 2),
            self.read_sram_byte(offset + 3),
        ];

        u32::from_le_bytes(le_bytes)
    }

    pub fn write_sram_byte(&mut self, value: u8, offset: u32) {
        todo!();
    }

    pub fn write_sram_hword(&mut self, value: u16, offset: u32) {
        todo!()
    }

    pub fn write_sram_word(&mut self, value: u32, offset: u32) {
        todo!()
    }
}

#[derive(Debug)]
enum EepromAction {
    SetReadAddress,
    Write,
}

#[derive(Debug)]
enum EepromStatus {
    ReceivingCommand,
    OngoingAction(EepromAction),
    StopBit,
}

struct Eeprom {
    data: [bool; 0x10000],
    rx_bits: u8,
    rx_buffer: u64,
    rx_offset: u16,
    tx_bits: u8,
    tx_offset: u16,
    status: EepromStatus,
}

impl Default for Eeprom {
    fn default() -> Self {
        Self {
            data: [true; 0x10000],
            rx_bits: 0,
            rx_buffer: 0,
            rx_offset: 0,
            tx_bits: 0,
            tx_offset: 0,
            status: EepromStatus::ReceivingCommand,
        }
    }
}

impl Eeprom {
    fn write_hword(&mut self, value: u16) {
        const SET_CHUNK_REQUEST: u64 = 0b11;
        const WRITE_REQUEST: u64 = 0b10;

        let bit = value.get_bit(0);
        self.rx_bits += 1;
        self.rx_buffer = (self.rx_buffer << 1) | if bit { 0b1 } else { 0b0 };

        match self.status {
            EepromStatus::ReceivingCommand => {
                assert!(self.rx_bits <= 2);
                if self.rx_bits == 2 {
                    self.status = match self.rx_buffer {
                        SET_CHUNK_REQUEST => {
                            EepromStatus::OngoingAction(EepromAction::SetReadAddress)
                        }
                        WRITE_REQUEST => EepromStatus::OngoingAction(EepromAction::Write),
                        _ => todo!("{:064b}", self.rx_buffer),
                    };

                    self.rx_bits = 0;
                    self.rx_buffer = 0;
                }
            }
            EepromStatus::OngoingAction(EepromAction::SetReadAddress) => {
                assert!(self.rx_bits <= 14);
                if self.rx_bits == 14 {
                    self.tx_offset = (self.rx_buffer as u16) * 64;
                    self.tx_bits = 0;

                    self.status = EepromStatus::StopBit;
                    self.rx_bits = 0;
                    self.rx_buffer = 0;
                }
            }
            EepromStatus::OngoingAction(EepromAction::Write) => {
                assert!(self.rx_bits <= 78);

                if self.rx_bits == 14 {
                    self.rx_offset = (self.rx_buffer as u16) * 64;
                    self.rx_buffer = 0;
                } else if self.rx_bits > 14 {
                    self.data[usize::from(self.rx_offset)] = bit;
                    self.rx_offset += 1;
                }

                if self.rx_bits == 78 {
                    self.rx_bits = 0;
                    self.rx_buffer = 0;
                    self.status = EepromStatus::StopBit;
                }
            }
            EepromStatus::StopBit => {
                assert!(self.rx_bits <= 1);

                if self.rx_bits == 1 {
                    if self.rx_buffer != 0b0 {
                        println!("awaiting set address stop bit got invalid stop bit");
                    }
                }

                self.rx_bits = 0;
                self.rx_buffer = 0;
                self.status = EepromStatus::ReceivingCommand;
            }
        }
    }

    fn read_hword(&mut self) -> u16 {
        if self.tx_bits < 4 {
            self.tx_bits += 1;
            0
        } else if self.tx_bits < 68 {
            let result_bit = self.data[usize::from(self.tx_offset)];

            self.tx_offset = self.tx_offset.wrapping_add(1);
            self.tx_bits += 1;

            if result_bit {
                1
            } else {
                0
            }
        } else {
            1
        }
    }
}
