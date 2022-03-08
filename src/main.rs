mod cpu;
mod emulated;
mod inst;
mod mem;
mod pipelined;
mod program;
mod regs;
mod util;

use crate::{cpu::Cpu, mem::Memory};
use std::collections::HashMap;

fn main() {
    let file = std::env::args()
        .nth(1)
        .expect("required input file as argument argument");
    let contents =
        std::fs::read_to_string(&format!("asm/{}.asm", file)).expect("failed to open file");

    let prog = contents
        .parse::<program::Program>()
        .expect("failed to parse program");

    let res = emulated::Emulated::new(prog, HashMap::new(), Memory::new()).exec_all();
    dbg!(&res);
    println!("Finished executing in {} cycles.", res.cycles_taken);
}
