use aca::{
    cpu::Cpu, emulated::Emulated, inst::ArchReg, mem::MainMemory, out_of_order::OutOfOrder,
    parse_and_exec, pipelined::Pipelined, util::Addr,
};

#[generic_tests::define]
mod t {
    use aca::regs::RegSet;

    use super::*;

    #[test]
    fn test_loop<C: Cpu>() {
        let initial_regs = RegSet::from([
            (ArchReg::A0, 0),
            (ArchReg::A1, 40),
            (ArchReg::A2, 80),
            (ArchReg::A3, 10),
        ]);

        let mut initial_mem = MainMemory::new();
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
        let res = parse_and_exec::<C>("label", RegSet::new(), MainMemory::new());
        for i in 0..10 {
            assert_eq!(res.mem.readw(Addr(i * 4)), 0);
        }
    }

    #[test]
    fn test_branch<C: Cpu>() {
        let res = parse_and_exec::<C>("branch", RegSet::new(), MainMemory::new());
        assert_eq!(res.mem.readw(Addr(0)), 4);
        assert_eq!(res.mem.readw(Addr(4)), 3);
        assert_eq!(res.mem.readw(Addr(8)), 2);
    }

    #[test]
    fn test_hazard_raw<C: Cpu>() {
        let res = parse_and_exec::<C>("hazard_raw", RegSet::new(), MainMemory::new());
        assert_eq!(res.mem.readw(Addr(0)), 3);
        assert_eq!(res.mem.readw(Addr(4)), 1);
        assert_eq!(res.mem.readw(Addr(8)), 1);
    }

    #[test]
    fn test_hazard_war<C: Cpu>() {
        let res = parse_and_exec::<C>("hazard_war", RegSet::new(), MainMemory::new());
        assert_eq!(res.mem.readw(Addr(0)), 1);
        assert_eq!(res.mem.readw(Addr(4)), 2);
    }

    #[test]
    fn test_hazard_waw<C: Cpu>() {
        let res = parse_and_exec::<C>("hazard_waw", RegSet::new(), MainMemory::new());
        assert_eq!(res.mem.readw(Addr(0)), 2);
        assert_eq!(res.mem.readw(Addr(4)), 2);
    }

    #[test]
    fn test_prime<C: Cpu>() {
        let run = |x| {
            parse_and_exec::<C>("prime", RegSet::from([(ArchReg::A0, x)]), MainMemory::new())
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

    #[test]
    fn test_matmul<C: Cpu>() {
        let run = |dim| {
            let mem = parse_and_exec::<C>(
                "matmul",
                RegSet::from([(ArchReg::A0, 0), (ArchReg::A1, dim)]),
                MainMemory::new(),
            )
            .mem;

            for i in 0..dim {
                for j in 0..dim {
                    let c_start = 2 * (4 * dim * dim);
                    let val = if i == j { 1 } else { 0 };
                    assert_eq!(mem.readw(Addr(c_start + 4 * (j * dim + i))), val);
                }
            }
        };

        run(1);
        run(2);
        run(4);
        run(8);
        run(9);
    }

    #[instantiate_tests(<Emulated>)]
    mod emulated {}

    #[instantiate_tests(<Pipelined>)]
    mod pipelined {}

    #[instantiate_tests(<OutOfOrder>)]
    mod out_of_order {}
}
