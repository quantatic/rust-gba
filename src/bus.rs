use crate::bit_manipulation::BitManipulation;

const BIOS: &[u8] = include_bytes!("gba_bios.bin");

#[derive(Debug)]
pub struct Bus {
    chip_wram: Box<[u8; 0x8000]>,
    io_registers: Box<[u8; 0x400]>,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            chip_wram: Box::new([0; 0x8000]),
            io_registers: Box::new([0; 0x400]),
        }
    }
}

impl Bus {
    pub fn read_byte_address(&self, address: u32) -> u8 {
        match address {
            0x00000000..=0x00003FFF => BIOS[address as usize],
            0x03000000..=0x03007FFF => self.chip_wram[(address - 0x03000000) as usize],
            0x03008000..=0x03FFFFFF => {
                println!("reading from unused 0x{:08x}", address);
                0
            }
            0x04000000..=0x040003FE => self.io_registers[(address - 0x04000000) as usize],
            _ => {
                println!("read 0x{:08x}", address);
                0
            }
        }
    }

    pub fn write_byte_address(&mut self, value: u8, address: u32) {
        // println!("0x{:02x} -> 0x{:08x}", value, address);
        match address {
            0x03000000..=0x03007FFF => self.chip_wram[(address - 0x03000000) as usize] = value,
            0x03008000..=0x03FFFFFF => {
                println!("writing 0x{:02x} to unused 0x{:08x}", value, address);
            }
            0x04000000..=0x040003FE => self.io_registers[(address - 0x04000000) as usize] = value,
            // _ => println!("write 0x{:02x} -> 0x{:08x}", value, address),
            _ => {}
        };
    }
}

impl Bus {
    pub fn read_halfword_address(&self, address: u32) -> u16 {
        let bytes = [
            self.read_byte_address(address.wrapping_add(0)),
            self.read_byte_address(address.wrapping_add(1)),
        ];

        u16::from_le_bytes(bytes)
    }

    pub fn write_halfword_address(&mut self, value: u16, address: u32) {
        let bytes = u16::to_le_bytes(value);
        self.write_byte_address(bytes[0], address.wrapping_add(0));
        self.write_byte_address(bytes[1], address.wrapping_add(1));
    }

    pub fn read_word_address(&self, address: u32) -> u32 {
        let bytes = [
            self.read_byte_address(address.wrapping_add(0)),
            self.read_byte_address(address.wrapping_add(1)),
            self.read_byte_address(address.wrapping_add(2)),
            self.read_byte_address(address.wrapping_add(3)),
        ];

        u32::from_le_bytes(bytes)
    }

    pub fn write_word_address(&mut self, value: u32, address: u32) {
        let bytes = u32::to_le_bytes(value);
        self.write_byte_address(bytes[0], address.wrapping_add(0));
        self.write_byte_address(bytes[1], address.wrapping_add(1));
        self.write_byte_address(bytes[2], address.wrapping_add(2));
        self.write_byte_address(bytes[3], address.wrapping_add(3));
    }
}
