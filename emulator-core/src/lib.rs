mod apu;
mod bit_manipulation;
mod bus;
mod cartridge;
mod cpu;
mod data_access;
mod keypad;
mod lcd;
mod timer;

use bit_manipulation::BitManipulation;
use data_access::DataAccess;

pub use cartridge::Cartridge;
pub use cpu::Cpu;
pub use keypad::Key;
pub use lcd::Lcd;
pub const CYCLES_PER_SECOND: u64 = 16_777_216;

pub fn calculate_lcd_checksum(cpu: &Cpu) -> u64 {
    use std::hash::Hasher;
    use xxhash_rust::xxh3::Xxh3;

    let mut hasher = Xxh3::default();

    for pixel in cpu.bus.lcd.get_buffer().iter().flatten() {
        hasher.write_u8(pixel.red);
        hasher.write_u8(pixel.green);
        hasher.write_u8(pixel.blue);
    }

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_checksum(cpu: &Cpu, checksum: u64) {
        assert_eq!(checksum, calculate_lcd_checksum(cpu));
    }

    fn press_key(cpu: &mut Cpu, key: Key) {
        // Tune this if this isn't long enough for key presses to register.
        //
        // This is currently about 100ms of cpu time (1/10 of a second).
        const KEY_PRESS_DELAY: usize = (CYCLES_PER_SECOND / 10) as usize;

        cpu.bus.keypad.set_pressed(key, true);
        for _ in 0..KEY_PRESS_DELAY {
            cpu.fetch_decode_execute_no_logs();
        }
        cpu.bus.keypad.set_pressed(key, false);
        for _ in 0..KEY_PRESS_DELAY {
            cpu.fetch_decode_execute_no_logs();
        }
    }

    macro_rules! simple_ppu_test {
        ($name:ident, $path:literal, $checksum:literal) => {
            #[test]
            fn $name() {
                let source = include_bytes!($path);
                let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
                let mut cpu = Cpu::new(cartridge);

                for _ in 0..100_000_000 {
                    cpu.fetch_decode_execute_no_logs();
                }

                assert_checksum(&cpu, $checksum);
            }
        };
    }

    simple_ppu_test!(eeprom, "../tests/eeprom_test.gba", 0x7AD21BBF19367764);
    simple_ppu_test!(flash, "../tests/flash_test.gba", 0x7AD21BBF19367764);
    simple_ppu_test!(mandelbrot, "../tests/mandelbrot.gba", 0x643CD59EBF90FAA9);
    simple_ppu_test!(memory, "../tests/memory.gba", 0x740626E6CC2D204A);
    simple_ppu_test!(swi_demo, "../tests/swi_demo.gba", 0xD55A7769AD7F9392);
    simple_ppu_test!(first, "../tests/first.gba", 0x36B520E8A096B03C);

    simple_ppu_test!(dma_demo_simple, "../tests/dma_demo.gba", 0x9BA3DB86C4D5D083);

    simple_ppu_test!(hello, "../tests/hello.gba", 0xCF2FB83F6755E1DB);

    simple_ppu_test!(m3_demo, "../tests/m3_demo.gba", 0x7F4A2DFC61FC7E34);

    simple_ppu_test!(
        armwrestler_simple,
        "../tests/armwrestler.gba",
        0x1C1579ACC537960D
    );

    simple_ppu_test!(
        gba_tests_memory,
        "../tests/gba_tests_memory.gba",
        0x740626E6CC2D204A
    );

    simple_ppu_test!(
        gba_tests_nes,
        "../tests/gba_tests_nes.gba",
        0x740626E6CC2D204A
    );

    simple_ppu_test!(
        gba_tests_arm,
        "../tests/gba_tests_arm.gba",
        0x740626E6CC2D204A
    );

    simple_ppu_test!(
        gba_tests_thumb,
        "../tests/gba_tests_thumb.gba",
        0x740626E6CC2D204A
    );

    simple_ppu_test!(
        gba_tests_hello,
        "../tests/gba_tests_hello.gba",
        0xE4167702EFF02E47
    );

    simple_ppu_test!(
        gba_tests_shades,
        "../tests/gba_tests_shades.gba",
        0x21D6D12973C70D5D
    );

    simple_ppu_test!(
        gba_tests_stripes,
        "../tests/gba_tests_stripes.gba",
        0x6E881E3A0BC09EBF
    );

    #[test]
    fn armwrestler_arm_complex() {
        const INITIAL_CHECKSUM: u64 = 0x1C1579ACC537960D;

        const ARM_ALU_PART_1: u64 = 0x53DA53FF9EF55555;
        const ARM_ALU_PART_2: u64 = 0x5987D6DD7264121C;
        const ARM_LOAD_TESTS_PART_1: u64 = 0x127F4528A024A777;
        const ARM_LOAD_TESTS_PART_2: u64 = 0x7569D8F3583A88BD;
        const ARM_LDM_STM_TESTS_1: u64 = 0x2F4688257C51FD03;

        let source = include_bytes!("../tests/armwrestler.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        for _ in 0..100_000_000 {
            cpu.fetch_decode_execute_no_logs();
        }

        assert_checksum(&cpu, INITIAL_CHECKSUM);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, ARM_ALU_PART_1);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, ARM_ALU_PART_2);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, ARM_LOAD_TESTS_PART_1);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, ARM_LOAD_TESTS_PART_2);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, ARM_LDM_STM_TESTS_1);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, INITIAL_CHECKSUM);
    }

    #[test]
    fn armwrestler_thumb_complex() {
        const INITIAL_CHECKSUM: u64 = 0x1C1579ACC537960D;

        const THUMB_TESTS_SELECTED_CHECKSUM: u64 = 0xF36B500E2D90CD52;

        const THUMB_ALU_TEST: u64 = 0x55D652A9DAE7F2A0;
        const THUMB_LDR_STR_TEST: u64 = 0xF4F5CBE6217EF9F0;
        const THUMB_LDM_STM_TEST: u64 = 0xDED0DBE7F075848E;

        let source = include_bytes!("../tests/armwrestler.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        for _ in 0..100_000_000 {
            cpu.fetch_decode_execute_no_logs();
        }

        assert_checksum(&cpu, INITIAL_CHECKSUM);

        // Scroll down to Thumb tests.
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        assert_checksum(&cpu, THUMB_TESTS_SELECTED_CHECKSUM);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, THUMB_ALU_TEST);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, THUMB_LDR_STR_TEST);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, THUMB_LDM_STM_TEST);

        press_key(&mut cpu, Key::Start);
        assert_checksum(&cpu, THUMB_TESTS_SELECTED_CHECKSUM);

        // Scroll back up to initial state.
        press_key(&mut cpu, Key::Up);
        press_key(&mut cpu, Key::Up);
        press_key(&mut cpu, Key::Up);
        assert_checksum(&cpu, INITIAL_CHECKSUM);
    }

    #[test]
    fn suite_shifter() {
        const INITIAL_CHECKSUM: u64 = 0x3B32CCEB3BAE455B;
        const SHIFTER_TEST_SELECTED_CHECKSUM: u64 = 0x44BFA86E38A2027E;
        const SHIFTER_SUCCESS_SCREEN_CHECKSUM: u64 = 0xF82D049DDEF321AC;

        let source = include_bytes!("../tests/suite.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        for _ in 0..100_000_000 {
            cpu.fetch_decode_execute_no_logs();
        }

        assert_checksum(&cpu, INITIAL_CHECKSUM);

        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);

        assert_checksum(&cpu, SHIFTER_TEST_SELECTED_CHECKSUM);

        press_key(&mut cpu, Key::A);

        assert_checksum(&cpu, SHIFTER_SUCCESS_SCREEN_CHECKSUM);
    }

    #[test]
    fn suite_carry() {
        const INITIAL_CHECKSUM: u64 = 0x3B32CCEB3BAE455B;
        const CARRY_TEST_SELECTED_CHECKSUM: u64 = 0x584DECF1B2656938;
        const CARRY_SUCCESS_SCREEN_CHECKSUM: u64 = 0x89F7F1CFD8DC70E3;

        let source = include_bytes!("../tests/suite.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        for _ in 0..100_000_000 {
            cpu.fetch_decode_execute_no_logs();
        }

        assert_checksum(&cpu, INITIAL_CHECKSUM);

        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);

        assert_checksum(&cpu, CARRY_TEST_SELECTED_CHECKSUM);

        press_key(&mut cpu, Key::A);

        assert_checksum(&cpu, CARRY_SUCCESS_SCREEN_CHECKSUM);
    }

    #[test]
    fn suite_bios_math() {
        const INITIAL_CHECKSUM: u64 = 0x3B32CCEB3BAE455B;
        const BIOS_MATH_TEST_SELECTED_CHECKSUM: u64 = 0x2950FA409FCAF1D2;
        const BIOS_MATH_SUCCESS_SCREEN_CHECKSUM: u64 = 0x43AD9E744E911293;

        let source = include_bytes!("../tests/suite.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        for _ in 0..100_000_000 {
            cpu.fetch_decode_execute_no_logs();
        }

        assert_checksum(&cpu, INITIAL_CHECKSUM);

        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);
        press_key(&mut cpu, Key::Down);

        assert_checksum(&cpu, BIOS_MATH_TEST_SELECTED_CHECKSUM);

        press_key(&mut cpu, Key::A);

        assert_checksum(&cpu, BIOS_MATH_SUCCESS_SCREEN_CHECKSUM);
    }
}
