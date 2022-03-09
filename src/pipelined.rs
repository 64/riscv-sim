use crate::{
    cpu::{Cpu, ExecResult},
    inst::{ArchReg, Inst},
    mem::Memory,
    program::Program,
    regs::RegSet,
};
use std::collections::HashMap;

mod stages {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Fetch {
        pub inst: Option<Inst>,
        pub new_pc: u32,
    }

    #[derive(Debug, Default)]
    pub struct Decode {
        pub inst: Option<Inst>,
        pub should_stall: bool,
    }

    #[derive(Debug, Default)]
    pub struct ExMem {
        pub inst: Option<Inst>,
        pub alu: u32,
        pub mem: u32,
    }

    #[derive(Debug, Default)]
    pub struct MemWb {
        pub resume_pipe: bool,
        pub should_halt: bool,
        pub retire: bool,
    }
}

#[derive(Debug, Default)]
struct Pipeline {
    fetch: stages::Fetch,
    decode: stages::Decode,
    ex_mem: stages::ExMem,
}

#[derive(Debug, Clone)]
pub struct Pipelined {
    regs: RegSet,
    mem: Memory,
    prog: Program,
    pc: u32,
}

impl Cpu for Pipelined {
    fn new(prog: Program, regs: HashMap<ArchReg, u32>, mem: Memory) -> Self {
        assert!(regs.get(&ArchReg::Zero).is_none());

        Self {
            regs: RegSet::new(regs),
            pc: 0,
            mem,
            prog,
        }
    }

    fn exec_all(mut self) -> ExecResult {
        let mut cycles = 0;
        let mut insts_retired = 0;
        let mut pipe = Pipeline::default();
        let mut stalled = false;

        loop {
            let fetch = self.stage_fetch(&pipe);
            let decode = self.stage_decode(&pipe);
            let ex_mem = self.stage_ex_mem(&pipe);
            let mem_wb = self.stage_mem_wb(&pipe);

            if decode.should_stall {
                stalled = true;
            }

            pipe = Pipeline {
                fetch: stages::Fetch {
                    inst: if stalled { None } else { fetch.inst },
                    new_pc: if mem_wb.resume_pipe {
                        self.pc
                    } else if stalled {
                        pipe.fetch.new_pc
                    } else {
                        fetch.new_pc
                    },
                },
                decode,
                ex_mem,
            };
            dbg!(&pipe, &mem_wb);

            if mem_wb.resume_pipe {
                stalled = false;
            }

            if mem_wb.should_halt {
                return ExecResult {
                    mem: self.mem,
                    cycles_taken: cycles,
                    insts_retired,
                };
            }

            if mem_wb.retire {
                insts_retired += 1;
            }

            cycles += 1;
            // debug_assert!(cycles < 10, "infinite loop detected");
            debug_assert!(cycles < 1_000, "infinite loop detected");
        }
    }
}

impl Pipelined {
    fn stage_fetch(&mut self, pipe: &Pipeline) -> stages::Fetch {
        let pc = pipe.fetch.new_pc;
        let inst = self.prog.fetch(pc).cloned();
        stages::Fetch {
            inst: Some(inst.unwrap_or(Inst::Halt)),
            new_pc: pc + 1,
        }
    }

    fn stage_decode(&mut self, pipe: &Pipeline) -> stages::Decode {
        let inst = match &pipe.fetch.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        let should_stall = match inst {
            Inst::BranchIfNotEqual(_, _, _)
            | Inst::BranchIfEqual(_, _, _)
            | Inst::BranchIfGreaterEqual(_, _, _) => true,
            _ => false,
        };

        stages::Decode {
            inst: Some(inst),
            should_stall,
        }
    }

    fn stage_ex_mem(&mut self, pipe: &Pipeline) -> stages::ExMem {
        let inst = match &pipe.decode.inst {
            Some(inst) => inst.clone(),
            None => return Default::default(),
        };

        let mut out = stages::ExMem::default();

        match inst {
            Inst::StoreByte(src, _) | Inst::StoreHalfWord(src, _) | Inst::StoreWord(src, _) => {
                out.mem = self.regs.get(src)
            }
            Inst::LoadByte(_, src) => {
                out.mem = self.mem.readb(self.regs.ref_to_addr(src));
            }
            Inst::LoadHalfWord(_, src) => {
                out.mem = self.mem.readh(self.regs.ref_to_addr(src));
            }
            Inst::LoadWord(_, src) => {
                out.mem = self.mem.readw(self.regs.ref_to_addr(src));
            }
            Inst::Add(_, src0, src1) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = a.wrapping_add(b);
            }
            Inst::AddImm(_, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                out.alu = a.wrapping_add(b);
            }
            Inst::ShiftLeftLogicalImm(_, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                out.alu = a.wrapping_shl(b);
            }
            Inst::BranchIfEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = (a == b).into();
            }
            Inst::BranchIfNotEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = (a != b).into();
            }
            Inst::BranchIfGreaterEqual(src0, src1, _) => {
                let a = self.regs.get(src0);
                let b = self.regs.get(src1);
                out.alu = (a >= b).into();
            }
            Inst::Halt => (),
            _ => unimplemented!("{:?}", inst),
        }

        out.inst = Some(inst);
        out
    }

    fn stage_mem_wb(&mut self, pipe: &Pipeline) -> stages::MemWb {
        let inst = match &pipe.ex_mem.inst {
            Some(Inst::Halt) => {
                return stages::MemWb {
                    resume_pipe: false,
                    should_halt: true,
                    retire: false,
                }
            }
            Some(inst) => inst.clone(),
            None => {
                return stages::MemWb {
                    resume_pipe: false,
                    should_halt: false,
                    retire: false,
                }
            }
        };

        let mut advance_pc = true;
        let mut resume_pipe = false;

        match inst {
            Inst::StoreByte(_, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeb(dst, pipe.ex_mem.mem);
            }
            Inst::StoreHalfWord(_, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writeh(dst, pipe.ex_mem.mem);
            }
            Inst::StoreWord(_, dst) => {
                let dst = self.regs.ref_to_addr(dst);
                self.mem.writew(dst, pipe.ex_mem.mem);
            }
            Inst::LoadByte(dst, _) | Inst::LoadHalfWord(dst, _) | Inst::LoadWord(dst, _) => {
                self.regs.set(dst, pipe.ex_mem.mem);
            }
            Inst::ShiftLeftLogicalImm(dst, _, _)
            | Inst::Add(dst, _, _)
            | Inst::AddImm(dst, _, _) => {
                self.regs.set(dst, pipe.ex_mem.alu);
            }
            Inst::BranchIfNotEqual(_, _, ref dst)
            | Inst::BranchIfEqual(_, _, ref dst)
            | Inst::BranchIfGreaterEqual(_, _, ref dst) => {
                resume_pipe = true;
                if pipe.ex_mem.alu != 0 {
                    self.pc = self.prog.labels[dst];
                    advance_pc = false;
                }
            }
            _ => unimplemented!("{:?}", inst),
        }

        if advance_pc {
            self.pc += 1;
        }

        stages::MemWb {
            resume_pipe,
            should_halt: false,
            retire: true,
        }
    }
}
