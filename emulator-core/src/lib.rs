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

pub use bus::Bus;
pub use cartridge::Cartridge;
pub use cpu::Cpu;
pub use cpu::CpuMode;
pub use cpu::Instruction;
pub use cpu::InstructionSet;
pub use cpu::Register;
pub use keypad::Key;
pub use lcd::{Lcd, Rgb555};
pub const CYCLES_PER_SECOND: u64 = 16_777_216;

pub fn calculate_lcd_checksum(cpu: &Cpu) -> u64 {
    use std::hash::Hasher;
    use xxhash_rust::xxh3::Xxh3;

    let mut hasher = Xxh3::default();

    for pixel in cpu.bus.lcd.get_buffer().iter().flatten() {
        hasher.write_u8(pixel.red());
        hasher.write_u8(pixel.green());
        hasher.write_u8(pixel.blue());
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
            cpu.fetch_decode_execute();
        }
        cpu.bus.keypad.set_pressed(key, false);
        for _ in 0..KEY_PRESS_DELAY {
            cpu.fetch_decode_execute();
        }
    }

    macro_rules! simple_ppu_test {
        ($name:ident, $path:literal, $checksum:literal) => {
            #[test]
            fn $name() {
                let source = include_bytes!($path);
                let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
                let mut cpu = Cpu::new(cartridge);

                while cpu.cycle_count() < 100_000_000 {
                    cpu.fetch_decode_execute();
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

    // https://github.com/PeterLemon/GBA
    simple_ppu_test!(
        peter_obj_4bpp,
        "../tests/peter_obj_4bpp.gba",
        0xEED8117DDF639EA1
    );
    simple_ppu_test!(
        peter_obj_8bpp,
        "../tests/peter_obj_8bpp.gba",
        0xEED8117DDF639EA1
    );
    simple_ppu_test!(
        peter_bg_rot_zoom_mode_3,
        "../tests/peter_bg_rot_zoom_mode_3.gba",
        0x541BB4DE9702EBA3
    );
    simple_ppu_test!(
        peter_bg_rot_zoom_mode_4,
        "../tests/peter_bg_rot_zoom_mode_4.gba",
        0x2C965A651DE49697
    );

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
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
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
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
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
    fn suite_memory() {
        const INITIAL_CHECKSUM: u64 = 0x3B32CCEB3BAE455B;
        const MEMORY_TEST_SELECTED_CHECKSUM: u64 = 0x3B32CCEB3BAE455B;
        const MEMORY_SUCCESS_SCREEN_CHECKSUM: u64 = 0x7849B12FEBF63283;

        let source = include_bytes!("../tests/suite.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
        }

        assert_checksum(&cpu, INITIAL_CHECKSUM);

        assert_checksum(&cpu, MEMORY_TEST_SELECTED_CHECKSUM);

        press_key(&mut cpu, Key::A);

        let start_cycles = cpu.cycle_count();

        // Memory test takes a while, so wait an extra second for test to run.
        while cpu.cycle_count() - start_cycles < CYCLES_PER_SECOND {
            cpu.fetch_decode_execute();
        }

        assert_checksum(&cpu, MEMORY_SUCCESS_SCREEN_CHECKSUM);
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
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
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
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
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
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
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

    #[test]
    fn suite_dma() {
        const INITIAL_CHECKSUM: u64 = 0x3B32CCEB3BAE455B;
        const DMA_TEST_SELECTED_CHECKSUM: u64 = 0xB5E03F00EB8D896A;
        const DMA_SUCCESS_SCREEN_CHECKSUM: u64 = 0x0B05ACFFFB452786;

        let source = include_bytes!("../tests/suite.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
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
        press_key(&mut cpu, Key::Down);

        assert_checksum(&cpu, DMA_TEST_SELECTED_CHECKSUM);

        press_key(&mut cpu, Key::A);

        let start_cycles = cpu.cycle_count();

        // DMA test takes a while, so wait an extra second for test to run.
        while cpu.cycle_count() - start_cycles < CYCLES_PER_SECOND {
            cpu.fetch_decode_execute();
        }

        assert_checksum(&cpu, DMA_SUCCESS_SCREEN_CHECKSUM);
    }

    #[test]
    fn openbuster() {
        const SCREEN_CHECKSUMS: &[u64] = &[
            0x3B65E15802C43DC1, // IWRAM - LDRB
            0xD04B19A7B4641F90, // IWRAM - STRB
            0x95BBB22BA233B656, // IWRAM - LDRH
            0x2D336CF7F9C81FA7, // EWRAM - LDRB/LDRH (0)
            0x6DCC7C311AFC714B, // EWRAM - LDRB/LDRH (1)
            0x43C0582E38FE5868, // MMIO - LDRB/LDRH (0)
            0x52060C8F3830A037, // MMIO - LDRB/LDRH (1)
            0x15F16BEADE6D951F, // VRAM - LDRB/LDRH (0)
            0xF10287B271137E86, // VRAM - LDRB/LDRH (1)
        ];
        const ALL_PASSED_CHECKSUM: u64 = 0x444CF2773FFA0FBA; // passed: 144 total: 144

        let source = include_bytes!("../tests/openbuster.gba");
        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
        let mut cpu = Cpu::new(cartridge);

        // skip boot screen
        while cpu.cycle_count() < 100_000_000 {
            cpu.fetch_decode_execute();
        }

        for &screen_checksum in SCREEN_CHECKSUMS {
            assert_checksum(&cpu, screen_checksum);
            press_key(&mut cpu, Key::A);
        }

        assert_checksum(&cpu, ALL_PASSED_CHECKSUM);
    }
}
