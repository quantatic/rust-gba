mod backup_types;

use anyhow::anyhow;
use backup_types::{BackupType, BACKUP_TYPES_MAP};
use serde_with::serde_as;

use std::{io::Read, ops::Range};

use lazy_static::lazy_static;
use regex::bytes::Regex;

use crate::{bit_manipulation::BitManipulation, data_access::DataAccess};
use serde::{Deserialize, Serialize};

use anyhow::Result;

lazy_static! {
    static ref EEPROM_PATTERN: Regex = Regex::new(r"EEPROM_V\w\w\w").unwrap();
    static ref SRAM_PATTERN: Regex = Regex::new(r"SRAM_V\w\w\w").unwrap();
    static ref FLASH_64KB_PATTERN: Regex = Regex::new(r"FLASH_V\w\w\w|FLASH512_V\w\w\w").unwrap();
    static ref FLASH_128KB_PATTERN: Regex = Regex::new(r"FLASH1M_V\w\w\w").unwrap();
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Backup {
    Eeprom(Eeprom),
    Flash(Flash),
    Sram(Sram),
    None,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct Cartridge {
    rom: Vec<u8>,
    backup: Backup,
}

impl Cartridge {
    pub fn new<T: Read>(mut input: T, existing_backup: Option<Backup>) -> Result<Self> {
        let mut data = Vec::new();
        input
            .read_to_end(&mut data)
            .expect("failed to read cartridge input data");

        const GAME_TITLE_BYTE_RANGE: Range<usize> = 0x0A0..0x0AC;
        const GAME_CODE_BYTE_RANGE: Range<usize> = 0x0AC..0x0B0;

        if let Some(title_bytes) = data.get(GAME_TITLE_BYTE_RANGE) {
            let title: String = title_bytes
                .iter()
                .copied()
                .take_while(|val| *val != 0)
                .map(char::from)
                .collect();
            log::info!("{}", title);
        }

        if let Some(code_bytes) = data.get(GAME_CODE_BYTE_RANGE) {
            let code: String = code_bytes
                .iter()
                .copied()
                .take_while(|val| *val != 0)
                .map(char::from)
                .collect();
            log::info!("{}", code);

            if let Some(backup_type) = BACKUP_TYPES_MAP.get(code_bytes) {
                log::info!("{:?}", backup_type);
            }
        }

        let new_backup = {
            let code_bytes = &data[GAME_CODE_BYTE_RANGE];

            match backup_types::BACKUP_TYPES_MAP.get(code_bytes).copied() {
                Some(BackupType::Eeprom512B) => todo!(),
                Some(BackupType::Eeprom8K) => Backup::Eeprom(Eeprom::default()),
                Some(BackupType::Flash {
                    device_type,
                    manufacturer,
                }) => Backup::Flash(Flash::new(device_type, manufacturer)),
                Some(BackupType::Sram) => Backup::Sram(Sram::default()),
                None | Some(BackupType::None) => {
                    log::warn!("falling back to ROM string search for backup detection");
                    let eeprom_match = EEPROM_PATTERN.is_match(&data);
                    let sram_match = SRAM_PATTERN.is_match(&data);
                    let flash64kb_match = FLASH_64KB_PATTERN.is_match(&data);
                    let flash128kb_match = FLASH_128KB_PATTERN.is_match(&data);

                    let num_matches = [eeprom_match, sram_match, flash64kb_match, flash128kb_match]
                        .into_iter()
                        .filter(|val| *val)
                        .count();
                    assert!(num_matches <= 1);

                    if eeprom_match {
                        Backup::Eeprom(Eeprom::default())
                    } else if sram_match {
                        Backup::Sram(Sram::default())
                    } else if flash64kb_match || flash128kb_match {
                        Backup::Flash(Flash::default())
                    } else {
                        Backup::None
                    }
                }
            }
        };

        let rom = data;

        let backup = if let Some(existing_backup) = existing_backup {
            let new_backup_discriminant = std::mem::discriminant(&new_backup);
            let existing_backup_discriminant = std::mem::discriminant(&existing_backup);

            if new_backup_discriminant != existing_backup_discriminant {
                return Err(anyhow!(
                    "expected existing backup to match detected backup type {:?}, but got {:?}",
                    new_backup_discriminant,
                    existing_backup_discriminant
                ));
            }

            existing_backup
        } else {
            new_backup
        };

        Ok(Self { rom, backup })
    }

    pub fn get_backup(&self) -> &Backup {
        &self.backup
    }

    pub fn set_backup(&mut self, backup: Backup) -> Result<()> {
        let current_variant = std::mem::discriminant(&self.backup);
        let new_variant = std::mem::discriminant(&backup);
        if current_variant != new_variant {
            return Err(anyhow!(
                "expected to get backup with existing variant {:?}, but got {:?}",
                current_variant,
                new_variant
            ));
        }

        self.backup = backup;
        Ok(())
    }
}

impl Cartridge {
    pub fn read_rom_byte(&self, offset: u32) -> u8 {
        if offset < (self.rom.len() as u32) {
            self.rom[offset as usize]
        } else {
            // Reading from GamePak ROM when no Cartridge is inserted
            //
            // Because Gamepak uses the same signal-lines for both 16bit data and for lower 16bit halfword address,
            // the entire gamepak ROM area is effectively filled by incrementing 16bit values (Address/2 AND FFFFh).
            let hword_value = (offset / 2) & 0xFFFF;
            hword_value.get_data(offset & 0b1)
        }
    }

    pub fn read_rom_hword(&mut self, offset: u32) -> u16 {
        match &mut self.backup {
            Backup::Eeprom(eeprom) if offset > 0x1FFFF00 || (offset as usize) >= self.rom.len() => {
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

    pub fn write_rom_byte(&mut self, _value: u8, _offset: u32) {
        // ROM byte writes ignored
    }

    pub fn write_rom_hword(&mut self, value: u16, offset: u32) {
        match &mut self.backup {
            Backup::Eeprom(eeprom) if offset > 0x1FFFF00 || (offset as usize) >= self.rom.len() => {
                eeprom.write_hword(value);
            }
            _ => {} // ignore all other ROM hword writes
        }
    }

    pub fn write_rom_word(&mut self, _value: u32, _offset: u32) {
        // ROM word writes ignored
    }

    pub fn read_sram_byte(&self, offset: u32) -> u8 {
        match &self.backup {
            Backup::Flash(flash) => flash.read_byte(offset),
            Backup::Sram(sram) => sram.read_byte(offset),
            _ => todo!(),
        }
    }

    pub fn write_sram_byte(&mut self, value: u8, offset: u32) {
        match &mut self.backup {
            Backup::Flash(flash) => flash.write_byte(value, offset),
            Backup::Sram(sram) => sram.write_byte(value, offset),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum EepromAction {
    SetReadAddress,
    Write,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum EepromStatus {
    ReceivingCommand,
    OngoingAction(EepromAction),
    StopBit,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Eeprom {
    #[serde_as(as = "Box<[_; 0x10000]>")]
    data: Box<[bool; 0x10000]>,
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
            data: Box::new([true; 0x10000]),
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
        self.rx_buffer = (self.rx_buffer << 1) | (bit as u64);

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
                        log::warn!("awaiting set address stop bit got invalid stop bit");
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

            result_bit as u16
        } else {
            1
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FlashCommandState {
    ReadCommand,
    BankSwitch,
    Identification,
    Erase,
    WriteSingleByte,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum FlashWantedWrite {
    Write_5555_AA,
    Write_2AAA_55,
    CommandData,
}

// Atmel flash chips are not handled.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flash {
    #[serde_as(as = "Box<[_; 0x10000]>")]
    low_bank: Box<[u8; 0x10000]>,
    #[serde_as(as = "Box<[_; 0x10000]>")]
    high_bank: Box<[u8; 0x10000]>,
    device_type: u8,
    manufacturer: u8,
    state: FlashCommandState,
    wanted_write: FlashWantedWrite,
    use_high_bank: bool,
}

impl Default for Flash {
    fn default() -> Self {
        Self::new(Self::DEFAULT_DEVICE_TYPE, Self::DEFAULT_MANUFACTURER)
    }
}

impl Flash {
    const DEFAULT_DEVICE_TYPE: u8 = 0xD4;
    const DEFAULT_MANUFACTURER: u8 = 0xBF;

    const ATMEL_DEVICE_TYPE: u8 = 0x3D;
    const ATMEL_MANUFACTURER: u8 = 0x1F;

    fn new(device_type: u8, manufacturer: u8) -> Self {
        assert!(device_type != Self::ATMEL_DEVICE_TYPE);
        assert!(manufacturer != Self::ATMEL_MANUFACTURER);

        Self {
            low_bank: Box::new([0xFF; 0x10000]),
            high_bank: Box::new([0xFF; 0x10000]),
            device_type,
            manufacturer,
            state: FlashCommandState::ReadCommand,
            wanted_write: FlashWantedWrite::Write_5555_AA,
            use_high_bank: false,
        }
    }

    fn read_byte(&self, offset: u32) -> u8 {
        match self.state {
            FlashCommandState::Identification if offset == 0x0000 => self.manufacturer,
            FlashCommandState::Identification if offset == 0x0001 => self.device_type,
            _ => {
                if self.use_high_bank {
                    self.high_bank[offset as usize]
                } else {
                    self.low_bank[offset as usize]
                }
            }
        }
    }

    fn write_byte(&mut self, value: u8, offset: u32) {
        match self.wanted_write {
            FlashWantedWrite::Write_5555_AA if offset == 0x5555 && value == 0xAA => {
                self.wanted_write = FlashWantedWrite::Write_2AAA_55;
            }
            FlashWantedWrite::Write_2AAA_55 if offset == 0x2AAA && value == 0x55 => {
                self.wanted_write = FlashWantedWrite::CommandData;
            }
            FlashWantedWrite::Write_5555_AA if offset == 0x5555 && value == 0xF0 => {
                log::warn!("Macronix force end of command");
                self.state = FlashCommandState::ReadCommand;
                self.wanted_write = FlashWantedWrite::Write_5555_AA;
            }
            FlashWantedWrite::CommandData => match self.state {
                FlashCommandState::ReadCommand if offset == 0x5555 => match value {
                    0x80 => {
                        self.state = FlashCommandState::Erase;
                        self.wanted_write = FlashWantedWrite::Write_5555_AA;
                    }
                    0x90 => {
                        self.state = FlashCommandState::Identification;
                        self.wanted_write = FlashWantedWrite::Write_5555_AA;
                    }
                    0xA0 => {
                        self.state = FlashCommandState::WriteSingleByte;
                        self.wanted_write = FlashWantedWrite::CommandData;
                    }
                    0xB0 => {
                        self.state = FlashCommandState::BankSwitch;
                        self.wanted_write = FlashWantedWrite::CommandData;
                    }
                    _ => unreachable!(),
                },
                FlashCommandState::Identification if offset == 0x5555 && value == 0xF0 => {
                    self.state = FlashCommandState::ReadCommand;
                    self.wanted_write = FlashWantedWrite::Write_5555_AA;
                }
                FlashCommandState::BankSwitch if offset == 0x0000 => {
                    self.use_high_bank = value != 0;
                    self.state = FlashCommandState::ReadCommand;
                    self.wanted_write = FlashWantedWrite::Write_5555_AA;
                }
                FlashCommandState::WriteSingleByte => {
                    if self.use_high_bank {
                        self.high_bank[offset as usize] = value;
                    } else {
                        self.low_bank[offset as usize] = value;
                    }

                    self.state = FlashCommandState::ReadCommand;
                    self.wanted_write = FlashWantedWrite::Write_5555_AA;
                }
                FlashCommandState::Erase => {
                    match value {
                        0x10 if offset == 0x5555 => {
                            for val in self.low_bank.iter_mut() {
                                *val = 0xFF;
                            }

                            for val in self.high_bank.iter_mut() {
                                *val = 0xFF;
                            }
                        }
                        0x30 => {
                            assert!(offset % 0x1000 == 0);
                            for erase_offset in 0..0x1000 {
                                if self.use_high_bank {
                                    self.high_bank[(offset + erase_offset) as usize] = 0xFF;
                                } else {
                                    self.low_bank[(offset + erase_offset) as usize] = 0xFF;
                                }
                            }
                        }
                        _ => unreachable!("erase command {:02X}", value),
                    }

                    self.state = FlashCommandState::ReadCommand;
                    self.wanted_write = FlashWantedWrite::Write_5555_AA;
                }
                _ => unreachable!(
                    "{:02X} {:08X} {:?} {:?}",
                    value, offset, self.state, self.wanted_write
                ),
            },
            _ => todo!(
                "{:02X} {:08X} {:?} {:?}",
                value,
                offset,
                self.state,
                self.wanted_write
            ),
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sram {
    #[serde_as(as = "Box<[_; 0x10000]>")]
    data: Box<[u8; 0x10000]>,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct Foo {
    #[serde_as(as = "Box<[_; 0x1000]>")]
    vals: Box<[u128; 0x1000]>,
}

impl Default for Sram {
    fn default() -> Self {
        Self {
            data: Box::new([0; 0x10000]),
        }
    }
}

impl Sram {
    fn read_byte(&self, offset: u32) -> u8 {
        self.data[offset as usize]
    }

    fn write_byte(&mut self, value: u8, offset: u32) {
        self.data[offset as usize] = value;
    }
}
