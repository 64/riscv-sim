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
mod rob;
mod util;
mod branch;

use crate::{cpu::Cpu, mem::Memory};
use std::{collections::HashMap, time::Instant};

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

    // let res = emulated::Emulated::new(prog, HashMap::new(), Memory::new()).exec_all();
    // let res = pipelined::Pipelined::new(prog, HashMap::new(), Memory::new()).exec_all();
    let res = out_of_order::OutOfOrder::new(prog, HashMap::new(), Memory::new()).exec_all();
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
