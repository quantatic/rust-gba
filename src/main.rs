mod bit_manipulation;
mod bus;
mod cpu;

#[cfg(target_pointer_width = "16")]
compile_error!("architecture with pointer size >= 32 required");

fn main() {
    let mut cpu = cpu::Cpu::default();
    for _ in 0..5_000_000 {
        let decoded = cpu.decode();
        println!("\n{}\n", cpu);
        eprintln!("{}", decoded);
        // println!("{:#08x?}", decoded);
        cpu.execute(decoded);
        println!("\n{}\n", cpu);
        println!("-----------------------------------------");
    }
}
