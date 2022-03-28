mod branch;
mod cpu;
mod emulated;
mod execution_unit;
mod inst;
mod lsq;
mod mem;
mod out_of_order;
mod pipelined;
mod program;
mod queue;
mod regs;
mod reservation_station;
mod rob;
mod util;

use crate::{cpu::Cpu, inst::ArchReg, mem::MainMemory, regs::RegSet};
use std::time::Instant;

fn main() {
    let start = Instant::now();

    let file = std::env::args()
        .nth(1)
        .expect("required input file as argument argument");

    let contents =
        std::fs::read_to_string(&format!("asm/{}.asm", file)).expect("failed to open file");

    let prog = contents
        .parse::<program::Program>()
        .expect("failed to parse program");

    let a0 = std::env::args()
        .nth(2)
        .and_then(|x| x.parse::<u32>().ok())
        .unwrap_or(0);
    let a1 = std::env::args()
        .nth(3)
        .and_then(|x| x.parse::<u32>().ok())
        .unwrap_or(0);
    let initial_regs = RegSet::from([(ArchReg::A0, a0), (ArchReg::A1, a1)]);

    // let res = emulated::Emulated::new(prog, initial_regs, MainMemory::new()).exec_all();
    // let res = pipelined::Pipelined::new(prog, initial_regs, MainMemory::new()).exec_all();
    let res = out_of_order::OutOfOrder::new(prog, initial_regs, MainMemory::new()).exec_all();
    dbg!(&res);

    println!("    EXECUTION COMPLETED");
    println!("    =====================");
    println!("    Instructions retired: {}", res.insts_retired);
    println!("            Cycles taken: {}", res.cycles_taken);
    println!(
        "  Instructions per clock: {:.2}",
        res.insts_retired as f32 / res.cycles_taken as f32
    );
    println!(
        "  Simulator time elapsed: {:.2}s",
        start.elapsed().as_secs_f32()
    );
}
