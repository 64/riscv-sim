use aca::{
    cpu::Cpu, emulated::Emulated, inst::ArchReg, mem::Memory, pipelined::Pipelined,
    program::Program, util::Addr,
};
use std::collections::HashMap;

#[generic_tests::define]
mod t {
    use super::*;

    #[test]
    fn test_loop<C: Cpu>() {
        let contents = std::fs::read_to_string("asm/loop.asm").unwrap();
        let prog = contents
            .parse::<Program>()
            .expect("failed to parse asm/loop.asm");

        let initial_regs = HashMap::from([
            (ArchReg::A0, 0),
            (ArchReg::A1, 40),
            (ArchReg::A2, 80),
            (ArchReg::A3, 10),
        ]);

        let mut initial_mem = Memory::new();
        for i in 0..10 {
            initial_mem.writew(Addr(40 + i * 4), i);
            initial_mem.writew(Addr(80 + i * 4), 10 - i);
        }

        let res = C::new(prog, initial_regs, initial_mem).exec_all();

        for i in 0..10 {
            assert_eq!(res.mem.readw(Addr(i * 4)), 10);
        }
    }

    #[test]
    fn test_label<C: Cpu>() {
        let contents = std::fs::read_to_string("asm/label.asm").unwrap();
        let prog = contents
            .parse::<Program>()
            .expect("failed to parse asm/label.asm");

        let res = C::new(prog, HashMap::new(), Memory::new()).exec_all();
        for i in 0..10 {
            assert_eq!(res.mem.readw(Addr(i * 4)), 0);
        }
    }

    #[instantiate_tests(<Emulated>)]
    mod emulated {}

    // #[instantiate_tests(<Pipelined>)]
    // mod pipelined {}
}
