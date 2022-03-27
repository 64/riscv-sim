use std::collections::HashMap;

use aca::{cpu::Cpu, inst::ArchReg, mem::MainMemory, out_of_order::OutOfOrder, parse_and_exec};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

type CpuType = OutOfOrder;

fn is_prime<C: Cpu>(x: u32) -> bool {
    parse_and_exec::<C>(
        "prime",
        HashMap::from([(ArchReg::A0, x)]),
        MainMemory::new(),
    )
    .regs
    .get(ArchReg::A0)
        == 1
}

fn primes_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("primes_large");
    group.sample_size(10);
    group.bench_function("prime 2946901", |b| {
        b.iter(|| is_prime::<CpuType>(black_box(2946901)))
    });
    group.finish();
}

criterion_group!(benches, primes_large);
criterion_main!(benches);
