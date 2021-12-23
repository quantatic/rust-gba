mod apu;
mod bit_manipulation;
mod bus;
mod cpu;
mod lcd;

#[cfg(target_pointer_width = "16")]
compile_error!("architecture with pointer size >= 32 required");

fn main() {
    let mut cpu = cpu::Cpu::default();

    for _ in 0..74_500_000 {
        cpu.fetch_decode_execute(false);
    }

    loop {
        cpu.fetch_decode_execute(true);
    }
}
