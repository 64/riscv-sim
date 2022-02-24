use aca::{cpu::{Cpu, Memory, Addr}, inst::ArchReg, program::Program};
use std::collections::HashMap;

#[test]
fn test_loop() {
    let contents = std::fs::read_to_string("asm/loop.asm").unwrap();
    let prog = contents
        .parse::<Program>()
        .expect("failed to parse asm/loop.asm");

    let initial_regs = HashMap::from([(ArchReg::R0, 0), (ArchReg::R1, 40), (ArchReg::R2, 80)]);

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
