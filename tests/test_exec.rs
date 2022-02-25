use aca::{
    cpu::{Addr, Cpu, Memory},
    inst::ArchReg,
    program::Program,
};
use std::collections::HashMap;

#[test]
fn test_loop() {
    let contents = std::fs::read_to_string("asm/loop.asm").unwrap();
    let prog = contents
        .parse::<Program>()
        .expect("failed to parse asm/loop.asm");

    let initial_regs = HashMap::from([(ArchReg::A0, 0), (ArchReg::A1, 40), (ArchReg::A2, 80), (ArchReg::A3, 10)]);

    let mut initial_mem = Memory::new();
    for i in 0..10 {
        initial_mem.writew(Addr(40 + i * 4), i);
        initial_mem.writew(Addr(80 + i * 4), 10 - i);
    }

    let cpu = Cpu::new(prog, initial_regs, initial_mem).exec_all();

    for i in 0..10 {
        assert_eq!(cpu.mem.readw(Addr(i * 4)), 10);
    }
}
