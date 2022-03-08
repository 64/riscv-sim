use crate::{
    cpu::{Cpu, CpuState, ExecResult},
    inst::{ArchReg, Inst},
    mem::Memory,
    program::Program,
    regs::RegSet,
};
use std::collections::HashMap;

mod stages {
    use super::*;

    #[derive(Debug, Default)]
    pub struct If {
        pub inst: Option<Inst>,
    }

    #[derive(Debug, Default)]
    pub struct ExMem {
        pub inst: Option<Inst>,
        pub alu: u32,
        pub mem: u32,
    }

    #[derive(Debug, Default)]
    pub struct MemWb {
        pub inst: Option<Inst>,
        pub should_halt: bool,
    }
}

#[derive(Debug, Default)]
struct Pipeline {
    fetch: stages::If,
    ex_mem: stages::ExMem,
    mem_wb: stages::MemWb,
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
        let mut pipe = Pipeline::default();

        loop {
            let fetch = self.stage_fetch(&pipe);
            dbg!(&fetch);
            let ex_mem = self.stage_ex_mem(&pipe);
            dbg!(&ex_mem);
            let mem_wb = self.stage_mem_wb(&pipe);
            dbg!(&mem_wb);

            if mem_wb.should_halt {
                return ExecResult {
                    mem: self.mem,
                    cycles_taken: cycles,
                };
            }

            pipe = Pipeline {
                fetch,
                ex_mem,
                mem_wb,
            };

            cycles += 1;
        }
    }
}

impl Pipelined {
    fn stage_fetch(&mut self, _pipe: &Pipeline) -> stages::If {
        let old_pc = self.pc;
        self.pc += 1;

        stages::If {
            inst: self.prog.fetch(old_pc).cloned(),
        }
    }

    fn stage_ex_mem(&mut self, pipe: &Pipeline) -> stages::ExMem {
        let inst = match &pipe.fetch.inst {
            Some(inst) => inst.clone(),
            None => return stages::ExMem::default(),
        };

        let mut out = stages::ExMem::default();

        match inst {
            Inst::AddImm(_, src, imm) => {
                let a = self.regs.get(src);
                let b = imm.0;
                out.alu = a.wrapping_add(b);
            }
            _ => unimplemented!(),
        }

        out.inst = Some(inst);
        out
    }

    fn stage_mem_wb(&mut self, pipe: &Pipeline) -> stages::MemWb {
        let inst = match &pipe.fetch.inst {
            Some(inst) => inst.clone(),
            None => {
                return stages::MemWb {
                    should_halt: true,
                    ..Default::default()
                }
            }
        };

        let mut out = stages::MemWb::default();

        match inst {
            Inst::AddImm(dst, _, _) => {
                self.regs.set(dst, pipe.ex_mem.alu);
            }
            _ => unimplemented!(),
        }

        out.inst = Some(inst);
        out
    }
}
