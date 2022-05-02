mod branch;
mod cpu;
mod emulated;
mod execution_unit;
mod inst;
mod lsq;
mod mem;
mod out_of_order;
mod program;
mod queue;
mod regs;
mod reservation_station;
mod rob;
mod util;

use std::path::PathBuf;

use crate::{cpu::Cpu, inst::ArchReg, mem::MainMemory, regs::RegSet, util::Addr};

fn main() {
    let file = std::env::args()
        .nth(1)
        .expect("required input file as argument argument");

    let contents =
        std::fs::read_to_string(&format!("asm/{}.asm", file)).expect("failed to open file");

    let prog = contents
        .parse::<program::Program>()
        .expect("failed to parse program");

    let mut mem = MainMemory::new();

    let a0 = std::env::args().nth(2).unwrap_or_else(|| "".to_string());
    let a0 = if let Ok(x) = a0.parse::<u32>() {
        x
    } else if let Ok(path) = a0.parse::<PathBuf>() {
        println!("Loading file: {}", path.display());

        let load_addr = 1000;
        let data = std::fs::read(path).expect("could not open file");
        mem.copy_from_slice(&data, Addr(load_addr));
        load_addr
    } else {
        0
    };

    let a1 = std::env::args()
        .nth(3)
        .and_then(|x| x.parse::<u32>().ok())
        .unwrap_or(0);
    let initial_regs = RegSet::from([(ArchReg::A0, a0), (ArchReg::A1, a1)]);

    // let res = emulated::Emulated::new(prog, initial_regs, mem).exec_all();
    let res = out_of_order::OutOfOrder::new(prog, initial_regs, mem).exec_all();

    // use std::io::Write;
    // let mut f = std::fs::File::create("/tmp/mem.txt").expect("Unable to create file");
    // writeln!(f, "{:#?}", res.mem).unwrap();

    println!("{res}");
}
