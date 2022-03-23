use aca::{
    cpu::{Cpu, ExecResult},
    emulated::Emulated,
    inst::ArchReg,
    mem::Memory,
    out_of_order::OutOfOrder,
    parse_and_exec,
    pipelined::Pipelined,
    program::Program,
    util::Addr,
};
use std::collections::HashMap;

#[generic_tests::define]
mod t {
    use super::*;

    #[test]
    fn test_loop<C: Cpu>() {
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

        let res = parse_and_exec::<C>("loop", initial_regs, initial_mem);

        for i in 0..10 {
            assert_eq!(res.mem.readw(Addr(i * 4)), 10);
        }
    }

    #[test]
    fn test_label<C: Cpu>() {
        let res = parse_and_exec::<C>("label", HashMap::new(), Memory::new());
        for i in 0..10 {
            assert_eq!(res.mem.readw(Addr(i * 4)), 0);
        }
    }

    #[test]
    fn test_branch<C: Cpu>() {
        let res = parse_and_exec::<C>("branch", HashMap::new(), Memory::new());
        assert_eq!(res.mem.readw(Addr(0)), 4);
        assert_eq!(res.mem.readw(Addr(4)), 3);
        assert_eq!(res.mem.readw(Addr(8)), 2);
    }

    #[test]
    fn test_hazard_raw<C: Cpu>() {
        let res = parse_and_exec::<C>("hazard_raw", HashMap::new(), Memory::new());
        assert_eq!(res.mem.readw(Addr(0)), 3);
        assert_eq!(res.mem.readw(Addr(4)), 1);
        assert_eq!(res.mem.readw(Addr(8)), 1);
    }

    #[test]
    fn test_hazard_war<C: Cpu>() {
        let res = parse_and_exec::<C>("hazard_war", HashMap::new(), Memory::new());
        assert_eq!(res.mem.readw(Addr(0)), 1);
        assert_eq!(res.mem.readw(Addr(4)), 2);
    }

    #[test]
    fn test_hazard_waw<C: Cpu>() {
        let res = parse_and_exec::<C>("hazard_waw", HashMap::new(), Memory::new());
        assert_eq!(res.mem.readw(Addr(0)), 2);
        assert_eq!(res.mem.readw(Addr(4)), 2);
    }

    #[test]
    fn test_prime<C: Cpu>() {
        let run = |x| {
            parse_and_exec::<C>("prime", HashMap::from([(ArchReg::A0, x)]), Memory::new())
                .regs
                .get(ArchReg::A0)
        };

        assert_eq!(run(2), 1);
        assert_eq!(run(3), 1);
        assert_eq!(run(4), 0);
        assert_eq!(run(5), 1);
        assert_eq!(run(10), 0);
        assert_eq!(run(100), 0);
        assert_eq!(run(293), 1);
    }

    #[instantiate_tests(<Emulated>)]
    mod emulated {}

    #[instantiate_tests(<Pipelined>)]
    mod pipelined {}

    #[instantiate_tests(<OutOfOrder>)]
    mod out_of_order {}
}
