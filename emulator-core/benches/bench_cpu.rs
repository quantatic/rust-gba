use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use emulator_core::{Cartridge, Cpu};

pub fn basic_cpu_benchmark(c: &mut Criterion) {
    let source = include_bytes!("../tests/armwrestler.gba");

    let mut group = c.benchmark_group("CPU BIOS");

    for num_steps in [1, 32, 1024, 32_768] {
        group.throughput(Throughput::Elements(num_steps));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_steps),
            source,
            |b, source| {
                b.iter_batched_ref(
                    || {
                        let cartridge = Cartridge::new(source.as_slice(), None).unwrap();
                        Cpu::new(cartridge)
                    },
                    |cpu| {
                        while cpu.cycle_count() < num_steps {
                            cpu.fetch_decode_execute_no_logs();
                        }
                    },
                    BatchSize::PerIteration,
                );
            },
        );
    }
}

criterion_group!(cpu, basic_cpu_benchmark);
criterion_main!(cpu);
