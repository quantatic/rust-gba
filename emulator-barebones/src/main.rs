use std::{fs::File, time::Instant};

use anyhow::{anyhow, Result};
use clap::Parser;

use emulator_core::{calculate_lcd_checksum, Cartridge, Cpu, Key, Lcd, CYCLES_PER_SECOND};

#[derive(Debug, Parser)]
struct Args {
    rom: String,

    #[clap(short, long)]
    frames: Option<u64>,
}

#[allow(unused)]
fn press_key(cpu: &mut Cpu, key: Key) {
    cpu.bus.keypad.set_pressed(key, true);
    for _ in 0..500_000 {
        cpu.fetch_decode_execute();
    }
    cpu.bus.keypad.set_pressed(key, false);
    for _ in 0..500_000 {
        cpu.fetch_decode_execute();
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let save_file_name = format!("{}.sav", args.rom);

    let rom_file =
        File::open(&args.rom).map_err(|_| anyhow!("failed to open ROM file \"{}\"", args.rom))?;

    let save_file = File::open(&save_file_name).ok();

    println!("initializing cart");
    let cartridge = Cartridge::new(rom_file, None)?;
    println!("cart initialized");
    let mut cpu = Cpu::new(cartridge);

    let init = Instant::now();
    let mut last_step = Instant::now();
    let mut i = 0;

    loop {
        cpu.fetch_decode_execute();
    }
}
