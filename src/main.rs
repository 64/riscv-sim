mod cpu;
mod inst;
mod program;

use crate::cpu::Memory;
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

    let cpu = cpu::Cpu::new(prog, HashMap::new(), Memory::new()).exec_all();
    dbg!(&cpu);
    println!("Finished executing in {} cycles.", cpu.cycles);
}
