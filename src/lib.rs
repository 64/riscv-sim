use std::collections::HashMap;

use cpu::{Cpu, ExecResult};
use inst::ArchReg;
use mem::Memory;
use program::Program;

pub mod branch;
pub mod cpu;
pub mod emulated;
pub mod execution_unit;
pub mod inst;
pub mod lsq;
pub mod mem;
pub mod out_of_order;
pub mod pipelined;
pub mod program;
pub mod queue;
pub mod regs;
pub mod reservation_station;
pub mod rob;
pub mod util;

pub fn parse_and_exec<C: Cpu>(
    name: &'static str,
    regs: HashMap<ArchReg, u32>,
    mem: Memory,
) -> ExecResult {
    let contents = std::fs::read_to_string(format!("asm/{}.asm", name)).unwrap();
    let prog = contents
        .parse::<Program>()
        .expect("failed to parse assembly");
    C::new(prog, regs, mem).exec_all()
}
